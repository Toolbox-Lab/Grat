//! [`DiskCache`]: a [`CacheProvider`] backed by [`sled`], a persistent,
//! embedded, lock-free B+tree store. Values are serialized with `bincode`
//! before being written to the tree, and survive process restarts.

use crate::cache::provider::CacheProvider;
use crate::error::{GratError, GratResult};

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

/// Persistent, embedded cache backed by a [`sled`] database.
///
/// `sled` provides lock-free concurrent access internally, so `DiskCache`
/// can be freely cloned and shared across tasks; blocking I/O is offloaded
/// to `tokio::task::spawn_blocking` to keep the async executor unblocked.
#[derive(Clone)]
pub struct DiskCache {
    db: Arc<sled::Db>,
}

impl DiskCache {
    /// Opens (or creates) a sled database at `path`.
    pub fn new(path: impl AsRef<Path>) -> GratResult<Self> {
        let path = path.as_ref();
        let db = sled::open(path).map_err(|e| {
            GratError::CacheError(format!(
                "Failed to open sled database at {}: {e}",
                path.display()
            ))
        })?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Opens the disk cache at the platform's default cache directory.
    pub fn default_location() -> GratResult<Self> {
        let project_dirs =
            directories::ProjectDirs::from("dev", "grat", "grat").ok_or_else(|| {
                GratError::CacheError("Could not determine cache directory".to_string())
            })?;

        Self::new(project_dirs.cache_dir().join("disk_cache"))
    }

    /// Flushes any buffered writes to disk. Sled batches and lazily flushes
    /// writes for performance, so callers that need durability guarantees
    /// (e.g. before a restart) should await this explicitly.
    pub async fn flush(&self) -> GratResult<()> {
        self.db
            .flush_async()
            .await
            .map_err(|e| GratError::CacheError(format!("Sled flush failed: {e}")))?;
        Ok(())
    }
}

impl CacheProvider for DiskCache {
    fn get<V>(&self, key: &str) -> impl Future<Output = GratResult<Option<V>>> + Send
    where
        V: DeserializeOwned + Send,
    {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();

        async move {
            let raw = tokio::task::spawn_blocking({
                let key = key.clone();
                move || db.get(key.as_bytes())
            })
            .await
            .map_err(|e| GratError::CacheError(format!("Cache task panicked: {e}")))?
            .map_err(|e| GratError::CacheError(format!("Sled get failed: {e}")))?;

            let Some(bytes) = raw else {
                return Ok(None);
            };

            let value =
                bincode::deserialize(&bytes).map_err(|e| GratError::CacheDeserializationError {
                    key,
                    reason: e.to_string(),
                })?;

            Ok(Some(value))
        }
    }

    fn put<V>(&self, key: &str, value: &V) -> impl Future<Output = GratResult<()>> + Send
    where
        V: Serialize + Sync,
    {
        let key = key.to_owned();
        let encoded = bincode::serialize(value).map_err(|e| GratError::CacheSerializationError {
            key: key.clone(),
            reason: e.to_string(),
        });

        let db = Arc::clone(&self.db);

        async move {
            let encoded = encoded?;

            tokio::task::spawn_blocking(move || db.insert(key.as_bytes(), encoded))
                .await
                .map_err(|e| GratError::CacheError(format!("Cache task panicked: {e}")))?
                .map_err(|e| GratError::CacheError(format!("Sled insert failed: {e}")))?;

            Ok(())
        }
    }

    fn remove(&self, key: &str) -> impl Future<Output = GratResult<()>> + Send {
        let db = Arc::clone(&self.db);
        let key = key.to_owned();

        async move {
            tokio::task::spawn_blocking(move || db.remove(key.as_bytes()))
                .await
                .map_err(|e| GratError::CacheError(format!("Cache task panicked: {e}")))?
                .map_err(|e| GratError::CacheError(format!("Sled remove failed: {e}")))?;

            Ok(())
        }
    }

    fn clear(&self) -> impl Future<Output = GratResult<()>> + Send {
        let db = Arc::clone(&self.db);

        async move {
            tokio::task::spawn_blocking(move || db.clear())
                .await
                .map_err(|e| GratError::CacheError(format!("Cache task panicked: {e}")))?
                .map_err(|e| GratError::CacheError(format!("Sled clear failed: {e}")))?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        id: u32,
        name: String,
    }

    #[tokio::test]
    async fn test_put_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        let value = Sample {
            id: 1,
            name: "wasm-blob".to_string(),
        };
        cache.put("key1", &value).await.unwrap();

        let fetched: Option<Sample> = cache.get("key1").await.unwrap();
        assert_eq!(fetched, Some(value));
    }

    #[tokio::test]
    async fn test_put_overwrites_existing_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        cache.put("key1", &1u32).await.unwrap();
        cache.put("key1", &2u32).await.unwrap();

        assert_eq!(cache.get::<u32>("key1").await.unwrap(), Some(2));
    }

    #[tokio::test]
    async fn test_cache_miss_returns_ok_none() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        let fetched: Option<Sample> = cache.get("missing").await.unwrap();
        assert_eq!(fetched, None);
    }

    #[tokio::test]
    async fn test_remove() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        cache.put("key1", &42u32).await.unwrap();
        cache.remove("key1").await.unwrap();

        let fetched: Option<u32> = cache.get("key1").await.unwrap();
        assert_eq!(fetched, None);
    }

    #[tokio::test]
    async fn test_remove_of_missing_key_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        cache.remove("never-existed").await.unwrap();
    }

    #[tokio::test]
    async fn test_clear() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        cache.put("key1", &1u32).await.unwrap();
        cache.put("key2", &2u32).await.unwrap();
        cache.clear().await.unwrap();

        assert_eq!(cache.get::<u32>("key1").await.unwrap(), None);
        assert_eq!(cache.get::<u32>("key2").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_persistence_across_reopen() {
        let dir = tempfile::tempdir().unwrap();

        {
            let cache = DiskCache::new(dir.path()).unwrap();
            cache.put("durable", &"value".to_string()).await.unwrap();
            cache.flush().await.unwrap();
        }

        let reopened = DiskCache::new(dir.path()).unwrap();
        let fetched: Option<String> = reopened.get("durable").await.unwrap();
        assert_eq!(fetched, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_concurrent_put_get() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        let mut handles = Vec::new();
        for i in 0..20u32 {
            let cache = cache.clone();
            handles.push(tokio::spawn(async move {
                let key = format!("key-{i}");
                cache.put(&key, &i).await.unwrap();
                let fetched: Option<u32> = cache.get(&key).await.unwrap();
                assert_eq!(fetched, Some(i));
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_deserialize_type_mismatch_returns_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path()).unwrap();

        // bincode is not self-describing, so a mismatch only reliably surfaces
        // as an error when the stored bytes are too short for the target type
        // (here, a bare u32 has no bytes left for `Sample`'s trailing String).
        cache.put("key1", &1u32).await.unwrap();

        let err = cache.get::<Sample>("key1").await.unwrap_err();
        assert!(matches!(err, GratError::CacheDeserializationError { .. }));
    }
}
