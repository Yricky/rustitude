use std::{
    fs,
    sync::{Arc, RwLock},
};

use egui::{
    load::{SizedTexture, TexturePoll},
    vec2, Context, Image, Vec2,
};
use rustc_hash::{FxHashMap, FxHashSet};
use rustitude_base::{map_view_state::MapViewState, qtree::QTreeKey};

use crate::view::priv_fn::build_req;

pub struct EguiMapImgRes {
    pub data_map: Arc<RwLock<FxHashMap<QTreeKey, SizedTexture>>>,
    pub rt: Arc<tokio::runtime::Runtime>,
    pub lock: Arc<RwLock<FxHashSet<u64>>>,
}

pub const TILE_SIZE_VEC2: Vec2 = vec2(256.0, 256.0);

impl EguiMapImgRes {
    pub fn get(
        &self,
        key: QTreeKey,
        mvs: Arc<RwLock<MapViewState>>,
        ctx: &Context,
    ) -> Option<SizedTexture> {
        if let Some(img) = self.data_map.read().unwrap().get(&key) {
            return Some(img.clone());
        }
        let z = key.depth();
        let x = key.x();
        let y = key.y();
        let tile_file_path = format!("tiles/{}_{}_{}.png", z, x, y);
        let mut lock = self.lock.write().unwrap();
        let send_lock = self.lock.clone();
        let c = ctx.clone();
        if fs::exists(tile_file_path.clone()).unwrap() {
            let tfp = tile_file_path.clone();
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
                    if !fs::exists(lock_file_path.clone()).unwrap() {
                        println!("fetch:{}_{}_{}", z, x, y);
                        let req = build_req(x, y, z);
                        let resp = ehttp::fetch_blocking(&req);
                        if let Ok(r) = resp {
                            if r.status == 200 {
                                if !fs::exists("tiles").unwrap() {
                                    let _ = fs::create_dir("tiles");
                                }
                                fs::write(lock_file_path.clone(), r.bytes).unwrap();
                                let _ = fs::rename(lock_file_path, tile_file_path.clone());
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
    dm: Arc<RwLock<FxHashMap<QTreeKey, SizedTexture>>>,
) {
    let vec = fs::read(tfp.clone()).unwrap();
    let uri = format!("bytes://{}", tfp);
    let img = Image::from_bytes(uri, vec);
    if let Ok(r) = img.load_for_size(&ctx, TILE_SIZE_VEC2) {
        if let TexturePoll::Ready { texture } = r {
            let mut m = dm.write().unwrap();
            m.insert(key, texture);
        }
    }
}
