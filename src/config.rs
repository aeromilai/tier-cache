use serde::Deserialize;

/// Configuration for the tiered cache system
#[derive(Clone, Debug, Deserialize)]
pub struct CacheConfig {
    /// Vector of tier configurations, ordered from smallest to largest size
    pub tiers: Vec<TierConfig>,
    /// Size of the channel used for cache update notifications
    pub update_channel_size: Option<usize>,
}

/// Configuration for a single cache tier
#[derive(Clone, Debug, Deserialize)]
pub struct TierConfig {
    /// Maximum total capacity of the tier in bytes
    pub total_capacity: usize,
    /// Valid size range for entries in this tier as (`min_size`, `max_size`) in bytes
    pub size_range: (usize, usize),
}
