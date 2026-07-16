

use crate::error::{GratError, GratResult};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheCategory {

    WasmBlob,

    ContractSpec,

    LedgerEntry,

    TransactionResult,
}

impl CacheCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::WasmBlob => "wasm",
            Self::ContractSpec => "spec",
            Self::LedgerEntry => "ledger",
            Self::TransactionResult => "tx",
        }
    }
}

___RUST_DOC_COMMENT___
pub struct CacheStore {
___RUST_DOC_COMMENT___    
    cache_dir: PathBuf,
___RUST_DOC_COMMENT___    
    #[allow(dead_code)]
    max_size: u64,
}

impl CacheStore {
___RUST_DOC_COMMENT___    
    pub fn new(cache_dir: PathBuf, max_size_mb: u64) -> GratResult<Self> {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| GratError::CacheError(format!("Failed to create cache dir: {e}")))?;

        Ok(Self {
            cache_dir,
            max_size: max_size_mb * 1024 * 1024,
        })
    }

___RUST_DOC_COMMENT___    
    pub fn default_location() -> GratResult<Self> {
        let project_dirs =
            directories::ProjectDirs::from("dev", "grat", "grat").ok_or_else(|| {
                GratError::CacheError("Could not determine cache directory".to_string())
            })?;

        Self::new(project_dirs.cache_dir().to_path_buf(), 512)
    }

___RUST_DOC_COMMENT___    
    pub fn put(&self, category: CacheCategory, key: &str, value: &[u8]) -> GratResult<()> {
        if value.len() as u64 > self.max_size {
            return Err(GratError::CacheError(format!(
                "Cache entry exceeds configured cache size limit of {} bytes",
                self.max_size
            )));
        }

        let path = self.entry_path(category, key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| GratError::CacheError(format!("Failed to create dir: {e}")))?;
        }
        std::fs::write(&path, value)
            .map_err(|e| GratError::CacheError(format!("Failed to write cache entry: {e}")))?;
        Ok(())
    }

___RUST_DOC_COMMENT___    
    pub fn get(&self, category: CacheCategory, key: &str) -> GratResult<Option<Vec<u8>>> {
        let path = self.entry_path(category, key);
        if path.exists() {
            let data = std::fs::read(&path)
                .map_err(|e| GratError::CacheError(format!("Failed to read cache entry: {e}")))?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

___RUST_DOC_COMMENT___    
    pub fn contains(&self, category: CacheCategory, key: &str) -> bool {
        self.entry_path(category, key).exists()
    }

___RUST_DOC_COMMENT___    
    pub fn remove(&self, category: CacheCategory, key: &str) -> GratResult<()> {
        let path = self.entry_path(category, key);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                GratError::CacheError(format!("Failed to remove cache entry: {e}"))
            })?;
        }
        Ok(())
    }

___RUST_DOC_COMMENT___    
    pub fn clear(&self) -> GratResult<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)
                .map_err(|e| GratError::CacheError(format!("Failed to clear cache: {e}")))?;
            std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
                GratError::CacheError(format!("Failed to recreate cache dir: {e}"))
            })?;
        }
        Ok(())
    }

___RUST_DOC_COMMENT___    
    fn entry_path(&self, category: CacheCategory, key: &str) -> PathBuf {
        self.cache_dir.join(category.as_str()).join(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_roundtrip() {
        let dir = std::env::temp_dir().join("grat_test_cache");
        let store = CacheStore::new(dir.clone(), 10).unwrap();

        store
            .put(CacheCategory::WasmBlob, "test_key", b"hello")
            .unwrap();
        let result = store.get(CacheCategory::WasmBlob, "test_key").unwrap();
        assert_eq!(result, Some(b"hello".to_vec()));

        store.clear().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
}
