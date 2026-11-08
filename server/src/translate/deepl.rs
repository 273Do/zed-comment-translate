//! DeepL API (v2), with Free (`:fx` keys) and Pro endpoints.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::{percent_encode, Translated};

/// Where to sign up for the API and get a key.
pub const API_PAGE: &str = "https://www.deepl.com/pro-api";

pub struct Deepl {
    api_key: String,
}

impl Deepl {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    fn endpoint(&self) -> &'static str {
        if self.api_key.ends_with(":fx") {
            "https://api-free.deepl.com/v2/translate"
        } else {
            "https://api.deepl.com/v2/translate"
        }
    }

    pub(super) async fn translate(
        &self,
        client: &reqwest::Client,
        from: &str,
        to: &str,
        texts: &[String],
    ) -> Result<Vec<Translated>> {
        let mut request = json!({
            "text": texts,
            "target_lang": target_lang(to),
            // The texts are escaped HTML; keep the `<code>` placeholders untouched.
            "tag_handling": "xml",
            "ignore_tags": ["code"],
        });
        if let Some(source) = source_lang(from) {
            request["source_lang"] = source.into();
        }

        let response = client
            .post(self.endpoint())
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("DeepL returned {status}: {body}");
        }

        let response: Value = serde_json::from_str(&body).context("invalid response JSON")?;
        response["translations"]
            .as_array()
            .context("unexpected response shape")?
            .iter()
            .map(|item| {
                Ok(Translated {
                    text: item["text"]
                        .as_str()
                        .context("unexpected response item")?
                        .to_owned(),
                    detected: item["detected_source_language"].as_str().map(str::to_owned),
                })
            })
            .collect()
    }
}

/// DeepL source languages have no region, and auto-detection is the default.
fn source_lang(language: &str) -> Option<String> {
    (language != "auto").then(|| primary(language).to_ascii_uppercase())
}

/// Maps Google-style codes to DeepL target codes ("en" → "EN-US", "zh-TW" → "ZH-HANT").
fn target_lang(language: &str) -> String {
    match language.to_ascii_lowercase().as_str() {
        "en" => "EN-US".into(),
        "pt" => "PT-BR".into(),
        "zh" | "zh-cn" => "ZH-HANS".into(),
        "zh-tw" => "ZH-HANT".into(),
        other => other.to_ascii_uppercase(),
    }
}

fn primary(language: &str) -> &str {
    language.split('-').next().unwrap_or(language)
}

/// The web translator has no "auto" source in its URL, so English is assumed.
pub(super) fn page_url(from: &str, to: &str, text: &str) -> String {
    let from = if from == "auto" { "en" } else { primary(from) };
    format!(
        "https://www.deepl.com/translator#{}/{}/{}",
        percent_encode(&from.to_ascii_lowercase()),
        percent_encode(&primary(to).to_ascii_lowercase()),
        percent_encode(text)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_languages() {
        assert_eq!(target_lang("ja"), "JA");
        assert_eq!(target_lang("en"), "EN-US");
        assert_eq!(target_lang("zh-TW"), "ZH-HANT");
        assert_eq!(source_lang("auto"), None);
        assert_eq!(source_lang("en-US").as_deref(), Some("EN"));
    }

    #[test]
    fn picks_endpoint_from_key() {
        assert!(Deepl::new("abc:fx".into()).endpoint().contains("api-free"));
        assert!(!Deepl::new("abc".into()).endpoint().contains("api-free"));
    }

    #[test]
    fn builds_page_url() {
        assert_eq!(
            page_url("auto", "ja", "a/b"),
            "https://www.deepl.com/translator#en/ja/a%2Fb"
        );
    }
}
