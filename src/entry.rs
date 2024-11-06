use std::sync::Arc;
use lru_mem::HeapSize;

#[allow(dead_code)]
pub(crate) struct CacheEntry<V> {
    pub value: Arc<V>,
    pub size: usize,
}

impl<V: HeapSize> HeapSize for CacheEntry<V> {
    fn heap_size(&self) -> usize {
        self.value.heap_size() + std::mem::size_of::<Self>()
    }
}

impl<V> CacheEntry<V> {
    pub fn new(value: V, size: usize) -> Self {
        Self {
            value: Arc::new(value),
            size,
        }
    }
}
