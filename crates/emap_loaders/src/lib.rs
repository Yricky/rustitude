use std::fmt::Display;

use ehttp::Request;

pub mod png;

pub trait RequestBuilder: Send + Sync {
    fn build_req(&self, typ: &str, x: u32, y: u32, z: u8) -> Request;
}
