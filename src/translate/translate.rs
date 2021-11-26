//! DeepL translation API wrapper.

use lingua::Language;
use reqwest;
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use std::{collections::HashMap, env};
use uuid::Uuid;

const DEEPL_API_URL: &'static str = "https://api-free.deepl.com/v2/translate";
const AZURE_API_URL: &'static str = "https://api.cognitive.microsofttranslator.com/translate";

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

/// A list of results from the Azure API.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct AzureResults {
    translations: Vec<AzureTranslationResult>,
}

impl AzureResults {
    /// Outputs a message containing all of the inner elements' contents.
    pub(crate) fn message(&self) -> String {
        let msgs: Vec<String> = self.translations.iter().map(|res| res.message()).collect();
        msgs.join("\n\n")
    }
}

/// A single translation returned from DeepL's API.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct AzureTranslationResult {
    /// The detected source language, as a string code. (The ISO code enums used in `lingua` don't
    /// implement Serde traits I'm using.)
    to: String,

    /// The translated text.
    text: String,
}

impl AzureTranslationResult {
    /// Outputs the message content that Nano should reply with.
    pub(crate) fn message(&self) -> String {
        let src_iso = self.to.parse();
        let src = src_iso
            .and_then(|code| Ok(Language::from_iso_code_639_1(&code)))
            .and_then(|lang| Ok(format!("{:?}", lang)))
            .unwrap_or("<unknown (report as bug!)>".to_string());

        format!("Translation:\n{}", self.text)
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct AzureTranslationRequest<'a> {
    /// The text to translate;
    text: &'a str,
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

    let mut params: Vec<(&str, &str)> = vec![];

    let src_code;
    if let Some(src_lang) = source {
        src_code = src_lang.iso_code_639_1().to_string().to_ascii_lowercase();
        params.push(("from", &src_code));
    }

    let target_code = target.iso_code_639_1().to_string().to_ascii_lowercase();
    params.push(("to", &target_code));
    params.push(("api-version", "3.0"));

    let body = vec![AzureTranslationRequest { text: msg }];

    let uuid = Uuid::new_v4().to_string();
    let api_key = env::var("AZURE_KEY").unwrap_or("bad".to_string());
    let client = client
        .post(AZURE_API_URL)
        .header("Ocp-Apim-Subscription-Key", &api_key)
        .header("Ocp-Apim-Subscription-Region", "global")
        .header("Content-type", "application/json")
        .header("X-ClientTraceId", uuid)
        .json(&body)
        .query(&params);

    let resp = client
        .send()
        .await
        .and_then(|resp_or_err| resp_or_err.error_for_status());

    match resp {
        Ok(resp) => {
            let result = resp.json::<Vec<AzureResults>>().await;
            result.and_then(|res| Ok(res[0].message()))
        }
        Err(e) => {
            dbg!(&e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translate() {
        assert_eq!(
            translate("hello world", None, Language::Spanish)
                .await
                .unwrap(),
            "hola mundo"
        );
    }
}
