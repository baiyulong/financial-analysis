use std::collections::HashMap;
use std::time::{Duration, Instant};

struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

pub struct InMemoryCache<T: Clone> {
    entries: HashMap<String, CacheEntry<T>>,
    ttl: Duration,
}

impl<T: Clone> InMemoryCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        self.entries.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: impl Into<String>, value: T) {
        self.entries.insert(
            key.into(),
            CacheEntry {
                value,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub fn invalidate(&mut self, key: &str) {
        self.entries.remove(key);
    }

    pub fn clear_expired(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_cache_hit() {
        let mut cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("AAPL", "price_data".to_string());
        assert_eq!(cache.get("AAPL"), Some("price_data".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = InMemoryCache::<String>::new(Duration::from_secs(60));
        assert_eq!(cache.get("TSLA"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let mut cache = InMemoryCache::new(Duration::from_millis(50));
        cache.set("BTC", "value".to_string());
        sleep(Duration::from_millis(100));
        assert_eq!(cache.get("BTC"), None);
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("AAPL", "v1".to_string());
        cache.set("AAPL", "v2".to_string());
        assert_eq!(cache.get("AAPL"), Some("v2".to_string()));
    }
}
