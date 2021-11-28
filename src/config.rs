//! Module for global bot configuration.

/// The number of seconds required between thanking people.
pub(crate) const THANK_COOLDOWN: usize = 30;
/// The URL of the Redis server.
pub(crate) const REDIS_URL: &'static str = "redis://127.0.0.1:1234/";
