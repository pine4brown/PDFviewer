//! LRU page cache.
//!
//! Because `Pdfium` is not `Send`, we cannot cache rendered images across
//! command invocations that run on different threads using the same `Pdfium`
//! instance.  Instead the cache stores the already-rendered **base64 PNG
//! strings** which are plain `String`s and are fully `Send + Sync`.

use lru::LruCache;
use std::num::NonZeroUsize;

/// Maximum number of rendered pages to keep in the cache.
const MAX_CACHE_SIZE: usize = 50;

/// Rounding precision for zoom levels (centiles: 2 decimal places).
const ZOOM_PRECISION: i32 = 2;

/// Cache key combining a page index and a normalised zoom level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub page_index: u16,
    /// Zoom × 10^ZOOM_PRECISION, rounded to integer.
    pub zoom_centile: i32,
}

impl CacheKey {
    pub fn new(page_index: u16, zoom: f32) -> Self {
        let factor = 10_f32.powi(ZOOM_PRECISION);
        Self {
            page_index,
            zoom_centile: (zoom * factor).round() as i32,
        }
    }
}

/// LRU cache of rendered page images (base64 PNG strings).
///
/// This type is `Send + Sync` because it only holds `String`s.
pub struct PageCache {
    inner: LruCache<CacheKey, String>,
}

impl PageCache {
    pub fn new() -> Self {
        Self {
            inner: LruCache::new(
                NonZeroUsize::new(MAX_CACHE_SIZE).expect("cache size > 0"),
            ),
        }
    }

    /// Return a cached entry, or `None` if the key is not present.
    pub fn get(&mut self, page_index: u16, zoom: f32) -> Option<&String> {
        self.inner.get(&CacheKey::new(page_index, zoom))
    }

    /// Insert a rendered page into the cache.
    pub fn put(&mut self, page_index: u16, zoom: f32, data: String) {
        self.inner.put(CacheKey::new(page_index, zoom), data);
    }

    /// Remove all cached entries.
    pub fn invalidate_all(&mut self) {
        self.inner.clear();
    }

    /// Check whether a key is present without promoting it in the LRU order.
    pub fn contains(&self, page_index: u16, zoom: f32) -> bool {
        self.inner.contains(&CacheKey::new(page_index, zoom))
    }
}

impl Default for PageCache {
    fn default() -> Self {
        Self::new()
    }
}
