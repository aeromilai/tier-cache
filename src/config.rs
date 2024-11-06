use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct CacheConfig {
    pub tiers: Vec<TierConfig>,
    pub default_ttl: Option<std::time::Duration>,
    pub update_channel_size: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TierConfig {
    pub total_capacity: usize,
    pub size_range: (usize, usize),
}
