//! Google Translate (the keyless `translate_a` endpoint).

use anyhow::{Context, Result};
use serde_json::Value;

use super::{percent_encode, Translated};

const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/t";

pub struct Google;

impl Google {
    pub(super) async fn translate(
        &self,
        client: &reqwest::Client,
        from: &str,
        to: &str,
        texts: &[String],
    ) -> Result<Vec<Translated>> {
        let form: Vec<(&str, &str)> = texts.iter().map(|text| ("q", text.as_str())).collect();
        let body = client
            .post(ENDPOINT)
            .query(&[
                ("client", "gtx"),
                ("sl", from),
                ("tl", to),
                ("format", "html"),
            ])
            .form(&form)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let response: Value = serde_json::from_str(&body).context("invalid response JSON")?;
        let items = response.as_array().context("unexpected response shape")?;

        // Each item is either "translation" or ["translation", "detected language"].
        items
            .iter()
            .map(|item| {
                let (text, detected) = match item {
                    Value::String(text) => (text.as_str(), None),
                    _ => (
                        item[0].as_str().context("unexpected response item")?,
                        item[1].as_str(),
                    ),
                };
                Ok(Translated {
                    text: text.to_owned(),
                    detected: detected.map(str::to_owned),
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
