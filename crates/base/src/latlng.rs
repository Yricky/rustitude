use std::{f64::consts::PI, fmt::Display, sync::Arc};

use crate::map_state::Location;

#[derive(Clone, Copy)]
pub struct LatLng {
    pub lat: f64,
    pub lng: f64,
}

impl Display for LatLng {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{lat:{},lng:{}}}", self.lat, self.lng)
    }
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

pub struct WebMercator;
impl WebMercator {
    // 地球半径（Web墨卡托投影参数）
    const EARTH_RADIUS: f64 = 6378137.0;
    // 墨卡托投影最大范围（赤道周长）
    const MERCATOR_MAX: f64 = 20037508.342789244;
}

impl WCS for WebMercator {
    fn to_lat_lng(&self, location: Location) -> LatLng {
        // 计算经度（直接线性映射）
        let lng = location.x * 360.0 - 180.0;
        // 计算墨卡托投影Y坐标（注意坐标系翻转）
        let y_merc = WebMercator::MERCATOR_MAX * (1.0 - 2.0 * location.y);
        // 通过反双曲正切计算纬度（核心转换公式）
        let lat_rad = (y_merc / WebMercator::EARTH_RADIUS).sinh().atan();
        let lat = lat_rad.to_degrees();

        LatLng { lat, lng }
    }

    fn to_location(&self, lat_lng: LatLng) -> Location {
        // 经度线性映射到[0,1]范围
        let x = (lat_lng.lng + 180.0) / 360.0;
        // 将纬度转换为墨卡托投影坐标
        let lat_rad = lat_lng.lat.to_radians();
        let y_merc = WebMercator::EARTH_RADIUS * (PI / 4.0 + lat_rad / 2.0).tan().ln();
        // 将墨卡托坐标归一化并翻转Y轴
        let y = (WebMercator::MERCATOR_MAX - y_merc) / (2.0 * WebMercator::MERCATOR_MAX);

        Location { x, y }
    }
}

// 单元测试验证关键坐标点
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion() {
        // 测试左上角（0,0）对应西北角
        let nw = WebMercator.to_lat_lng(Location { x: 0.0, y: 0.0 });
        assert_eq!(nw.lng, -180.0);
        assert_eq!(nw.lat, 85.0511287798066);
        let nl = WebMercator.to_location(nw);
        assert_eq!(nl.x, 0.0);
        assert_eq!(nl.y, 0.0);

        // 测试中心点（0.5,0.5）对应赤道
        let center = WebMercator.to_lat_lng(Location { x: 0.5, y: 0.5 });
        assert_eq!(center.lng, 0.0);
        assert_eq!(center.lat, 0.0);

        // 测试右下角（1,1）对应东南角
        let se = WebMercator.to_lat_lng(Location { x: 1.0, y: 1.0 });
        assert_eq!(se.lng, 180.0);
        assert_eq!(se.lat, -85.0511287798066);
    }
}
