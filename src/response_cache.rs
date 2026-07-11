use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct CacheEntry {
    at: Instant,
    value: Value,
}

static ARCHIVE_CACHE: Mutex<Option<HashMap<String, CacheEntry>>> = Mutex::new(None);

fn cache_map() -> std::sync::MutexGuard<'static, Option<HashMap<String, CacheEntry>>> {
    ARCHIVE_CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn get(key: &str, ttl: Duration) -> Option<Value> {
    let guard = cache_map();
    let map = guard.as_ref()?;
    let entry = map.get(key)?;
    if entry.at.elapsed() > ttl {
        return None;
    }
    Some(entry.value.clone())
}

pub fn set(key: String, value: Value) {
    let mut guard = cache_map();
    let map = guard.get_or_insert_with(HashMap::new);
    if map.len() > 128 {
        let stale: Vec<String> = map
            .iter()
            .filter(|(_, entry)| entry.at.elapsed() > Duration::from_secs(60 * 60))
            .map(|(k, _)| k.clone())
            .collect();
        for item in stale {
            map.remove(&item);
        }
        if map.len() > 128 {
            map.clear();
        }
    }
    map.insert(
        key,
        CacheEntry {
            at: Instant::now(),
            value,
        },
    );
}
