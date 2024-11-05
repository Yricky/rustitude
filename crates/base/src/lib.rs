use std::time::{SystemTime, UNIX_EPOCH};

pub mod latlng;
pub mod map_state;
pub mod map_view_state;
pub mod qtree;

pub fn curr_time_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
