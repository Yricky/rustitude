use std::sync::Arc;

use crate::map_state::Location;

#[derive(Clone, Copy)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

pub trait WCS {
    fn to_lat_lng(&self, location: Location) -> LatLng;
    fn to_location(&self, lat_lng: LatLng) -> Location;
}

impl<T> WCS for Arc<T>
where
    T: WCS + Send + Sync,
{
    fn to_lat_lng(&self, location: Location) -> LatLng {
        self.as_ref().to_lat_lng(location)
    }

    fn to_location(&self, lat_lng: LatLng) -> Location {
        self.as_ref().to_location(lat_lng)
    }
}

pub struct WGS84();
impl WGS84 {
    pub const INST: WGS84 = WGS84();
}

impl WCS for WGS84 {
    fn to_lat_lng(&self, location: Location) -> LatLng {
        todo!()
    }

    fn to_location(&self, lat_lng: LatLng) -> Location {
        todo!()
    }
}
