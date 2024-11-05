use std::sync::{Arc, RwLock};
pub mod view;
use egui::{Margin, Rect, Sense};
use emap::{egui_map::EguiMap, EguiMapImgRes};
use rustitude_base::{map_state::Location, map_view_state::MapViewState};
use view::egui::EguiMapImgResImpl;

fn main() {
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
                self.map_view_state.egui_map(
                    ui,
                    self.main_res.clone(),
                    &self.other_res,
                    self.debug,
                );
            });
    }
}
