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
