use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CacheStats {
    pub tier_stats: Vec<TierStats>,
    pub total_items: usize,
    pub total_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TierStats {
    pub items: usize,
    pub size: usize,
    pub capacity: usize,
    pub hit_count: u64,
    pub miss_count: u64,
}
