
// src/tier.rs
use super::*;
use lru_mem::HeapSize;

pub(crate) struct Tier<K, V> {
    cache: RwLock<lru_mem::LruCache<K, CacheEntry<V>>>,
    size_range: (usize, usize),
}

impl<K, V> Tier<K, V>
where
    K: Hash + Eq + Clone + HeapSize + Send + Sync + 'static,
    V: Clone + HeapSize + Send + Sync + 'static,
{
    pub fn new(capacity: usize, size_range: (usize, usize)) -> Self {
        Self {
            cache: RwLock::new(lru_mem::LruCache::new(capacity)),
            size_range,
        }
    }

    #[inline]
    pub fn put(&self, key: K, entry: CacheEntry<V>) -> Option<V> {
        let mut cache = self.cache.write();
        cache.insert(key, entry)
            .map(|old| Arc::try_unwrap(old.unwrap().value).unwrap_or_else(|arc| (*arc).clone()))
    }

    #[inline]
    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        let mut cache = self.cache.write();
        cache.get(key)
            .filter(|entry| entry.is_valid())
            .map(|entry| entry.value.clone())
    }

    #[inline]
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write();
        cache.remove(key)
            .map(|entry| Arc::try_unwrap(entry.value).unwrap_or_else(|arc| (*arc).clone()))
    }

    pub fn clear(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    pub fn stats(&self, config: &TierConfig) -> TierStats {
        let cache = self.cache.read();
        TierStats {
            items: cache.len(),
            size: cache.iter().map(|(_, entry)| entry.heap_size()).sum(),
            capacity: config.total_capacity,
            hit_count: 0,
            miss_count: 0,
        }
    }
}
