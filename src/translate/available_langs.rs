//! This module defines the languages that Nano can translate into and from. The
//! limiter here is the problem that Discord only supports 25 options, so if I
//! want autocomplete in the slash command I can only pick 25.

use deepl_openapi::models::{source_language::SourceLanguage, TargetLanguage};
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
    Language::Hungarian,
    Language::Indonesian,
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

/// Converts to DeepL SourceLanguage
pub(crate) fn lingua_to_deepl_source(lang: Language) -> SourceLanguage {
    match lang {
        Language::Bulgarian => SourceLanguage::Bg,
        Language::Chinese => SourceLanguage::Zh,
        Language::Czech => SourceLanguage::Cs,
        Language::Danish => SourceLanguage::Da,
        Language::Dutch => SourceLanguage::Nl,
        Language::English => SourceLanguage::En,
        Language::Estonian => SourceLanguage::Et,
        Language::Finnish => SourceLanguage::Fi,
        Language::French => SourceLanguage::Fr,
        Language::German => SourceLanguage::De,
        Language::Greek => SourceLanguage::El,
        Language::Hungarian => SourceLanguage::Hu,
        Language::Indonesian => SourceLanguage::Id,
        Language::Italian => SourceLanguage::It,
        Language::Japanese => SourceLanguage::Ja,
        Language::Latvian => SourceLanguage::Lv,
        Language::Lithuanian => SourceLanguage::Lt,
        Language::Polish => SourceLanguage::Pl,
        Language::Portuguese => SourceLanguage::Pt,
        Language::Romanian => SourceLanguage::Ro,
        Language::Russian => SourceLanguage::Ru,
        Language::Slovak => SourceLanguage::Sk,
        Language::Slovene => SourceLanguage::Sl,
        Language::Spanish => SourceLanguage::Es,
        Language::Swedish => SourceLanguage::Sv,
        Language::Ukrainian => SourceLanguage::Uk,
        _ => SourceLanguage::En,
    }
}

/// Converts to DeepL TargetLanguage
pub(crate) fn lingua_to_deepl_target(lang: Language) -> TargetLanguage {
    match lang {
        Language::Bulgarian => TargetLanguage::Bg,
        Language::Chinese => TargetLanguage::Zh,
        Language::Czech => TargetLanguage::Cs,
        Language::Danish => TargetLanguage::Da,
        Language::Dutch => TargetLanguage::Nl,
        Language::English => TargetLanguage::En,
        Language::Estonian => TargetLanguage::Et,
        Language::Finnish => TargetLanguage::Fi,
        Language::French => TargetLanguage::Fr,
        Language::German => TargetLanguage::De,
        Language::Greek => TargetLanguage::El,
        Language::Hungarian => TargetLanguage::Hu,
        Language::Indonesian => TargetLanguage::Id,
        Language::Italian => TargetLanguage::It,
        Language::Japanese => TargetLanguage::Ja,
        Language::Latvian => TargetLanguage::Lv,
        Language::Lithuanian => TargetLanguage::Lt,
        Language::Polish => TargetLanguage::Pl,
        Language::Portuguese => TargetLanguage::Pt,
        Language::Romanian => TargetLanguage::Ro,
        Language::Russian => TargetLanguage::Ru,
        Language::Slovak => TargetLanguage::Sk,
        Language::Slovene => TargetLanguage::Sl,
        Language::Spanish => TargetLanguage::Es,
        Language::Swedish => TargetLanguage::Sv,
        Language::Ukrainian => TargetLanguage::Uk,
        _ => TargetLanguage::En,
    }
}
