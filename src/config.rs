//! Module for global bot configuration.

/// The number of seconds required between thanking people.
pub(crate) const THANK_COOLDOWN: usize = 30;
/// The URL of the Redis server.
pub(crate) const REDIS_URL: &str = "redis://127.0.0.1:1234/";
/// The opening delimiter that indicate Typst math code to render as such.
pub(crate) const TYPST_OPEN_DELIM: &str = r"<.";
/// The closing delimiter that indicate Typst math code to render as such.
pub(crate) const TYPST_CLOSE_DELIM: &str = r".>";
