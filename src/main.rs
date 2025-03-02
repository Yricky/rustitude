use egui::Margin;
use ehttp::Request;
use emap::{egui_map::EguiMap, DebugPrintKeyTileRes, EguiMapTileRes};
use emap_loaders::{png::EguiMapPngResImpl, RequestBuilder};
use rustitude_base::{latlng::{WebMercator, WCS}, map_state::Location, map_view_state::MapViewState};
use std::sync::{Arc, RwLock};

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
                main_res: Arc::new(EguiMapPngResImpl::new(
                    "img",
                    Some("tiles"),
                    Arc::new(ShipxyReqBuilder),
                )),
                other_res: vec![
                    // Arc::new(EguiMapPngResImpl::new(
                    //     "cia",
                    //     Some("tiles"),
                    //     Arc::new(TiandituRequestBuilder::Test),
                    // )),
                    // Arc::new(EguiMapPngResImpl::new(
                    //     "cva",
                    //     Some("tiles"),
                    //     Arc::new(TiandituRequestBuilder::Test),
                    // )),
                ],
                debug: false,
            }))
        }),
    );
}

pub struct ShipxyReqBuilder;
impl RequestBuilder for ShipxyReqBuilder {
    fn build_req(&self, _typ: &str, x: u32, y: u32, z: u8) -> ehttp::Request {
        Request::get(format!(
            "https://gwxc.shipxy.com/tile.g?z={}&x={}&y={}",
            z, x, y
        ))
    }
}

struct MapViewStateTestApp {
    map_view_state: Arc<RwLock<MapViewState>>,
    main_res: Arc<dyn EguiMapTileRes>,
    other_res: Vec<Arc<dyn EguiMapTileRes>>,
    debug: bool,
}

impl EguiMap for MapViewStateTestApp {
    fn map_view_state(&self) -> Arc<RwLock<MapViewState>> {
        self.map_view_state.clone()
    }
}

impl eframe::App for MapViewStateTestApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::canvas(&ctx.style()).inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                ui.horizontal(|ui|{
                    ui.label(format!("{}",WebMercator.to_lat_lng(self.map_view_state.read().unwrap().central)));
                    if let Some(cpu_usage) = frame.info().cpu_usage {
                        ui.label(format!("cpuTime:{}ms", cpu_usage * 1000.0));
                    }
                });

                ui.checkbox(&mut self.debug, "debug");
                if self.debug && self.other_res.len() == 0 {
                    self.other_res.push(Arc::new(DebugPrintKeyTileRes));
                } else if !self.debug && self.other_res.len() == 1 {
                    self.other_res.pop();
                }
                self.egui_map(ui, self.main_res.clone(), &self.other_res, self.debug);
            });
    }
}
