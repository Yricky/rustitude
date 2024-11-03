use crate::base::{map_state::Location, qtree::QTreeKey};

pub mod egui;
pub struct MapViewState {
    pub central: Location,
    pub view_size: [f64; 2],
    pub zoom_lvl: f64,
}

pub static TILE_SIZE: f64 = 256.0;

impl MapViewState {
    pub fn zoom(&self) -> f64 {
        2.0_f64.powf(self.zoom_lvl)
    }

    pub fn apply_zoom_delta(&mut self, delta: f64, zoom_central: Location) {
        self.zoom_lvl += delta.log2();
        if self.zoom_lvl < 2.0 {
            self.zoom_lvl = 2.0
        } else if self.zoom_lvl > 18.5 {
            self.zoom_lvl = 18.5
        } else {
            let zc_c_delta = self.central - zoom_central;
            self.set_central(
                self.central
                    - Location::new(zc_c_delta.x * (delta - 1.0), zc_c_delta.y * (delta - 1.0)),
            );
        }
    }

    pub fn set_central(&mut self, central: Location) {
        self.central = central;
        if self.central.x < 0.0 {
            self.central.x = 0.0
        } else if self.central.x > 1.0 {
            self.central.x = 1.0
        }
        if self.central.y < 0.0 {
            self.central.y = 0.0
        } else if self.central.y > 1.0 {
            self.central.y = 1.0
        }
    }

    pub fn view_pos_to_location(&self, pos: [f64; 2]) -> Location {
        let zoom = self.zoom();
        let x: f64 = self.central.x + (pos[0] - self.view_size[0] / 2.0) / (TILE_SIZE * zoom);
        let y: f64 = self.central.y + (pos[1] - self.view_size[1] / 2.0) / (TILE_SIZE * zoom);
        Location::new(
            if x < 0.0 {
                0.0
            } else if x > 1.0 {
                1.0
            } else {
                x
            },
            if y < 0.0 {
                0.0
            } else if y > 1.0 {
                1.0
            } else {
                y
            },
        )
    }

    pub fn location_to_view_pos(&self, location: Location) -> [f64; 2] {
        let zoom = self.zoom();
        let x = (location.x - self.central.x) * (TILE_SIZE * zoom) + self.view_size[0] / 2.0;
        let y = (location.y - self.central.y) * (TILE_SIZE * zoom) + self.view_size[1] / 2.0;
        [x, y]
    }

    pub fn top_left_location(&self) -> Location {
        self.view_pos_to_location([0.0, 0.0])
    }

    pub fn top_left_key(&self) -> QTreeKey {
        self.top_left_location()
            .as_qtree_key(self.zoom_lvl as u8)
            .unwrap()
    }

    pub fn bottom_right_location(&self) -> Location {
        self.view_pos_to_location(self.view_size)
    }

    pub fn bottom_right_key(&self) -> QTreeKey {
        self.bottom_right_location()
            .as_qtree_key(self.zoom_lvl as u8)
            .unwrap()
    }
}
