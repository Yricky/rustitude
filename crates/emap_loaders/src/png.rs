use std::sync::Arc;

use egui::{
    load::{BytesLoader, TexturePoll},
    Context, Image,
};
use emap::tile_drawable::TILE_SIZE_VEC2;
use rustitude_base::qtree::QTreeKey;

use crate::{MemoryDrawableCache, TileLoader};

pub struct PngLoader {
    pub typ: String,
    pub mem_cache: MemoryDrawableCache,
}

impl PngLoader {
    fn uri_of(&self, key: QTreeKey) -> String {
        format!(
            "tiles/{}/{}_{}_{}.png",
            self.typ.as_str(),
            key.depth(),
            key.x(),
            key.y()
        )
    }

    fn remove_img_cache(self: &Self, ctx: &Context, will_del: &mut Vec<QTreeKey>) {
        println!("del:{}", will_del.len());
        will_del.iter().for_each(|k| {
            let uri = self.uri_of(*k);
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
}

impl TileLoader for PngLoader {
    fn load_img(self: &Self, key: QTreeKey, ctx: Context, vec: Arc<[u8]>) -> bool {
        let uri = self.uri_of(key);
        let img = Image::from_bytes(uri, vec);
        match img.load_for_size(&ctx, TILE_SIZE_VEC2) {
            Ok(r) => {
                if let TexturePoll::Ready { texture } = r {
                    let mut will_del = self.mem_cache.put(key, Arc::new(texture));
                    if !will_del.is_empty() {
                        self.remove_img_cache(&ctx, &mut will_del);
                    }
                }
                true
            }
            Err(e) => {
                self.mem_cache.remove(key);
                let mut will_del: Vec<QTreeKey> = vec![key];
                self.remove_img_cache(&ctx, &mut will_del);
                println!("load_img error:{}", e);
                match e {
                    egui::load::LoadError::Loading(_) => true,
                    _ => false,
                }
            }
        }
    }

    fn mem_cache(self: &Self) -> &MemoryDrawableCache {
        &self.mem_cache
    }
}
