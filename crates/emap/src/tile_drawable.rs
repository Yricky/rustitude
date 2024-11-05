use std::sync::Arc;

use egui::{load::SizedTexture, pos2, vec2, Align2, Color32, FontId, Painter, Rect, Vec2};
use rustitude_base::qtree::QTreeKey;

pub const TILE_SIZE_VEC2: Vec2 = vec2(256.0, 256.0);

pub trait EguiTileDrawable: Send + Sync {
    fn draw(&self, painter: &Painter, rect: Rect);
    fn clip(&self, rect: Rect) -> Option<CommonEguiTileDrawable>;
}

pub type CommonEguiTileDrawable = Arc<dyn EguiTileDrawable>;

impl EguiTileDrawable for SizedTexture {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.image(
            self.id,
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    fn clip(&self, rect: Rect) -> Option<CommonEguiTileDrawable> {
        let d = (self.clone(), rect);
        Some(Arc::new(d))
    }
}

impl EguiTileDrawable for (SizedTexture, Rect) {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.image(self.0.id, rect, self.1, Color32::WHITE);
    }

    fn clip(&self, rect: Rect) -> Option<CommonEguiTileDrawable> {
        let self_size = self.1.size();
        let rect_size = rect.size();
        Some(Arc::new((
            self.0,
            Rect::from_min_size(
                pos2(
                    self.1.min.x + self_size.x * rect.min.x,
                    self.1.min.y + self_size.y * rect.min.y,
                ),
                vec2(self_size.x * rect_size.x, self_size.y * rect_size.y),
            ),
        )))
    }
}

impl EguiTileDrawable for QTreeKey {
    fn draw(&self, painter: &Painter, rect: Rect) {
        painter.text(
            rect.min,
            Align2::LEFT_TOP,
            format!("{}", self),
            FontId {
                size: 8.0,
                family: egui::FontFamily::Monospace,
            },
            Color32::from_gray(0xff),
        );
    }

    fn clip(&self, _rect: Rect) -> Option<CommonEguiTileDrawable> {
        None
    }
}
