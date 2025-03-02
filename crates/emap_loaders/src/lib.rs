use std::{
    cell::OnceCell,
    sync::{Arc, Mutex, RwLock},
};

use dir_tile_cache::DiskDirTileCache;
use egui::Context;
use ehttp::{Request, Response};
use emap::{tile_drawable::CommonEguiTileDrawable, EguiMapTileRes};
use rustc_hash::{FxHashMap, FxHashSet};
use rustitude_base::{curr_time_millis, map_view_state::MapViewState, qtree::QTreeKey};

pub mod dir_tile_cache;
#[cfg(feature = "mvt")]
pub mod mvt;
#[cfg(feature = "png")]
pub mod png;

pub trait RequestBuilder: Send + Sync {
    fn build_req(&self, typ: &str, x: u32, y: u32, z: u8) -> Request;

    fn decode_response(&self, resp: Response) -> Arc<[u8]> {
        resp.bytes.into()
    }
}

pub trait TileLoader: Send + Sync {
    fn mem_cache(self: &Self) -> &MemoryDrawableCache;
    fn load_img(self: &Self, key: QTreeKey, ctx: Context, vec: Arc<[u8]>) -> bool;
}

pub trait BinTileCache: Send + Sync {
    fn save(&self, key: QTreeKey, value: Arc<[u8]>);
    fn load(&self, key: QTreeKey) -> Option<Arc<[u8]>>;
    fn exist(&self, key: QTreeKey) -> bool;
    fn delete(&self, key: QTreeKey);
}

pub struct MemoryDrawableCache {
    data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiTileDrawable>>>,
    /// 记录每个key的最后一次访问时间，用于清理过期的内存缓存
    hot_map: Arc<Mutex<FxHashMap<QTreeKey, u128>>>,
}

impl MemoryDrawableCache {
    pub fn new() -> Self {
        Self {
            data_map: Arc::new(RwLock::new(FxHashMap::default())),
            hot_map: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }

    fn get(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable> {
        if let Some(img) = self.data_map.read().unwrap().get(&key) {
            if let Ok(mut hm) = self.hot_map.try_lock() {
                hm.insert(key, curr_time_millis());
            }
            return Some(img.clone());
        }
        return None;
    }

    fn put(&self, key: QTreeKey, value: CommonEguiTileDrawable) -> Vec<QTreeKey> {
        {
            let mut m = self.data_map.write().unwrap();
            m.insert(key, value);
        }
        let time = curr_time_millis();
        let mut hot_map = self.hot_map.lock().unwrap();
        hot_map.insert(key, time);
        let mut will_del: Vec<QTreeKey> = vec![];
        if hot_map.len() > 500 {
            hot_map.iter().for_each(|t| {
                if t.0.depth() > 3 && time.checked_sub(t.1.clone()).unwrap_or(0) > 20_000 {
                    will_del.push(t.0.clone());
                }
            });
            if will_del.len() > 0 {
                let mut m = self.data_map.write().unwrap();
                will_del.iter().for_each(|k| {
                    m.remove(k);
                    hot_map.remove(k);
                });
            }
        }
        return will_del;
    }

    fn remove(&self, key: QTreeKey) {
        let mut m = self.data_map.write().unwrap();
        m.remove(&key);

        let mut hot_map = self.hot_map.lock().unwrap();
        hot_map.remove(&key);
    }
}

struct _EguiMapBinResImpl {
    /// 类型，用于生成全局图片缓存的key等场景
    typ: String,
    /// 非空时会优先从缓存中加载，从网络加载时也会缓存到路径中，为空时只会从网络加载。
    cache: Option<Arc<dyn BinTileCache>>,
    request_builder: Box<dyn RequestBuilder>,
    rt: Arc<tokio::runtime::Runtime>,
    /// 记录正在加载中的key
    loading_lock: RwLock<FxHashSet<u64>>,
    loader: Box<dyn TileLoader>,
}

#[derive(Clone)]
pub struct EguiMapBinResImpl {
    inner: Arc<_EguiMapBinResImpl>,
}

impl EguiMapBinResImpl {
    const TOKIO_RT: OnceCell<Arc<tokio::runtime::Runtime>> = OnceCell::new();

    pub fn new(
        typ: &str,
        file_ext: impl Into<String>,
        cache_path_prefix: Option<&str>,
        request_builder: Box<dyn RequestBuilder>,
        loader: Box<dyn TileLoader>,
    ) -> Self {
        let rt = EguiMapBinResImpl::TOKIO_RT
            .get_or_init(|| {
                Arc::new(
                    tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(8) // 8个工作线程
                        // .enable_io() // 可在runtime中使用异步IO
                        // .enable_time() // 可在runtime中使用异步计时器(timer)
                        .build() // 创建runtime
                        .unwrap(),
                )
            })
            .clone();
        Self {
            inner: Arc::new(_EguiMapBinResImpl {
                cache: cache_path_prefix
                    .map(|s| format!("{}/{}", s, typ))
                    .map(|s| {
                        Arc::new(DiskDirTileCache {
                            cache_path_prefix: s,
                            file_ext: file_ext.into(),
                        }) as Arc<dyn BinTileCache>
                    }),
                request_builder: request_builder,

                rt: rt,
                loading_lock: RwLock::new(FxHashSet::default()),
                loader: loader,
                typ: String::from(typ),
            }),
        }
    }
}

impl EguiMapTileRes for EguiMapBinResImpl {
    fn get_memory_cache(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable> {
        return self.inner.loader.mem_cache().get(key);
    }

    fn get_or_fetch(
        &self,
        key: QTreeKey,
        mvs: Arc<RwLock<MapViewState>>,
        ctx: &Context,
    ) -> Option<CommonEguiTileDrawable> {
        let weak = self.get_memory_cache(key);
        if weak.is_some() {
            return weak;
        }
        let z = key.depth();
        let x = key.x();
        let y = key.y();
        let c = ctx.clone();
        let s = self.clone();
        let mut loading_locks = self.inner.loading_lock.write().unwrap();
        let is_loading = loading_locks.contains(&key.inner_key());
        if let Some(cache) = self.inner.cache.clone() {
            if is_loading {
                return None;
            } else if cache.exist(key) {
                //存在缓存
                loading_locks.insert(key.inner_key());
                self.inner.rt.spawn(async move {
                    let lt;
                    let rb;
                    {
                        let mvs = mvs.read().unwrap();
                        lt = mvs.top_left_key();
                        rb = mvs.bottom_right_key();
                    }
                    if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                        if let Some(vec) = cache.load(key) {
                            if !s.inner.loader.load_img(key, c.clone(), vec) {
                                cache.delete(key);
                            }
                        }
                        c.request_repaint();
                    }
                    s.inner
                        .loading_lock
                        .write()
                        .unwrap()
                        .remove(&key.inner_key());
                });
                return None;
            } else {
                loading_locks.insert(key.inner_key());
                self.inner.rt.spawn(async move {
                    let lt;
                    let rb;
                    {
                        let mvs = mvs.read().unwrap();
                        lt = mvs.top_left_key();
                        rb = mvs.bottom_right_key();
                    }
                    if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                        println!("fetch:{}_{}_{}", z, x, y);
                        let req = s
                            .inner
                            .request_builder
                            .build_req(s.inner.typ.as_str(), x, y, z);
                        let resp = ehttp::fetch_blocking(&req);
                        if let Ok(r) = resp {
                            if r.status == 200 {
                                let bytes: Arc<[u8]> = s.inner.request_builder.decode_response(r);
                                if !s.inner.loader.load_img(key, c.clone(), bytes.clone()) {
                                    cache.delete(key);
                                } else {
                                    cache.save(key, bytes);
                                    c.request_repaint();
                                }
                            } else {
                                println!("resp status:{}", r.status);
                            }
                        } else if let Err(r) = resp {
                            println!("resp error:{}", r);
                        }
                    }
                    s.inner
                        .loading_lock
                        .write()
                        .unwrap()
                        .remove(&key.inner_key());
                });
                return None;
            }
        } else {
            if is_loading {
                return None;
            } else {
                loading_locks.insert(key.inner_key());
                self.inner.rt.spawn(async move {
                    let lt;
                    let rb;
                    {
                        let mvs = mvs.read().unwrap();
                        lt = mvs.top_left_key();
                        rb = mvs.bottom_right_key();
                    }
                    if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                        println!("fetch:{}_{}_{}", z, x, y);
                        let req = s
                            .inner
                            .request_builder
                            .build_req(s.inner.typ.as_str(), x, y, z);
                        let resp = ehttp::fetch_blocking(&req);
                        if let Ok(r) = resp {
                            if r.status == 200 {
                                s.inner.loader.load_img(key, c.clone(), r.bytes.into());
                                c.request_repaint();
                            } else {
                                println!("resp status:{}", r.status);
                            }
                        } else if let Err(r) = resp {
                            println!("resp error:{}", r);
                        }
                    }
                    s.inner
                        .loading_lock
                        .write()
                        .unwrap()
                        .remove(&key.inner_key());
                });
                return None;
            }
        }
    }
}
