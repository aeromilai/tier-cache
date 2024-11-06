use std::time::Duration;
use tiered_cache::{AutoCache, CacheConfig, TierConfig};

const MB: usize = 1024 * 1024; // 1 megabyte
const GB: usize = 1024 * MB;   // 1 gigabyte

fn main() {
    // Configure a 1GB cache with three tiers:
    // - Small tier: 200MB for items 0-64KB
    // - Medium tier: 300MB for items 64KB-1MB
    // - Large tier: 500MB for items 1MB-10MB
    let config = CacheConfig {
        tiers: vec![
            TierConfig {
                total_capacity: 200 * MB,
                size_range: (0, 64 * 1024),  // 0-64KB
            },
            TierConfig {
                total_capacity: 300 * MB,
                size_range: (64 * 1024, MB), // 64KB-1MB
            },
            TierConfig {
                total_capacity: 500 * MB,
                size_range: (MB, 10 * MB),   // 1MB-10MB
            },
        ],
        update_channel_size: 1024,
    };

    let cache = AutoCache::<String, Vec<u8>>::new(config);

    // Example usage
    let key = "example".to_string();
    let value = vec![0u8; 500 * 1024]; // 500KB value
    
    cache.put(key.clone(), value);
    
    if let Some(retrieved) = cache.get(&key) {
        println!("Retrieved value of size: {} bytes", retrieved.len());
    }

    // Print cache statistics
    let stats = cache.stats();
    println!("Cache statistics:");
    println!("Total items: {}", stats.total_items);
    println!("Total size: {} MB", stats.total_size / MB);
    
    for (i, tier) in stats.tier_stats.iter().enumerate() {
        println!("\nTier {}:", i);
        println!("  Items: {}", tier.items);
        println!("  Size: {} MB", tier.size / MB);
        println!("  Capacity: {} MB", tier.capacity / MB);
    }
}
