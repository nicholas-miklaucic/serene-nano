//! DeepL translation API wrapper.

use deepl_openapi::{
    apis::{
        configuration::{ApiKey, Configuration},
        translate_text_api::translate_text,
    },
    models::TranslateText200ResponseTranslationsInner,
};
use lingua::Language;
use reqwest;
use serde::{de::IntoDeserializer, Deserialize, Serialize};
use std::{collections::HashMap, env};
use uuid::Uuid;

use crate::translate::available_langs::{lingua_to_deepl_source, lingua_to_deepl_target};

/// Translate a message from the given source language (or None, to autodetect) to the given target
/// language. Returns an error if DeepL cannot be reached or an error response is returned.
pub(crate) async fn translate(
    msg: &str,
    source: Option<Language>,
    target: Language,
) -> Option<String> {
    let client = reqwest::Client::new();

    // setting this to an invalid key will trigger a request error which saves me having to make a
    // custom error type here
    let api_key = env::var("DEEPL_KEY").unwrap_or("bad".to_string());

    let config = Configuration {
        base_path: "https://api-free.deepl.com/v2".to_owned(),
        user_agent: Some("OpenAPI-Generator/2.7.0/rust".to_owned()),
        client,
        basic_auth: None,
        oauth_access_token: None,
        bearer_access_token: None,
        api_key: Some(ApiKey {
            prefix: Some("DeepL-Auth-Key".to_string()),
            key: api_key,
        }),
    };

    let result = translate_text(
        &config,
        vec![msg.to_string()],
        lingua_to_deepl_target(target),
        source.clone().map(lingua_to_deepl_source),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .ok()
    .and_then(|r| r.translations.and_then(|v| v.get(0).cloned()));

    result.map(|res| match res.detected_source_language {
        Some(src) => {
            if source.is_none() {
                format!(
                    "Translated from {}:\n{}",
                    src.to_string(),
                    res.text.unwrap_or("".to_string())
                )
            } else {
                format!("{}", res.text.unwrap_or("".to_string()))
            }
        }
        None => format!("{}", res.text.unwrap_or("".to_string())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_translate() {
        assert_eq!(
            translate("hello world", Some(Language::English), Language::Spanish)
                .await
                .unwrap(),
            "hola mundo".to_string()
        );
    }
}
