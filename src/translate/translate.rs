//! DeepL translation API wrapper.

use lingua::Language;
use reqwest;
use serde::{Deserialize, Serialize};
use std::env;

const DEEPL_API_URL: &'static str = "https://api-free.deepl.com/v2/translate";

/// A single translation returned from DeepL's API.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct DeeplTranslationResult {
    /// The detected source language, as a string code. (The ISO code enums used in `lingua` don't
    /// implement Serde traits I'm using.)
    detected_source_language: String,

    /// The translated text.
    text: String,
}

impl DeeplTranslationResult {
    /// Outputs the message content that Nano should reply with.
    pub(crate) fn message(&self) -> String {
        let src_iso = self.detected_source_language.parse();
        let src = src_iso
            .and_then(|code| Ok(Language::from_iso_code_639_1(&code)))
            .and_then(|lang| Ok(format!("{:?}", lang)))
            .unwrap_or("<unknown (report as bug!)>".to_string());

        format!("Translated from {}:\n{}", src, self.text)
    }
}

/// A list of results from the DeepL API.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct DeeplResults {
    translations: Vec<DeeplTranslationResult>,
}

impl DeeplResults {
    /// Outputs a message containing all of the inner elements' contents.
    pub(crate) fn message(&self) -> String {
        let msgs: Vec<String> = self.translations.iter().map(|res| res.message()).collect();
        msgs.join("\n\n")
    }
}

/// Translate a message from the given source language (or None, to autodetect) to the given target
/// language. Returns an error if DeepL cannot be reached or an error response is returned.
pub(crate) async fn translate(
    msg: &str,
    source: Option<Language>,
    target: Language,
) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    // setting this to an invalid key will trigger a request error which saves me having to make a
    // custom error type here
    let api_key = env::var("DEEPL_KEY").unwrap_or("bad".to_string());
    let mut params: Vec<(&str, &str)> = vec![];
    params.push(("auth_key", &api_key));

    let src_code;
    if let Some(src_lang) = source {
        src_code = src_lang.iso_code_639_1().to_string().to_ascii_lowercase();
        params.push(("source_lang", &src_code));
    }

    let target_code = target.iso_code_639_1().to_string().to_ascii_lowercase();
    params.push(("target_lang", &target_code));
    params.push(("text", msg));
    params.push(("preserve_formatting", "1"));

    let resp = client
        .get(DEEPL_API_URL)
        .query(&params)
        .send()
        .await
        .and_then(|resp_or_err| resp_or_err.error_for_status());

    match resp {
        Ok(resp) => {
            let result = resp.json::<DeeplResults>().await;
            result.and_then(|res| Ok(res.message()))
        }
        Err(e) => Err(e),
    }
}
