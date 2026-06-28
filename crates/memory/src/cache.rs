//! LLM Cache — moka-based LRU cache for LLM responses.
//!
//! Avoids redundant API calls by caching identical prompts.
//! Configurable TTL and max size.

use std::sync::atomic::{AtomicU64, Ordering};

/// Cache key: hash of (model, messages, temperature).
pub type CacheKey = u64;

/// Cached LLM response.
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_at: std::time::Instant,
}

/// LLM response cache with hit/miss tracking.
pub struct LLMCache {
    cache: moka::sync::Cache<CacheKey, CachedResponse>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl LLMCache {
    /// Create a new cache with max entries and TTL.
    pub fn new(max_entries: usize, ttl_seconds: u64) -> Self {
        let cache = moka::sync::Cache::builder()
            .max_capacity(max_entries as u64)
            .time_to_live(std::time::Duration::from_secs(ttl_seconds))
            .build();

        Self {
            cache,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Create a cache with default settings (1000 entries, 5min TTL).
    pub fn default_cache() -> Self {
        Self::new(1000, 300)
    }

    /// Generate a cache key from prompt parameters.
    pub fn key(model: &str, messages: &[String], temperature: f32) -> CacheKey {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        model.hash(&mut hasher);
        for msg in messages {
            msg.hash(&mut hasher);
        }
        temperature.to_bits().hash(&mut hasher);
        hasher.finish()
    }

    /// Get a cached response.
    pub fn get(&self, key: &CacheKey) -> Option<CachedResponse> {
        match self.cache.get(key) {
            Some(response) => {
                self.hits.fetch_add(1, Ordering::SeqCst);
                tracing::debug!("LLM cache hit for key {:x}", key);
                Some(response)
            }
            None => {
                self.misses.fetch_add(1, Ordering::SeqCst);
                None
            }
        }
    }

    /// Store a response in the cache.
    pub fn insert(&self, key: CacheKey, response: CachedResponse) {
        self.cache.insert(key, response);
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::SeqCst);
        let misses = self.misses.load(Ordering::SeqCst);
        let total = hits + misses;

        CacheStats {
            hits,
            misses,
            total_requests: total,
            hit_rate: if total > 0 {
                hits as f64 / total as f64
            } else {
                0.0
            },
            size: self.cache.entry_count(),
            memory_usage: self.cache.weighted_size(),
        }
    }

    /// Reset counters.
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::SeqCst);
        self.misses.store(0, Ordering::SeqCst);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }
}

/// Cache statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_requests: u64,
    pub hit_rate: f64,
    pub size: u64,
    pub memory_usage: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_and_get() {
        let cache = LLMCache::new(10, 60);
        let key = LLMCache::key("gpt-5", &["hello".to_string()], 0.3);

        let response = CachedResponse {
            content: "Hi there!".to_string(),
            model: "gpt-5".to_string(),
            input_tokens: 5,
            output_tokens: 3,
            cached_at: std::time::Instant::now(),
        };

        cache.insert(key, response.clone());
        let cached = cache.get(&key).unwrap();
        assert_eq!(cached.content, "Hi there!");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_miss() {
        let cache = LLMCache::new(10, 60);
        let key = LLMCache::key("gpt-5", &["hello".to_string()], 0.3);

        let result = cache.get(&key);
        assert!(result.is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_key_uniqueness() {
        let k1 = LLMCache::key("gpt-5", &["hello".to_string()], 0.3);
        let k2 = LLMCache::key("gpt-5", &["hello".to_string()], 0.3);
        let k3 = LLMCache::key("gpt-5", &["hello".to_string()], 0.5);
        let k4 = LLMCache::key("gpt-4", &["hello".to_string()], 0.3);

        assert_eq!(k1, k2); // Same params = same key
        assert_ne!(k1, k3); // Different temperature = different key
        assert_ne!(k1, k4); // Different model = different key
    }

    #[test]
    fn test_cache_clear() {
        let cache = LLMCache::new(10, 60);
        let key = LLMCache::key("gpt-5", &["hello".to_string()], 0.3);

        cache.insert(key, CachedResponse {
            content: "Hi".to_string(),
            model: "gpt-5".to_string(),
            input_tokens: 5,
            output_tokens: 3,
            cached_at: std::time::Instant::now(),
        });

        cache.clear();
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_hit_rate() {
        let cache = LLMCache::new(10, 60);
        let key = LLMCache::key("gpt-5", &["test".to_string()], 0.3);

        // 3 misses
        cache.get(&key);
        cache.get(&key);
        cache.get(&key);

        // 1 hit
        cache.insert(key, CachedResponse {
            content: "ok".to_string(),
            model: "gpt-5".to_string(),
            input_tokens: 1,
            output_tokens: 1,
            cached_at: std::time::Instant::now(),
        });
        cache.get(&key);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 3);
        assert!((stats.hit_rate - 0.25).abs() < 0.01);
    }
}