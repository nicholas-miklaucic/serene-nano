//! This module defines the languages that Nano can translate into and from. The
//! limiter here is the problem that Discord only supports 25 options, so if I
//! want autocomplete in the slash command I can only pick 25.

use lingua::Language;

// https://www.deepl.com/docs-api/translating-text/example/
pub(crate) const AVAILABLE_LANGS: [Language; 25] = [
    Language::Bulgarian,
    Language::Czech,
    Language::Danish,
    Language::German,
    Language::Greek,
    Language::English,
    Language::Spanish,
    Language::Estonian,
    Language::Finnish,
    Language::French,
    Language::Hindi,
    Language::Hungarian,
    Language::Italian,
    Language::Japanese,
    Language::Lithuanian,
    Language::Latvian,
    Language::Dutch,
    Language::Polish,
    Language::Portuguese,
    Language::Romanian,
    Language::Russian,
    Language::Slovak,
    Language::Slovene,
    Language::Swedish,
    Language::Chinese,
];

/// Gets the name of the language.
pub(crate) fn lang_name(lang: &Language) -> String {
    format!("{:?}", lang)
}

/// Gets the available language names.
pub(crate) fn available_lang_names() -> Vec<String> {
    AVAILABLE_LANGS.iter().map(lang_name).collect()
}

/// Gets the language corresponding to the given name, failing if no such name exists.
pub(crate) fn get_language(name: &str) -> Option<Language> {
    for (lang, lang_name) in AVAILABLE_LANGS.iter().zip(available_lang_names().iter()) {
        if name == lang_name {
            return Some(lang.clone());
        }
    }
    None
}
