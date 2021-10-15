//! This module defines the languages that Nano can translate into and from. This is a simplified
//! version of the true support of DeepL: only outer languages are listed, not the dialects that
//! DeepL supports in output translations. This means that we can reuse the same list of languages
//! everywhere, instead of having one for input and one for output (e.g., EN-US in one place and EN
//! in another).

use lingua::Language;

// https://www.deepl.com/docs-api/translating-text/example/
pub(crate) const AVAILABLE_LANGS: [Language; 24] = [
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
