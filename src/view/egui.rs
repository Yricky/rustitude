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

use crate::view::priv_fn::build_req;

type HotMap = Arc<Mutex<FxHashMap<QTreeKey, u128>>>;

#[derive(Clone)]
pub struct EguiMapImgResImpl {
    pub typ: String,
    cache_path_prefix: Option<String>,
    pub data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiTileDrawable>>>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub loading_lock: Arc<RwLock<FxHashSet<u64>>>,
    hot_map: HotMap,
}

impl EguiMapImgResImpl {
    const TOKIO_RT: OnceCell<Arc<tokio::runtime::Runtime>> = OnceCell::new();

    pub fn new(typ: &str, cache_path_prefix: Option<&str>) -> Self {
        let rt = EguiMapImgResImpl::TOKIO_RT
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
        EguiMapImgResImpl {
            typ: String::from(typ),
            cache_path_prefix: cache_path_prefix.map(|s| String::from(s)),
            data_map: Arc::new(RwLock::new(FxHashMap::default())),
            rt: rt,
            loading_lock: Arc::new(RwLock::new(FxHashSet::default())),
            hot_map: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }
}

impl EguiMapTileRes for EguiMapImgResImpl {
    fn get_memory_cache(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable> {
        if let Some(img) = self.data_map.read().unwrap().get(&key) {
            if let Ok(mut hm) = self.hot_map.try_lock() {
                hm.insert(key, curr_time_millis());
            }
            return Some(img.clone());
        }
        return None;
    }

    fn get_or_update(
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
        if let Some(cpp) = self.cache_path_prefix.clone() {
            let tfp = format!(
                "{}/{}/{}_{}_{}.png",
                cpp.as_str(),
                self.typ.as_str(),
                z,
                x,
                y
            );
            if is_loading {
                return None;
            } else if fs::exists(tfp.as_str()).unwrap_or(false) {
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
                        let vec = fs::read(tfp.as_str()).unwrap_or(vec![]);
                        if !s.load_img(key, c.clone(), vec.into()) {
                            let _ = fs::remove_file(tfp.as_str());
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
                        let lock_file_path = format!("{}.tmp", tfp.as_str());
                        println!("fetch:{}_{}_{}", z, x, y);
                        let req = build_req(typ.as_str(), x, y, z);
                        let resp = ehttp::fetch_blocking(&req);
                        if let Ok(r) = resp {
                            if r.status == 200 {
                                let path = format!("{}/{}", cpp.as_str(), typ.as_str());
                                if !fs::exists(path.as_str()).unwrap_or(false) {
                                    let _ = fs::create_dir_all(path.as_str());
                                }
                                let bytes: Arc<[u8]> = r.bytes.into();
                                if !s.load_img(key, c.clone(), bytes.clone()) {
                                    let _ = fs::remove_file(tfp.as_str());
                                } else {
                                    c.request_repaint();
                                    fs::write(lock_file_path.clone(), bytes).unwrap();
                                    let _ = fs::rename(lock_file_path, tfp.as_str());
                                }
                            } else {
                                println!("resp status:{}", r.status);
                                let _ = fs::remove_file(lock_file_path);
                            }
                        } else if let Err(r) = resp {
                            println!("resp error:{}", r);
                            let _ = fs::remove_file(lock_file_path);
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
                        let req = build_req(typ.as_str(), x, y, z);
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

impl EguiMapImgResImpl {
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
        dm: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiTileDrawable>>>,
        hot_map: &mut FxHashMap<QTreeKey, u128>,
        will_del: &mut Vec<QTreeKey>,
    ) {
        {
            let mut m = dm.write().unwrap();
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
