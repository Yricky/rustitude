use std::{
    sync::{Arc, RwLock},
    thread,
};
pub mod view;
use egui::{
    load::BytesLoader, vec2, Align2, Color32, FontId, Margin, Painter, Pos2, Rect, Rounding, Sense,
    Stroke,
};
use rustitude_base::{
    map_state::{walk, Location},
    map_view_state::{MapViewState, TILE_SIZE},
};
use view::{
    clip_from_top_key,
    egui::{EguiMapImgRes, EguiMapImgResImpl},
};

fn main() {
    let rc = Arc::new("value");
    thread::spawn(move || println!("{}", rc.clone()));
    let _ = eframe::run_native(
        "app_name",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MapViewStateTestApp {
                map_view_state: Arc::new(RwLock::new(MapViewState {
                    central: Location { x: 0.5, y: 0.5 },
                    view_size: [1280.0, 800.0],
                    zoom_lvl: 2.0,
                })),
                main_res: Arc::new(EguiMapImgResImpl::new("img")),
                other_res: vec![
                    Arc::new(EguiMapImgResImpl::new("cia")),
                    Arc::new(EguiMapImgResImpl::new("cva")),
                ],
                debug: false,
            }))
        }),
    );
    println!("Hello, world!");
}

trait EguiMap {
    fn egui_map(
        self: &mut Self,
        ui: &mut egui::Ui,
        res: Arc<dyn EguiMapImgRes>,
        other_res: &Vec<Arc<dyn EguiMapImgRes>>,
        self_ref: Arc<RwLock<Self>>,
        debug: bool,
    ) -> egui::Response;
}

impl EguiMap for MapViewState {
    fn egui_map(
        self: &mut Self,
        ui: &mut egui::Ui,
        res: Arc<dyn EguiMapImgRes>,
        other_res: &Vec<Arc<dyn EguiMapImgRes>>,
        self_ref: Arc<RwLock<Self>>,
        debug: bool,
    ) -> egui::Response {
        let rect = ui.available_rect_before_wrap();
        self.view_size[0] = rect.width() as f64;
        self.view_size[1] = rect.height() as f64;
        let painter = Painter::new(ui.ctx().clone(), ui.layer_id(), rect);

        let ppos = ui.input(|i| i.pointer.latest_pos().map(|pos| pos - rect.left_top()));
        let scroll = ui.input(|i| i.smooth_scroll_delta);
        let zoom = ui.input(|i| i.zoom_delta());
        let click = ui.input(|i| i.pointer.any_click());
        if let Some(p) = ppos {
            if zoom != 1.0 {
                self.apply_zoom_delta(zoom.into(), [p.x.into(), p.y.into()]);
            }
        }
        if scroll.x != 0.0 || scroll.y != 0.0 {
            self.set_central(Location::new(
                self.central.x - (scroll.x as f64) / (TILE_SIZE * self.zoom()),
                self.central.y - (scroll.y as f64) / (TILE_SIZE * self.zoom()),
            ));
        }

        walk(self.top_left_key(), self.bottom_right_key()).for_each(|k| {
            let lt = self.location_to_view_pos(Location::from_qtree_key(k));
            let screen_zoom = (TILE_SIZE * 2.0_f64.powf(self.zoom_lvl - k.depth() as f64)) as f32;
            let ltpos = Pos2::new(lt[0] as f32, lt[1] as f32) + rect.min.to_vec2();
            let this_rect = Rect::from_min_size(ltpos, vec2(screen_zoom, screen_zoom));
            let mut tile = res.get(k, self_ref.clone(), ui.ctx());
            //tile对应的key
            let mut k1 = Some(k);
            while tile.is_none() && k1.is_some() {
                k1 = k1.unwrap().parent();
                if let Some(k1) = k1 {
                    tile = res.get(k1, self_ref.clone(), ui.ctx());
                    if let Some(t) = tile {
                        tile = Some(t.clip(clip_from_top_key(k1, k)));
                    }
                }
            }
            if let Some(t) = tile {
                t.draw(&painter, this_rect);
            } else {
                painter.rect_filled(
                    this_rect,
                    Rounding::ZERO,
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
            other_res.iter().for_each(|r| {
                let tile = r.get(k, self_ref.clone(), ui.ctx());
                if let Some(t) = tile {
                    t.draw(&painter, this_rect);
                }
            });
            if debug {
                painter.text(
                    ltpos,
                    Align2::LEFT_TOP,
                    format!("{}", k),
                    FontId {
                        size: 8.0,
                        family: egui::FontFamily::Monospace,
                    },
                    Color32::from_gray(0xff),
                );
            }
        });
        if debug {
            painter.rect_stroke(
                rect.shrink(1.0),
                Rounding::same(0.0),
                Stroke::new(1.0, Color32::from_rgb(0xff, 0x11, 0)),
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
                        self.view_pos_to_location([pos.x as f64, pos.y as f64])
                    ))
                    .unwrap_or(String::from("None"))
                ));
                ui.label(format!("Scroll delta:{}", scroll));
                ui.label(format!("Zoom delta:{}", zoom));
                ui.label(format!("Click:{}", click));
                ui.label("--------------------");
                ui.label(format!("Center:{}", self.central));
                ui.label(format!("Zoom level:{}", self.zoom_lvl as u8));
                ui.label(format!("Top left:{}", self.top_left_key()));
                ui.label(format!("Bottom right:{}", self.bottom_right_key()));
                ui.label("--------------------");
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
            });
        }
        ui.allocate_rect(rect, Sense::click_and_drag())
    }
}

struct MapViewStateTestApp {
    map_view_state: Arc<RwLock<MapViewState>>,
    main_res: Arc<dyn EguiMapImgRes>,
    other_res: Vec<Arc<dyn EguiMapImgRes>>,
    debug: bool,
}

impl eframe::App for MapViewStateTestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::canvas(&ctx.style()).inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                if let Some(cpu_usage) = frame.info().cpu_usage {
                    ui.label(format!("cpuTime:{}ms", cpu_usage * 1000.0));
                }
                ui.checkbox(&mut self.debug, "debug");
                let mut nvs = self.map_view_state.write().unwrap();
                nvs.egui_map(
                    ui,
                    self.main_res.clone(),
                    &self.other_res,
                    self.map_view_state.clone(),
                    self.debug,
                );
            });
    }
}
