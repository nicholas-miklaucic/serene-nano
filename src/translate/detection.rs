//! Language detection, with an eye towards identifying non-English text to translate while avoiding
//! spurious translation of slang or other common issues on Discord.

use super::available_langs::AVAILABLE_LANGS;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

use lingua::Language;
use lingua::LanguageDetectorBuilder;
use regex::Regex;
use regex::RegexBuilder;

const URL_PATTERN: &str =
    r#"((https?|ftp|smtp):\/\/)?(www.)?[a-z0-9]+\.[a-z]+(\/[a-zA-Z0-9#?=]+\/?)*"#;

const EMOJI_PATTERN: &str = r#"<a?:\w+:\d+>"#;

static URL_RE: OnceLock<Regex> = OnceLock::new();
static EMOJI_RE: OnceLock<Regex> = OnceLock::new();

/// Filters out URLs, emojis, and mentions from text to make it easier to detect language.
fn filter_for_language_detection(msg: &str) -> String {
    let url_re = URL_RE.get_or_init(|| {
        RegexBuilder::new(URL_PATTERN)
            .case_insensitive(true)
            .build()
            .unwrap()
    });

    let emoji_re = EMOJI_RE.get_or_init(|| {
        RegexBuilder::new(EMOJI_PATTERN)
            .case_insensitive(true)
            .build()
            .unwrap()
    });

    emoji_re
        .replace_all(&url_re.replace_all(msg, ""), "")
        .to_string()
}

/// Given message text, returns an Option indicating whether a language could be detected with
/// sufficient certainty and, if so, what language was detected.
pub(crate) fn detect_language(msg: &str) -> Option<Language> {
    let detector = LanguageDetectorBuilder::from_languages(&AVAILABLE_LANGS)
        .with_minimum_relative_distance(0.1)
        .build();

    let filtered = filter_for_language_detection(msg);

    let conf_vals: HashMap<Language, f64> = detector
        .compute_language_confidence_values(&filtered)
        .into_iter()
        .collect();

    if msg.starts_with("---") {
        dbg!(msg);
        dbg!(&filtered);
        dbg!(conf_vals.clone());
        dbg!((&filtered).chars().filter(|c| c.is_numeric()).count());
        dbg!(&filtered.chars().count());
        dbg!(conf_vals.get(&Language::English).unwrap_or(&0.0));
        dbg!(filtered.len() >= 30);
    }

    let best_lang = conf_vals
        .iter()
        .max_by(|&(_k1, v1), &(_k2, v2)| {
            if (v1 - v2).is_sign_negative() {
                Ordering::Less
            } else if (v2 - v1).is_sign_negative() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .unwrap();

    // math can trip it up: if heavily numeric, don't return anything
    if (filtered.chars().filter(|c| c.is_numeric()).count() as f64)
        / (filtered.chars().count() as f64)
        >= 0.3
    {
        None
    } else if conf_vals.get(&Language::English).unwrap_or(&0.0) * 5.0 <= *best_lang.1
        && filtered.len() > 30
    {
        Some(*best_lang.0)
    } else {
        None
    }
}
