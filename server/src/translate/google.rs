//! Google Cloud Translation API (Basic, v2), authenticated with an API key.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::{percent_encode, Translated};

const ENDPOINT: &str = "https://translation.googleapis.com/language/translate/v2";
/// Where to enable the API and create a key.
pub const API_PAGE: &str = "https://console.cloud.google.com/apis/library/translate.googleapis.com";

pub struct Google {
    api_key: String,
}

impl Google {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub(super) async fn translate(
        &self,
        client: &reqwest::Client,
        from: &str,
        to: &str,
        texts: &[String],
    ) -> Result<Vec<Translated>> {
        let mut request = json!({
            "q": texts,
            "target": to,
            // The texts are escaped HTML; `<code>` placeholders are left untouched.
            "format": "html",
        });
        // Without a source, the language is detected.
        if from != "auto" {
            request["source"] = from.into();
        }

        // The key goes in a header rather than the URL, so that it is not logged.
        let response = client
            .post(ENDPOINT)
            .header("X-Goog-Api-Key", &self.api_key)
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            bail!("Google returned {status}: {body}");
        }

        let response: Value = serde_json::from_str(&body).context("invalid response JSON")?;
        response["data"]["translations"]
            .as_array()
            .context("unexpected response shape")?
            .iter()
            .map(|item| {
                Ok(Translated {
                    text: item["translatedText"]
                        .as_str()
                        .context("unexpected response item")?
                        .to_owned(),
                    detected: item["detectedSourceLanguage"].as_str().map(str::to_owned),
                })
            })
            .collect()
    }
}

pub(super) fn page_url(from: &str, to: &str, text: &str) -> String {
    format!(
        "https://translate.google.com/?sl={}&tl={}&text={}&op=translate",
        percent_encode(from),
        percent_encode(to),
        percent_encode(text)
    )
}
