use std::{
    cell::OnceCell,
    fs::{self},
    sync::{Arc, Mutex, RwLock},
};

use egui::{
    load::{BytesLoader, TexturePoll},
    Context, Image,
};
use emap::{
    tile_drawable::{CommonEguiTileDrawable, TILE_SIZE_VEC2},
    EguiMapTileRes,
};
use rustc_hash::{FxHashMap, FxHashSet};
use rustitude_base::{curr_time_millis, map_view_state::MapViewState, qtree::QTreeKey};

use crate::{BinTileCache, RequestBuilder};

type HotMap = Arc<Mutex<FxHashMap<QTreeKey, u128>>>;

#[derive(Clone)]
pub struct EguiMapPngResImpl {
    /// 类型，用于生成全局图片缓存的key等场景
    typ: String,
    /// 磁盘缓存路径前缀，非空时会优先从对应路径的磁盘缓存中加载，从网络加载时也会缓存到路径中，为空时只会从网络加载。
    /// 本地路径格式为：{cache_path_prefix}/{typ}/{z}_{x}_{y}.png
    cache: Option<Arc<dyn BinTileCache>>,
    request_builder: Arc<dyn RequestBuilder>,
    data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiTileDrawable>>>,
    rt: Arc<tokio::runtime::Runtime>,
    /// 记录正在加载中的key
    loading_lock: Arc<RwLock<FxHashSet<u64>>>,
    /// 记录每个key的最后一次访问时间，用于清理过期的内存缓存
    hot_map: HotMap,
}

impl EguiMapPngResImpl {
    const TOKIO_RT: OnceCell<Arc<tokio::runtime::Runtime>> = OnceCell::new();

    pub fn new(
        typ: &str,
        cache_path_prefix: Option<&str>,
        request_builder: Arc<dyn RequestBuilder>,
    ) -> Self {
        let rt = EguiMapPngResImpl::TOKIO_RT
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
        EguiMapPngResImpl {
            typ: String::from(typ),
            cache: cache_path_prefix.map(|s| format!("{}/{}",s,typ)).map(|s| Arc::new(DiskDirTileCache{
                cache_path_prefix: s,
                file_ext: String::from("png")
            }) as Arc<dyn BinTileCache>),
            request_builder: request_builder,
            data_map: Arc::new(RwLock::new(FxHashMap::default())),
            rt: rt,
            loading_lock: Arc::new(RwLock::new(FxHashSet::default())),
            hot_map: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }
}

impl EguiMapTileRes for EguiMapPngResImpl {
    fn get_memory_cache(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable> {
        if let Some(img) = self.data_map.read().unwrap().get(&key) {
            if let Ok(mut hm) = self.hot_map.try_lock() {
                hm.insert(key, curr_time_millis());
            }
            return Some(img.clone());
        }
        return None;
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
        let mut loading_locks = self.loading_lock.write().unwrap();
        let send_lock = self.loading_lock.clone();
        let c = ctx.clone();
        let s = self.clone();
        let is_loading = loading_locks.contains(&key.inner_key());
        let rq = self.request_builder.clone();
        if let Some(cache) = self.cache.clone() {
            if is_loading {
                return None;
            } else if cache.exist(key) { //存在缓存
                loading_locks.insert(key.inner_key());
                self.rt.spawn(async move {
                    let lt;
                    let rb;
                    {
                        let mvs = mvs.read().unwrap();
                        lt = mvs.top_left_key();
                        rb = mvs.bottom_right_key();
                    }
                    if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                        if let Some(vec) = cache.load(key) {
                            if !s.load_img(key, c.clone(), vec) {
                                cache.delete(key);
                            }
                        }
                        c.request_repaint();
                    }
                    send_lock.write().unwrap().remove(&key.inner_key());
                });
                return None;
            } else {
                loading_locks.insert(key.inner_key());
                let typ = self.typ.clone();
                self.rt.spawn(async move {
                    let lt;
                    let rb;
                    {
                        let mvs = mvs.read().unwrap();
                        lt = mvs.top_left_key();
                        rb = mvs.bottom_right_key();
                    }
                    if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                        println!("fetch:{}_{}_{}", z, x, y);
                        let req = rq.build_req(typ.as_str(), x, y, z);
                        let resp = ehttp::fetch_blocking(&req);
                        if let Ok(r) = resp {
                            if r.status == 200 {
                                let bytes: Arc<[u8]> = rq.decode_response(r);
                                if !s.load_img(key, c.clone(), bytes.clone()) {
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
                    send_lock.write().unwrap().remove(&key.inner_key());
                });
                return None;
            }
        } else {
            if is_loading {
                return None;
            } else {
                loading_locks.insert(key.inner_key());
                let typ = self.typ.clone();
                self.rt.spawn(async move {
                    let lt;
                    let rb;
                    {
                        let mvs = mvs.read().unwrap();
                        lt = mvs.top_left_key();
                        rb = mvs.bottom_right_key();
                    }
                    if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                        println!("fetch:{}_{}_{}", z, x, y);
                        let req = rq.build_req(typ.as_str(), x, y, z);
                        let resp = ehttp::fetch_blocking(&req);
                        if let Ok(r) = resp {
                            if r.status == 200 {
                                s.load_img(key, c.clone(), r.bytes.into());
                                c.request_repaint();
                            } else {
                                println!("resp status:{}", r.status);
                            }
                        } else if let Err(r) = resp {
                            println!("resp error:{}", r);
                        }
                    }
                    send_lock.write().unwrap().remove(&key.inner_key());
                });
                return None;
            }
        }
    }
}

impl EguiMapPngResImpl {
    fn load_img(self: &Self, key: QTreeKey, ctx: Context, vec: Arc<[u8]>) -> bool {
        let dm = self.data_map.clone();
        let hm = self.hot_map.clone();
        let uri = self.uri_of(key);
        let img = Image::from_bytes(uri, vec);
        match img.load_for_size(&ctx, TILE_SIZE_VEC2) {
            Ok(r) => {
                if let TexturePoll::Ready { texture } = r {
                    {
                        let mut m = dm.write().unwrap();
                        m.insert(key, Arc::new(texture));
                    }
                    let time = curr_time_millis();
                    let mut hot_map = hm.lock().unwrap();
                    hot_map.insert(key, time);
                    let mut will_del: Vec<QTreeKey> = vec![];
                    if hot_map.len() > 500 {
                        hot_map.iter().for_each(|t| {
                            if t.0.depth() > 3
                                && time.checked_sub(t.1.clone()).unwrap_or(0) > 20_000
                            {
                                will_del.push(t.0.clone());
                            }
                        });
                        if will_del.len() > 0 {
                            self.remove_img_cache(&ctx, dm, &mut hot_map, &mut will_del);
                            println!("del:{}", will_del.len());
                        }
                    }
                }
                true
            }
            Err(e) => {
                let mut hot_map = hm.lock().unwrap();
                let mut will_del: Vec<QTreeKey> = vec![key];
                self.remove_img_cache(&ctx, dm, &mut hot_map, &mut will_del);
                println!("load_img error:{}", e);
                match e {
                    egui::load::LoadError::Loading(_) => true,
                    _ => false,
                }
            }
        }
    }

    fn remove_img_cache(
        self: &Self,
        ctx: &Context,
        data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiTileDrawable>>>,
        hot_map: &mut FxHashMap<QTreeKey, u128>,
        will_del: &mut Vec<QTreeKey>,
    ) {
        {
            let mut m = data_map.write().unwrap();
            will_del.iter().for_each(|k| {
                m.remove(k);
            });
        }
        will_del.iter().for_each(|k| {
            hot_map.remove(k);
            let uri = self.uri_of(k.clone());
            ctx.loaders().include.forget(&uri);
            ctx.loaders()
                .bytes
                .lock()
                .iter()
                .for_each(|l| l.forget(&uri));
            ctx.loaders()
                .image
                .lock()
                .iter()
                .for_each(|l| l.forget(&uri));
            ctx.loaders()
                .texture
                .lock()
                .iter()
                .for_each(|l| l.forget(&uri));
        });
    }

    fn uri_of(&self, key: QTreeKey) -> String {
        format!(
            "tiles/{}/{}_{}_{}.png",
            self.typ.as_str(),
            key.depth(),
            key.x(),
            key.y()
        )
    }
}

pub struct DiskDirTileCache{
    cache_path_prefix:String,
    file_ext:String
}

impl DiskDirTileCache {
    fn path_of(&self,key:QTreeKey) -> String {
        format!(
            "{}/{}_{}_{}.{}",
            self.cache_path_prefix.as_str(),
            key.depth(),
            key.x(),
            key.y(),
            self.file_ext.as_str()
        )
    }
}

impl BinTileCache for DiskDirTileCache {
    fn save(&self,key:QTreeKey,value: Arc<[u8]>) {
        if !fs::exists(self.cache_path_prefix.as_str()).unwrap_or(false) {
            let _ = fs::create_dir_all(self.cache_path_prefix.as_str());
        }
        let cache_file_path = self.path_of(key);
        let lock_file_path = format!("{}.tmp",cache_file_path.as_str());
        if fs::exists(lock_file_path.as_str()).unwrap_or(false) {
            return;
        }
        fs::write(lock_file_path.clone(), value).unwrap();
        let _ = fs::rename(lock_file_path, cache_file_path.as_str());
    }

    fn load(&self,key:QTreeKey) -> Option<Arc<[u8]>> {
        let cache_file_path = self.path_of(key);
        return fs::read(cache_file_path.as_str()).map(|v|v.into()).ok();
    }

    fn exist(&self,key:QTreeKey) -> bool {
        let cache_file_path = self.path_of(key);
        fs::exists(cache_file_path.as_str()).unwrap_or(false)
    }

    fn delete(&self,key:QTreeKey) {
        let cache_file_path = self.path_of(key);
        let _ = fs::remove_file(cache_file_path.as_str());
    }
}
