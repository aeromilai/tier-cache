#![deny(missing_docs)]
#![deny(unsafe_code)]

//! High-performance multi-tiered cache with automatic sizing and async support.

mod config;
mod entry;
mod stats;
mod tier;

pub use config::{CacheConfig, TierConfig};
pub use stats::{CacheStats, TierStats};

use crossbeam_utils::CachePadded;
use lru_mem::HeapSize;
use dashmap::DashMap;
use entry::CacheEntry;
use futures::Future;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::{hash::Hash, sync::Arc, future::Future};
use tokio::sync::oneshot;
use tokio::sync::broadcast;
use tier::Tier;


type TierVec<K, V> = SmallVec<[Arc<CachePadded<Tier<K, V>>>; 4]>;

/// High-performance multi-tiered cache with automatic sizing
pub struct TieredCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + HeapSize + 'static,
    V: Clone + Send + Sync + HeapSize + 'static,
{
    tiers: TierVec<K, V>,
    key_to_tier: Arc<DashMap<K, usize>>,
    config: Arc<CacheConfig>,
    update_tx: Option<broadcast::Sender<K>>,
    put_lock: parking_lot::RwLock<()>,
    pending_updates: DashMap<K, oneshot::Sender<Option<Arc<V>>>>,
}

impl<K, V> TieredCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + HeapSize + 'static,
    V: Clone + Send + Sync + HeapSize + 'static,
{
    /// Creates a new cache
    #[inline]
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
        let total_cache_size: usize = config.tiers.iter()
            .map(|t| t.total_capacity)
            .sum();

        let tx = config
            .update_channel_size
            .map(|size| broadcast::channel(size).0);

        Self {
            tiers,
            key_to_tier: Arc::new(DashMap::with_capacity(
                total_cache_size / std::mem::size_of::<(K, usize)>()
            )),
            config: Arc::new(config),
            update_tx: tx,
            put_lock: parking_lot::RwLock::new(()),
            pending_updates: DashMap::new(),
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

        // Check if there's already an update in progress
        if let Some(entry) = self.pending_updates.get(&key) {
            // Create a new channel to receive the result
            let (tx, rx) = oneshot::channel();
            // Clone the key for the removal closure
            let key_clone = key.clone();
            let pending_updates = self.pending_updates.clone();
            
            // Forward the result to our new channel
            tokio::spawn(async move {
                if let Ok(result) = rx.await {
                    let _ = tx.send(result);
                }
                pending_updates.remove(&key_clone);
            });
            
            return rx.await.ok().flatten();
        }

        // No update in progress, we'll do it
        let (tx, rx) = oneshot::channel();
        self.pending_updates.insert(key.clone(), tx);

        // Perform the update
        let result = self.update_value(key.clone(), updater).await;
        
        // Share result with other waiters
        if let Some(entry) = self.pending_updates.remove(&key) {
            let _ = entry.1.send(result.clone());
        }

        result
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
        
        // Fast path: check if size is within any tier
        let tier_idx = self.find_tier_for_size(size)?;
        
        // Acquire write lock for the entire operation
        let _guard = self.put_lock.write();
        
        // Remove from ALL other tiers to ensure consistency
        let mut old_value = None;
        for (i, tier) in self.tiers.iter().enumerate() {
            if i != tier_idx {
                if let Some(removed) = tier.remove(&key) {
                    old_value = Some(removed);
                }
            }
        }
        
        let entry = CacheEntry::new(value, size);
        let tier = &self.tiers[tier_idx];
        
        // Update mapping and insert new value
        self.key_to_tier.insert(key.clone(), tier_idx);
        old_value.or_else(|| tier.put(key, entry))
        
        // Lock is automatically released when _guard goes out of scope
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
        if !self.tiers.is_empty() && size < self.config.tiers[0].size_range.1 {
            return Some(0);
        }

        // Binary search for larger items
        self.config
            .tiers
            .binary_search_by_key(&size, |tier| tier.size_range.0)
            .ok()
    }

    /// Gets cache statistics
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
        // Acquire write lock for the entire operation
        let _guard = self.put_lock.write();
        
        let tier_idx = self.key_to_tier.remove(key)?;
        let tier = &self.tiers[tier_idx.1];
        tier.remove(key)
        // Lock is automatically released when _guard goes out of scope
    }

    /// Clears the cache
    pub fn clear(&self) {
        for tier in &self.tiers {
            tier.clear();
        }
        self.key_to_tier.clear();
    }
}

