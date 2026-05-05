use std::time::Duration;
use std::sync::Arc;
use crate::flush::FlushSink;

/// Configuration for the Token Enforcer.
///
/// Use `Config::default()` and modify fields as needed, or use a builder pattern approach.
pub struct Config {
    /// Redis connection URL. Default: "redis://127.0.0.1:6379"
    pub redis_url: String,

    /// How often the background task flushes usage events to the sink.
    /// Default: 30 seconds
    pub flush_interval: Duration,

    /// Redis key prefix to namespace all keys.
    /// Default: "te" (keys will look like "te:budget:tenant_123")
    pub key_prefix: String,

    /// Maximum Redis connection pool size.
    /// Default: 10
    pub pool_size: usize,

    /// Optional sink for persisting usage events outside Redis.
    /// If None, events are collected in memory until the flush interval, then discarded.
    pub sink: Option<Arc<dyn FlushSink>>,
}

impl Default for Config {
    /// Creates a new `Config` with sensible defaults.
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".to_string(),
            flush_interval: Duration::from_secs(30),
            key_prefix: "te".to_string(),
            pool_size: 10,
            sink: None,
        }
    }
}

impl Config {
    /// Helper to set the Redis URL.
    pub fn with_redis_url(mut self, url: impl Into<String>) -> Self {
        self.redis_url = url.into();
        self
    }

    /// Helper to set the flush interval.
    pub fn with_flush_interval(mut self, interval: Duration) -> Self {
        self.flush_interval = interval;
        self
    }

    /// Helper to set the Redis key prefix.
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = prefix.into();
        self
    }

    /// Helper to set the Redis pool size.
    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    /// Helper to set the flush sink.
    pub fn with_sink(mut self, sink: Arc<dyn FlushSink>) -> Self {
        self.sink = Some(sink);
        self
    }
}
