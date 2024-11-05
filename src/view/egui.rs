use std::{
    cell::OnceCell,
    fs,
    sync::{Arc, Mutex, MutexGuard, RwLock},
};

use egui::{
    load::{BytesLoader, SizedTexture, TexturePoll},
    pos2, vec2, Color32, Context, Image, Painter, Rect, Vec2,
};
use rustc_hash::{FxHashMap, FxHashSet};
use rustitude_base::{curr_time_millis, map_view_state::MapViewState, qtree::QTreeKey};

use crate::view::priv_fn::build_req;

pub const TILE_SIZE_VEC2: Vec2 = vec2(256.0, 256.0);

pub trait EguiDrawable: Send + Sync {
    fn draw(&self, painter: &Painter, rect: Rect);
    fn clip(&self, rect: Rect) -> CommonEguiDrawable;
}

type CommonEguiDrawable = Arc<dyn EguiDrawable>;
type HotMap = Arc<Mutex<FxHashMap<QTreeKey, u128>>>;

trait CleanLock<T> {
    fn clean_lock(&self) -> MutexGuard<'_, T>;
}

impl<T> CleanLock<T> for Mutex<T> {
    fn clean_lock(&self) -> MutexGuard<'_, T> {
        if self.is_poisoned() {
            self.clear_poison();
        }
        self.lock().unwrap()
    }
}

pub trait EguiMapImgRes {
    fn get(&self, key: QTreeKey) -> Option<CommonEguiDrawable>;

    fn get_or_update(
        &self,
        key: QTreeKey,
        mvs: Arc<RwLock<MapViewState>>,
        ctx: &Context,
    ) -> Option<CommonEguiDrawable>;
}

impl EguiDrawable for SizedTexture {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.image(
            self.id,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn clip(&self, rect: Rect) -> CommonEguiDrawable {
        let d = (self.clone(), rect);
        Arc::new(d)
    }
}

impl EguiDrawable for (SizedTexture, Rect) {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.image(self.0.id, rect, self.1, Color32::WHITE);
    }

    fn clip(&self, rect: Rect) -> CommonEguiDrawable {
        let self_size = self.1.size();
        let rect_size = rect.size();
        Arc::new((
            self.0,
            Rect::from_min_size(
                pos2(
                    self.1.min.x + self_size.x * rect.min.x,
                    self.1.min.y + self_size.y * rect.min.y,
                ),
                vec2(self_size.x * rect_size.x, self_size.y * rect_size.y),
            ),
        ))
    }
}

#[derive(Clone)]
pub struct EguiMapImgResImpl {
    pub typ: String,
    pub data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiDrawable>>>,
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
    fn get(&self, key: QTreeKey) -> Option<CommonEguiDrawable> {
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
    ) -> Option<CommonEguiDrawable> {
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
                    s.load_img(key, c.clone(), tfp);
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
                    // println!("fetch:{}_{}_{}", z, x, y);
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
                            s.load_img(key, c.clone(), tile_file_path);
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
    fn load_img(self: &Self, key: QTreeKey, ctx: Context, tfp: String) {
        let dm = self.data_map.clone();
        let hm = self.hot_map.clone();
        let vec = fs::read(tfp.clone()).unwrap();
        let uri = self.uri_of(key);
        let img = Image::from_bytes(uri, vec);
        if let Ok(r) = img.load_for_size(&ctx, TILE_SIZE_VEC2) {
            if let TexturePoll::Ready { texture } = r {
                {
                    let mut m = dm.write().unwrap();
                    m.insert(key, Arc::new(texture));
                }
                let time = curr_time_millis();
                let mut hot_map = hm.clean_lock();
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
