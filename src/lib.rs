#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(missing_debug_implementations, rust_2018_idioms, missing_docs)]

//! High-performance multi-tiered cache with automatic sizing and async support.

mod config;
mod entry;
mod stats;
mod tier;

pub use config::{CacheConfig, TierConfig};
pub use stats::{CacheStats, TierStats};

use crossbeam_utils::CachePadded;
use dashmap::DashMap;
use entry::CacheEntry;
use futures::Future;
use lru_mem::HeapSize;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::{hash::Hash, sync::Arc};
use tier::Tier;
use tokio::sync::broadcast;

type TierVec<K, V> = SmallVec<[Arc<CachePadded<Tier<K, V>>>; 4]>;

/// High-performance multi-tiered cache with automatic sizing
#[derive(Debug)]
pub struct TieredCache<K: Hash + Eq, V> {
    tiers: TierVec<K, V>,
    key_to_tier: Arc<DashMap<K, usize>>,
    config: Arc<CacheConfig>,
    update_tx: Option<broadcast::Sender<K>>,
}

impl<K, V> TieredCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + HeapSize + 'static,
    V: Clone + Send + Sync + HeapSize + 'static,
{
    /// Creates a new cache
    #[inline]
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        let tiers = config
            .tiers
            .iter()
            .map(|tier_config| {
                Arc::new(CachePadded::new(Tier::new(
                    tier_config.total_capacity,
                    tier_config.size_range,
                )))
            })
            .collect();

        // Calculate total cache size in bytes
        let total_cache_size: usize = config.tiers.iter().map(|t| t.total_capacity).sum();

        let tx = config
            .update_channel_size
            .map(|size| broadcast::channel(size).0);

        Self {
            tiers,
            key_to_tier: Arc::new(DashMap::with_capacity(
                total_cache_size / std::mem::size_of::<(K, usize)>(),
            )),
            config: Arc::new(config),
            update_tx: tx,
        }
    }

    /// Gets or updates a cache entry asynchronously
    #[inline]
    pub async fn get_or_update<F, Fut>(&self, key: K, updater: F) -> Option<Arc<V>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<V>>,
    {
        // Fast path: check cache first
        if let Some(value) = self.get(&key) {
            return Some(value);
        }

        // Slow path: update cache
        self.update_value(key, updater).await
    }

    #[inline]
    async fn update_value<F, Fut>(&self, key: K, updater: F) -> Option<Arc<V>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<V>>,
    {
        if let Some(new_value) = updater().await {
            self.put(key.clone(), new_value.clone());
            self.notify_update(key);
            Some(Arc::new(new_value))
        } else {
            None
        }
    }

    /// Puts a value into the cache
    #[inline]
    pub fn put(&self, key: K, value: V) -> Option<V> {
        let size = value.heap_size();

        let old_tier = self.key_to_tier.get(&key);
        let new_tier = self.find_tier_for_size(size);

        match (old_tier, new_tier) {
            // key does not fit into any tier
            (Some(old_idx), Some(new_idx)) if *old_idx == new_idx => {
                // Fast path: same tier
                let tier = &self.tiers[*old_idx];
                let entry = CacheEntry::new(value, size);
                tier.put(key, entry)
            }
            (Some(old_idx), Some(new_idx)) => {
                // Move to new tier
                let tier = &self.tiers[*old_idx];
                tier.remove(&key);
                drop(old_idx);
                self.key_to_tier.insert(key.clone(), new_idx);
                let entry = CacheEntry::new(value, size);
                self.tiers[new_idx].put(key, entry)
            }
            (_, Some(new_idx)) => {
                // New entry
                self.key_to_tier.insert(key.clone(), new_idx);
                let entry = CacheEntry::new(value, size);
                self.tiers[new_idx].put(key, entry)
            }
            _ => None,
        }
    }

    /// Gets a value from the cache
    #[inline]
    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        // Fast path: direct tier lookup
        let tier_idx = self.key_to_tier.get(key)?;
        let tier = &self.tiers[*tier_idx];
        tier.get(key)
    }

    /// Subscribes to cache updates
    #[inline]
    #[must_use]
    pub fn subscribe_updates(&self) -> Option<broadcast::Receiver<K>> {
        self.update_tx.as_ref().map(|tx| tx.subscribe())
    }

    #[inline]
    fn notify_update(&self, key: K) {
        self.update_tx.as_ref().map(|tx| tx.send(key));
    }

    #[inline]
    fn find_tier_for_size(&self, size: usize) -> Option<usize> {
        // Optimize for common case of small items
        self.tiers.iter().position(|tier| size < tier.size_range.1)
    }

    /// Gets cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let mut tier_stats = Vec::with_capacity(self.tiers.len());
        let mut total_items = 0;
        let mut total_size = 0;

        for (tier, config) in self.tiers.iter().zip(self.config.tiers.iter()) {
            let stats = tier.stats(config);
            total_items += stats.items;
            total_size += stats.size;
            tier_stats.push(stats);
        }

        CacheStats {
            tier_stats,
            total_items,
            total_size,
        }
    }

    /// Removes a value from the cache
    #[inline]
    pub fn remove(&self, key: &K) -> Option<V> {
        let tier_idx = self.key_to_tier.remove(key)?;
        let tier = &self.tiers[tier_idx.1];
        tier.remove(key)
    }

    /// Clears the cache
    pub fn clear(&self) {
        for tier in &self.tiers {
            tier.clear();
        }
        self.key_to_tier.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache() {
        let config = CacheConfig {
            tiers: vec![
                TierConfig {
                    total_capacity: 200 * 1024 * 1024,
                    size_range: (0, 64),
                },
                TierConfig {
                    total_capacity: 300 * 1024 * 1024,
                    size_range: (65, 1024 * 1024),
                },
            ],
            update_channel_size: None,
        };

        let cache = TieredCache::<Vec<u8>, Vec<u8>>::new(config);

        let key: Vec<u8> = b"example".to_vec();

        assert!(cache.put(key.clone(), vec![0u8; 1]).is_none());
        cache.put(key.clone(), vec![0u8; 65]);

        let retrieved = cache.get(&key).unwrap();
        assert_eq!(retrieved.len(), 65);

        cache.remove(&key);
        assert_eq!(cache.get(&key), None);

        let stats = cache.stats();
        assert_eq!(stats.total_items, 0);
        assert_eq!(stats.total_size, 0);
    }
}
