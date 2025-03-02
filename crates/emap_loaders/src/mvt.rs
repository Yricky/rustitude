use std::sync::Arc;

use crate::{MemoryDrawableCache, TileLoader};
use egui::{Align2, Color32, Context, FontId, Pos2};
use emap::tile_drawable::EguiTileDrawable;
use rustitude_base::qtree::QTreeKey;
use rustitude_mvt::mvt::tile::{Feature, Layer, Tile};

pub struct MvtLoader {
    pub typ: String,
    pub mem_cache: MemoryDrawableCache,
}

impl TileLoader for MvtLoader {
    fn load_img(self: &Self, key: QTreeKey, ctx: Context, vec: Arc<[u8]>) -> bool {
        if let Ok(mvt) = Tile::decode(&vec) {
            let _ = self.mem_cache.put(key, Arc::new(MvtLayer(mvt.layers)));
            true
        } else {
            false
        }
    }

    fn mem_cache(self: &Self) -> &MemoryDrawableCache {
        &self.mem_cache
    }
}

pub struct MvtLayer(Vec<Layer>);
impl EguiTileDrawable for MvtLayer {
    fn draw(&self, painter: &egui::Painter, rect: egui::Rect) {
        let width = rect.width();
        let height = rect.height();
        self.0.iter().for_each(|l| {
            l.features.iter().for_each(|f| {
                match &f.geometry {
                    rustitude_mvt::mvt::tile::Geometry::UnKnown => {}
                    rustitude_mvt::mvt::tile::Geometry::Point { points } => {
                        points
                            .iter()
                            .map(|p| {
                                Pos2::new((rect.min.x) + width * p.0, (rect.min.y) + height * p.1)
                            })
                            .for_each(|p| {
                                // painter.circle_filled(p, 2.0, Color32::RED);
                                let name = f
                                    .props
                                    .get("name")
                                    .map(|v| v.string_value())
                                    .unwrap_or_default();
                                painter.text(
                                    p,
                                    Align2::CENTER_CENTER,
                                    format!("{}\n{}", name, l.name),
                                    FontId {
                                        size: 8.0,
                                        family: egui::FontFamily::Monospace,
                                    },
                                    Color32::WHITE,
                                );
                            });
                    }
                };
            })
        });
        // todo!()
    }

    fn clip(&self, rect: egui::Rect) -> Option<emap::tile_drawable::CommonEguiTileDrawable> {
        return None;
    }
}
