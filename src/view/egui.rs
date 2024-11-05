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
    EguiMapImgRes,
};
use rustc_hash::{FxHashMap, FxHashSet};
use rustitude_base::{curr_time_millis, map_view_state::MapViewState, qtree::QTreeKey};

use crate::view::priv_fn::build_req;

type HotMap = Arc<Mutex<FxHashMap<QTreeKey, u128>>>;

#[derive(Clone)]
pub struct EguiMapImgResImpl {
    pub typ: String,
    pub data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiTileDrawable>>>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub loading_lock: Arc<RwLock<FxHashSet<u64>>>,
    hot_map: HotMap,
}

impl EguiMapImgResImpl {
    const TOKIO_RT: OnceCell<Arc<tokio::runtime::Runtime>> = OnceCell::new();

    pub fn new(typ: &str) -> Self {
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
            data_map: Arc::new(RwLock::new(FxHashMap::default())),
            rt: rt,
            loading_lock: Arc::new(RwLock::new(FxHashSet::default())),
            hot_map: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }
}

impl EguiMapImgRes for EguiMapImgResImpl {
    fn get(&self, key: QTreeKey) -> Option<CommonEguiTileDrawable> {
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
        let weak = self.get(key);
        if weak.is_some() {
            return weak;
        }
        let z = key.depth();
        let x = key.x();
        let y = key.y();
        let tile_file_path = format!("tiles/{}/{}_{}_{}.png", self.typ.as_str(), z, x, y);
        let mut lock = self.loading_lock.write().unwrap();
        let send_lock = self.loading_lock.clone();
        let c = ctx.clone();
        if fs::exists(tile_file_path.as_str()).unwrap() {
            let tfp = tile_file_path;
            let c = ctx.clone();
            let s = self.clone();
            self.rt.spawn(async move {
                let lt;
                let rb;
                {
                    let mvs = mvs.read().unwrap();
                    lt = mvs.top_left_key();
                    rb = mvs.bottom_right_key();
                }
                if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                    let vec = fs::read(tfp.as_str()).unwrap();
                    s.load_img(key, c.clone(), vec);
                    c.request_repaint();
                }
            });
            return None;
        } else if !lock.contains(&key.inner_key()) {
            lock.insert(key.inner_key());
            let typ = self.typ.clone();
            let s = self.clone();
            self.rt.spawn(async move {
                let lt;
                let rb;
                {
                    let mvs = mvs.read().unwrap();
                    lt = mvs.top_left_key();
                    rb = mvs.bottom_right_key();
                }
                if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                    let lock_file_path =
                        format!("tiles/{}/{}_{}_{}.png.tmp", typ.as_str(), z, x, y);
                    println!("fetch:{}_{}_{}", z, x, y);
                    let req = build_req(typ.as_str(), x, y, z);
                    let resp = ehttp::fetch_blocking(&req);
                    if let Ok(r) = resp {
                        if r.status == 200 {
                            let path = format!("tiles/{}", typ.as_str());
                            if !fs::exists(path.as_str()).unwrap() {
                                let _ = fs::create_dir_all(path.as_str());
                            }
                            fs::write(lock_file_path.clone(), r.bytes).unwrap();
                            let _ = fs::rename(lock_file_path, tile_file_path.as_str());
                            let vec = fs::read(tile_file_path.as_str()).unwrap();
                            s.load_img(key, c.clone(), vec);
                            c.request_repaint();
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
        } else {
            return None;
        }
    }
}

impl EguiMapImgResImpl {
    fn load_img(self: &Self, key: QTreeKey, ctx: Context, vec: Vec<u8>) {
        let dm = self.data_map.clone();
        let hm = self.hot_map.clone();
        let uri = self.uri_of(key);
        let img = Image::from_bytes(uri, vec);
        if let Ok(r) = img.load_for_size(&ctx, TILE_SIZE_VEC2) {
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
                        if t.0.depth() > 3 && time.checked_sub(t.1.clone()).unwrap_or(0) > 20_000 {
                            will_del.push(t.0.clone());
                        }
                    });
                    if will_del.len() > 0 {
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
                        println!("del:{}", will_del.len());
                    }
                }
            }
        }
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
