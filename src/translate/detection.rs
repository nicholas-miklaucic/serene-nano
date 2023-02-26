//! Language detection, with an eye towards identifying non-English text to translate while avoiding
//! spurious translation of slang or other common issues on Discord.

use super::available_langs::AVAILABLE_LANGS;
use std::collections::HashMap;

use lingua::Language;
use lingua::LanguageDetectorBuilder;

/// Given message text, returns an Option indicating whether a language could be detected with
/// sufficient certainty and, if so, what language was detected.
pub(crate) fn detect_language(msg: &str) -> Option<Language> {
    let detector = LanguageDetectorBuilder::from_languages(&AVAILABLE_LANGS)
        .with_minimum_relative_distance(0.1)
        .build();

    let conf_vals: HashMap<Language, f64> = detector
        .compute_language_confidence_values(msg)
        .into_iter()
        .collect();

    if msg.starts_with("---") {
        dbg!(msg.clone());
        dbg!(conf_vals.clone());
        dbg!(msg.chars().filter(|c| c.is_numeric()).count());
        dbg!(msg.chars().count());
        dbg!(conf_vals.get(&Language::English).unwrap_or(&0.0));
        dbg!(msg.len() >= 30);
    }

    // emojis and math can trip it up: if heavily numeric
    if (msg.chars().filter(|c| c.is_numeric()).count() as f64) / (msg.chars().count() as f64) >= 0.3
    {
        None
    } else if conf_vals.get(&Language::English).unwrap_or(&0.0) <= &0.75 && msg.len() >= 30 {
        detector.detect_language_of(msg)
    } else {
        None
    }
}
