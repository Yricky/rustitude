use std::sync::{Arc, RwLock};

use egui::{
    load::BytesLoader, vec2, Color32, CornerRadius, InnerResponse, Painter, Pos2, Rect, Sense,
    Stroke,
};
use rustitude_base::{
    map_state::{walk, Location},
    map_view_state::{MapViewState, TILE_SIZE},
};

use crate::{clip_from_top_key, EguiMapTileRes};

pub trait EguiMap {
    fn egui_map(
        self: &Self,
        ui: &mut egui::Ui,
        res: Arc<dyn EguiMapTileRes>,
        other_res: &Vec<Arc<dyn EguiMapTileRes>>,
        debug: bool,
    ) -> egui::Response {
        let rect = ui.available_rect_before_wrap();
        let mvs_ref = self.map_view_state();
        let mut mvs = mvs_ref.write().unwrap();
        mvs.view_size[0] = rect.width() as f64;
        mvs.view_size[1] = rect.height() as f64;
        let painter = Painter::new(ui.ctx().clone(), ui.layer_id(), rect);

        let ppos = ui.input(|i| i.pointer.latest_pos().map(|pos| pos - rect.left_top()));
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        let zoom = ui.input(|i| i.zoom_delta());
        let click = ui.input(|i| i.pointer.any_click());
        if let Some(p) = ppos {
            if zoom != 1.0 {
                mvs.apply_zoom_delta(zoom.into(), [p.x.into(), p.y.into()]);
            }
        }
        let mvs_zoom = mvs.zoom();
        let mvs_central = mvs.central;
        if scroll.x != 0.0 || scroll.y != 0.0 {
            mvs.set_central(Location::new(
                mvs_central.x - (scroll.x as f64) / (TILE_SIZE * mvs_zoom),
                mvs_central.y - (scroll.y as f64) / (TILE_SIZE * mvs_zoom),
            ));
        }
        emap_default_impl_draw_map_tile(ui, &painter, &mut mvs, self.map_view_state(), res, true);
        other_res.iter().for_each(|res| {
            emap_default_impl_draw_map_tile(
                ui,
                &painter,
                &mut mvs,
                self.map_view_state(),
                res.clone(),
                false,
            );
        });
        if debug {
            painter.rect_stroke(
                rect.shrink(1.0),
                CornerRadius::ZERO,
                Stroke::new(1.0, Color32::from_rgb(0xff, 0x11, 0)),
                egui::StrokeKind::Middle,
            );
            ui.vertical(|ui| {
                ui.label(format!("Rect:{}", rect));
                ui.label(format!(
                    "Pointer position:{}",
                    ppos.map(|pos| format!("{}", pos))
                        .unwrap_or(String::from("None"))
                ));
                ui.label(format!(
                    "Pointer position:{}",
                    ppos.map(|pos| format!(
                        "{}",
                        mvs.view_pos_to_location([pos.x as f64, pos.y as f64])
                    ))
                    .unwrap_or(String::from("None"))
                ));
                ui.label(format!("Scroll delta:{}", scroll));
                ui.label(format!("Zoom delta:{}", zoom));
                ui.label(format!("Click:{}", click));
                ui.label("--------------------");
                emap_debug_mvs(ui, &mvs);
                ui.label("--------------------");
                emap_debug_loader_size(ui);
            });
        }
        ui.allocate_rect(rect, Sense::click_and_drag())
    }

    fn map_view_state(&self) -> Arc<RwLock<MapViewState>>;
}

pub fn emap_default_impl_draw_map_tile(
    ui: &mut egui::Ui,
    painter: &Painter,
    mvs: &MapViewState,
    mvs_ref: Arc<RwLock<MapViewState>>,
    res: Arc<dyn EguiMapTileRes>,
    is_base_tile: bool,
) {
    walk(mvs.top_left_key(), mvs.bottom_right_key()).for_each(|k| {
        let lt = mvs.location_to_view_pos(Location::from_qtree_key(k));
        let screen_zoom = (TILE_SIZE * 2.0_f64.powf(mvs.zoom_lvl - k.depth() as f64)) as f32;
        let ltpos = Pos2::new(lt[0] as f32, lt[1] as f32) + painter.clip_rect().min.to_vec2();
        let this_rect = Rect::from_min_size(ltpos, vec2(screen_zoom, screen_zoom));
        let mut tile = res.get_or_fetch(k, mvs_ref.clone(), ui.ctx());
        //tile对应的key
        let mut tile_key = Some(k);
        while tile.is_none() && tile_key.is_some() {
            tile_key = tile_key.unwrap().parent();
            if let Some(k1) = tile_key {
                tile = res.get_memory_cache(k1);
                if let Some(t) = tile {
                    tile = t.clip(clip_from_top_key(k1, k));
                }
            }
        }
        if let Some(t) = tile {
            t.draw(&painter, this_rect);
        } else if is_base_tile {
            painter.rect_filled(
                this_rect,
                CornerRadius::ZERO,
                Color32::from_rgb(
                    k.depth() * 8,
                    0xff - k.depth() * 8,
                    if (k.x() + k.y()) % 2 == 0 {
                        k.depth()
                    } else {
                        0xff - k.depth()
                    },
                ),
            );
        }
    });
}

pub fn emap_debug_mvs(ui: &mut egui::Ui, mvs: &MapViewState) -> InnerResponse<()> {
    ui.vertical(|ui| {
        ui.label(format!("Center:{}", mvs.central));
        ui.label(format!("Zoom level:{}", mvs.zoom_lvl as u8));
        ui.label(format!("Top left:{}", mvs.top_left_key()));
        ui.label(format!("Bottom right:{}", mvs.bottom_right_key()));
    })
}

pub fn emap_debug_loader_size(ui: &mut egui::Ui) -> InnerResponse<()> {
    ui.vertical(|ui| {
        ui.label(format!(
            "include_mem:{}",
            ui.ctx().loaders().include.byte_size()
        ));
        ui.label(format!(
            "texture_mem:{}",
            ui.ctx()
                .loaders()
                .texture
                .lock()
                .iter()
                .map(|l| l.byte_size())
                .sum::<usize>()
        ));
        ui.label(format!(
            "bytes_mem:{}",
            ui.ctx()
                .loaders()
                .bytes
                .lock()
                .iter()
                .map(|l| l.byte_size())
                .sum::<usize>()
        ));
        ui.label(format!(
            "image_mem:{}",
            ui.ctx()
                .loaders()
                .image
                .lock()
                .iter()
                .map(|l| l.byte_size())
                .sum::<usize>()
        ));
    })
}
