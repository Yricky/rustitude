use std::{fs, sync::Arc};

use rustitude_base::qtree::QTreeKey;

use crate::BinTileCache;


pub struct DiskDirTileCache {
    pub cache_path_prefix: String,
    pub file_ext: String,
}

impl DiskDirTileCache {
    fn path_of(&self, key: QTreeKey) -> String {
        format!(
            "{}/{}_{}_{}.{}",
            self.cache_path_prefix.as_str(),
            key.depth(),
            key.x(),
            key.y(),
            self.file_ext.as_str()
        )
    }
}

impl BinTileCache for DiskDirTileCache {
    fn save(&self, key: QTreeKey, value: Arc<[u8]>) {
        if !fs::exists(self.cache_path_prefix.as_str()).unwrap_or(false) {
            let _ = fs::create_dir_all(self.cache_path_prefix.as_str());
        }
        let cache_file_path = self.path_of(key);
        let lock_file_path = format!("{}.tmp", cache_file_path.as_str());
        if fs::exists(lock_file_path.as_str()).unwrap_or(false) {
            return;
        }
        fs::write(lock_file_path.clone(), value).unwrap();
        let _ = fs::rename(lock_file_path, cache_file_path.as_str());
    }

    fn load(&self, key: QTreeKey) -> Option<Arc<[u8]>> {
        let cache_file_path = self.path_of(key);
        return fs::read(cache_file_path.as_str()).map(|v| v.into()).ok();
    }

    fn exist(&self, key: QTreeKey) -> bool {
        let cache_file_path = self.path_of(key);
        fs::exists(cache_file_path.as_str()).unwrap_or(false)
    }

    fn delete(&self, key: QTreeKey) {
        let cache_file_path = self.path_of(key);
        let _ = fs::remove_file(cache_file_path.as_str());
    }
}