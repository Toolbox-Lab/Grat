use dashmap::DashMap;
use std::hash::Hash;
use std::sync::Arc;

pub trait CacheProvider<K, V> {
    fn get(&self, key: &K) -> Option<Arc<V>>;
    fn insert(&self, key: K, value: V);
    fn remove(&self, key: &K) -> Option<Arc<V>>;
    fn clear(&self);
}

pub struct MemoryCache<K, V> {
    store: DashMap<K, Arc<V>>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
        }
    }
}

impl<K, V> Default for MemoryCache<K, V>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> CacheProvider<K, V> for MemoryCache<K, V>
where
    K: Eq + Hash + Clone,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        self.store.get(key).map(|v| Arc::clone(v.value()))
    }

    fn insert(&self, key: K, value: V) {
        self.store.insert(key, Arc::new(value));
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        self.store.remove(key).map(|(_, v)| v)
    }

    fn clear(&self) {
        self.store.clear();
    }
}
