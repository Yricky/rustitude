use std::{
    fs,
    sync::{Arc, RwLock},
};

use egui::{
    load::{SizedTexture, TexturePoll},
    pos2, vec2, Color32, Context, Image, Painter, Rect, Vec2,
};
use rustc_hash::{FxHashMap, FxHashSet};
use rustitude_base::{map_view_state::MapViewState, qtree::QTreeKey};

use crate::view::priv_fn::build_req;

pub trait EguiDrawable: Send + Sync {
    fn draw(&self, painter: &Painter, rect: Rect);
}

type CommonEguiDrawable = Arc<dyn EguiDrawable>;

pub trait EguiMapImgRes {
    fn get(
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
}

pub struct EguiMapImgResImpl {
    pub data_map: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiDrawable>>>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub loading_lock: Arc<RwLock<FxHashSet<u64>>>,
}

pub const TILE_SIZE_VEC2: Vec2 = vec2(256.0, 256.0);

impl EguiMapImgRes for EguiMapImgResImpl {
    fn get(
        &self,
        key: QTreeKey,
        mvs: Arc<RwLock<MapViewState>>,
        ctx: &Context,
    ) -> Option<CommonEguiDrawable> {
        if let Some(img) = self.data_map.read().unwrap().get(&key) {
            return Some(img.clone());
        }
        let z = key.depth();
        let x = key.x();
        let y = key.y();
        let tile_file_path = format!("tiles/{}_{}_{}.png", z, x, y);
        let mut lock = self.loading_lock.write().unwrap();
        let send_lock = self.loading_lock.clone();
        let c = ctx.clone();
        if fs::exists(tile_file_path.as_str()).unwrap() {
            let tfp = tile_file_path;
            let dm = self.data_map.clone();
            let c = ctx.clone();
            self.rt.spawn(async move {
                load_img(key, c.clone(), tfp, dm);
                c.request_repaint();
            });
            return None;
        } else if !lock.contains(&key.inner_key()) {
            lock.insert(key.inner_key());
            let dm = self.data_map.clone();
            self.rt.spawn(async move {
                let lt;
                let rb;
                {
                    let mvs = mvs.read().unwrap();
                    lt = mvs.top_left_key();
                    rb = mvs.bottom_right_key();
                }
                if z == lt.depth() && lt.x() <= x && x <= rb.x() && lt.y() <= y && y <= rb.y() {
                    let lock_file_path = format!("tiles/{}_{}_{}.png.tmp", z, x, y);
                    println!("fetch:{}_{}_{}", z, x, y);
                    let req = build_req(x, y, z);
                    let resp = ehttp::fetch_blocking(&req);
                    if let Ok(r) = resp {
                        if r.status == 200 {
                            if !fs::exists("tiles").unwrap() {
                                let _ = fs::create_dir("tiles");
                            }
                            fs::write(lock_file_path.clone(), r.bytes).unwrap();
                            let _ = fs::rename(lock_file_path, tile_file_path.as_str());
                            load_img(key, c.clone(), tile_file_path, dm);
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

fn load_img(
    key: QTreeKey,
    ctx: Context,
    tfp: String,
    dm: Arc<RwLock<FxHashMap<QTreeKey, CommonEguiDrawable>>>,
) {
    let vec = fs::read(tfp.clone()).unwrap();
    let uri = format!("bytes://{}", tfp);
    let img = Image::from_bytes(uri, vec);
    if let Ok(r) = img.load_for_size(&ctx, TILE_SIZE_VEC2) {
        if let TexturePoll::Ready { texture } = r {
            let mut m = dm.write().unwrap();
            m.insert(key, Arc::new(texture));
        }
    }
}
