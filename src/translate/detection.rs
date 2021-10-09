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
    }

    // emojis and math can trip it up: if detected, don't translate
    if msg.chars().filter(|c| c.is_numeric()).count() >= 10 {
        None
    } else if conf_vals.get(&Language::English).unwrap_or(&0.0) <= &0.75 && msg.len() >= 30 {
        detector.detect_language_of(msg)
    } else {
        None
    }
}
