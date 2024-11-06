use std::sync::Arc;
use std::time::{Duration, Instant};
use lru_mem::HeapSize;

#[allow(dead_code)]
pub(crate) struct CacheEntry<V> {
    pub value: Arc<V>,
    pub size: usize,
    pub expires_at: Option<Instant>,
}

impl<V: HeapSize> HeapSize for CacheEntry<V> {
    fn heap_size(&self) -> usize {
        self.value.heap_size() + std::mem::size_of::<Self>()
    }
}

impl<V> CacheEntry<V> {
    pub fn new(value: V, size: usize, ttl: Option<Duration>) -> Self {
        Self {
            value: Arc::new(value),
            size,
            expires_at: ttl.map(|ttl| Instant::now() + ttl),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.expires_at
            .map(|expires| expires > Instant::now())
            .unwrap_or(true)
    }
}
