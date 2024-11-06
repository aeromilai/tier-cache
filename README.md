# tiered-cache

[![Crates.io](https://img.shields.io/crates/v/tiered-cache.svg)](https://crates.io/crates/tiered-cache)
[![Documentation](https://docs.rs/tiered-cache/badge.svg)](https://docs.rs/tiered-cache)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

A high-performance multi-tiered cache implementation in Rust with automatic sizing and async support.

## Features

- 🚀 Multiple cache tiers with automatic item placement based on size
- ⚡ Async support with Tokio
- 🔄 LRU eviction policy
- 📊 Detailed statistics for monitoring
- 🔍 Efficient lookup with DashMap
- 🛡️ Thread-safe design
- 📦 Zero unsafe code

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tiered-cache = "0.1.1"
```

## Quick Start

```rust
use tiered_cache::{AutoCache, CacheConfig, TierConfig};
use std::time::Duration;

const MB: usize = 1024 * 1024;

// Configure a cache with two tiers
let config = CacheConfig {
    tiers: vec![
        TierConfig {
            total_capacity: 100 * MB,    // 100MB
            size_range: (0, 64 * 1024),  // 0-64KB
        },
        TierConfig {
            total_capacity: 900 * MB,    // 900MB
            size_range: (64 * 1024, MB), // 64KB-1MB
        },
    ],
    default_ttl: Some(Duration::from_secs(3600)),
    update_channel_size: 1024,
};

// Create the cache
let cache = AutoCache::<String, Vec<u8>>::new(config);

// Basic operations
cache.put("key1".to_string(), vec![0; 1024]);
if let Some(value) = cache.get(&"key1".to_string()) {
    println!("Retrieved value of size: {}", value.len());
}

// Async get_or_update
let value = cache.get_or_update("key2".to_string(), async {
    // Fetch or compute value asynchronously
    Some(vec![0; 2048])
}).await;
```

## Cache Configuration

The cache is configured using tiers, where each tier has:
- A total capacity in bytes
- A size range for entries (min_size, max_size)

Items are automatically placed in the appropriate tier based on their size. The cache uses the `HeapSize` trait to accurately measure memory usage.

## Performance

The implementation uses:
- `DashMap` for concurrent key-to-tier mapping
- `parking_lot` locks for low contention
- `SmallVec` for efficient tier storage
- Cache-padded structures to prevent false sharing

## Examples

See the [examples](examples/) directory for more usage examples, including:
- Setting up a 1GB cache with multiple tiers
- Handling different value sizes
- Monitoring cache statistics

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
