use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct CacheEntry<V> {
    pub value: Arc<V>,
    pub size: usize,
    pub expires_at: Option<Instant>,
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
