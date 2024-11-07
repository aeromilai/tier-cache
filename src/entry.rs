use lru_mem::HeapSize;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct CacheEntry<V> {
    pub value: Arc<V>,
    pub _size: usize,
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
            _size: size,
        }
    }
}
