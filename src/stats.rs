use serde::Serialize;

/// Statistics for the entire cache system
#[derive(Clone, Debug, Serialize)]
pub struct CacheStats {
    /// Statistics for each individual tier
    pub tier_stats: Vec<TierStats>,
    /// Total number of items across all tiers
    pub total_items: usize,
    /// Total size in bytes across all tiers
    pub total_size: usize,
}

/// Statistics for a single cache tier
#[derive(Clone, Debug, Serialize)]
pub struct TierStats {
    /// Number of items currently in the tier
    pub items: usize,
    /// Current total size in bytes
    pub size: usize,
    /// Maximum capacity in bytes
    pub capacity: usize,
    /// Number of cache hits
    pub hit_count: u64,
    /// Number of cache misses
    pub miss_count: u64,
}
