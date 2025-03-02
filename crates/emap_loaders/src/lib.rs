use std::sync::Arc;

use ehttp::{Request, Response};
use rustitude_base::qtree::QTreeKey;

#[cfg(feature = "png")]
pub mod png;

pub trait RequestBuilder: Send + Sync {
    fn build_req(&self, typ: &str, x: u32, y: u32, z: u8) -> Request;
    
    fn decode_response(&self, resp:Response) -> Arc<[u8]> {
        resp.bytes.into()
    }
}

pub trait BinTileCache: Send + Sync{
    fn save(&self,key:QTreeKey,value: Arc<[u8]>);
    fn load(&self,key:QTreeKey) -> Option<Arc<[u8]>>;
    fn exist(&self,key:QTreeKey) -> bool;
    fn delete(&self,key:QTreeKey);
}
