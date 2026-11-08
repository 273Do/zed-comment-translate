//! Translation with an in-memory cache, in front of a [`Provider`].

mod deepl;
mod google;
mod placeholder;

use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Result};

/// Page text beyond this many characters is cut from "open in ..." links.
const MAX_PAGE_TEXT_CHARS: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Google,
    Deepl,
}

impl FromStr for ProviderKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "google" => Ok(Self::Google),
            "deepl" => Ok(Self::Deepl),
            other => bail!("unknown provider: {other} (expected google or deepl)"),
        }
    }
}

/// A translated text and, when the provider reports it, its detected source language.
struct Translated {
    text: String,
    detected: Option<String>,
}

pub enum Provider {
    Google(google::Google),
    Deepl(deepl::Deepl),
}

impl Provider {
    /// Falls back to Google when DeepL is selected without an API key.
    pub fn new(kind: ProviderKind, deepl_api_key: Option<String>) -> Self {
        match (kind, deepl_api_key) {
            (ProviderKind::Google, _) => Self::Google(google::Google),
            (ProviderKind::Deepl, Some(api_key)) => Self::Deepl(deepl::Deepl::new(api_key)),
            (ProviderKind::Deepl, None) => {
                log!("DeepL needs an API key (deepl_api_key); using Google instead");
                Self::Google(google::Google)
            }
        }
    }

    pub fn kind(&self) -> ProviderKind {
        match self {
            Self::Google(_) => ProviderKind::Google,
            Self::Deepl(_) => ProviderKind::Deepl,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Google(_) => "Google",
            Self::Deepl(_) => "DeepL",
        }
    }

    /// The provider's web page with `text` filled in.
    fn page_url(&self, from: &str, to: &str, text: &str) -> String {
        let text: String = text.chars().take(MAX_PAGE_TEXT_CHARS).collect();
        match self {
            Self::Google(_) => google::page_url(from, to, &text),
            Self::Deepl(_) => deepl::page_url(from, to, &text),
        }
    }

    /// Translates HTML texts in which `<code>` elements must be left alone.
    async fn translate(
        &self,
        client: &reqwest::Client,
        from: &str,
        to: &str,
        texts: &[String],
    ) -> Result<Vec<Translated>> {
        let translations = match self {
            Self::Google(google) => google.translate(client, from, to, texts).await?,
            Self::Deepl(deepl) => deepl.translate(client, from, to, texts).await?,
        };
        if translations.len() != texts.len() {
            bail!(
                "expected {} translations, got {}",
                texts.len(),
                translations.len()
            );
        }
        Ok(translations)
    }
}

pub struct Translator {
    inner: Arc<Inner>,
    timeout: Duration,
}

struct Inner {
    client: reqwest::Client,
    provider: Provider,
    from: String,
    to: String,
    /// `None` means the text is already in the target language.
    cache: Mutex<HashMap<String, Option<String>>>,
}

impl Translator {
    pub fn new(provider: Provider, from: String, to: String, timeout: Duration) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self {
            inner: Arc::new(Inner {
                client,
                provider,
                from,
                to,
                cache: Mutex::default(),
            }),
            timeout,
        })
    }

    pub fn provider(&self) -> &Provider {
        &self.inner.provider
    }

    /// The provider's web page translating `text`.
    pub fn page_url(&self, text: &str) -> String {
        let inner = &self.inner;
        inner.provider.page_url(&inner.from, &inner.to, text)
    }

    /// Translates every text in one request. Each element is `None` when that text
    /// is already in the target language. Returns `None` if the request failed or
    /// is still in flight after the timeout; a late result is still cached, so the
    /// next hover over the same item shows it.
    pub async fn translate_all(&self, texts: &[String]) -> Option<Vec<Option<String>>> {
        let mut missing: Vec<String> = {
            let cache = self.inner.cache.lock().unwrap();
            texts
                .iter()
                .filter(|text| !cache.contains_key(*text))
                .cloned()
                .collect()
        };
        missing.sort();
        missing.dedup();

        if !missing.is_empty() {
            let inner = self.inner.clone();
            let task = tokio::spawn(async move {
                match inner.fetch(&missing).await {
                    Ok(translations) => {
                        let mut cache = inner.cache.lock().unwrap();
                        cache.extend(missing.into_iter().zip(translations));
                        true
                    }
                    Err(error) => {
                        log!("translation failed: {error:#}");
                        false
                    }
                }
            });
            match tokio::time::timeout(self.timeout, task).await {
                Ok(Ok(true)) => {}
                Ok(_) => return None,
                Err(_) => {
                    log!("translation timed out; it will appear on the next hover");
                    return None;
                }
            }
        }

        let cache = self.inner.cache.lock().unwrap();
        texts.iter().map(|text| cache.get(text).cloned()).collect()
    }
}

impl Inner {
    async fn fetch(&self, texts: &[String]) -> Result<Vec<Option<String>>> {
        let protected: Vec<(String, Vec<String>)> = texts
            .iter()
            .map(|text| placeholder::protect(text))
            .collect();
        let html: Vec<String> = protected.iter().map(|(html, _)| html.clone()).collect();
        let translations = self
            .provider
            .translate(&self.client, &self.from, &self.to, &html)
            .await?;
        Ok(translations
            .into_iter()
            .zip(&protected)
            .map(|(translated, (_, tokens))| {
                let already_target = translated
                    .detected
                    .is_some_and(|detected| same_language(&detected, &self.to));
                (!already_target).then(|| placeholder::restore(&translated.text, tokens))
            })
            .collect())
    }
}

/// Whether a detected language ("EN", "ja", "zh-CN") matches the target
/// language ("en-US", "ja", "zh-TW"). A detected code without a region matches
/// any region of the target.
fn same_language(detected: &str, target: &str) -> bool {
    let detected = detected.to_ascii_lowercase();
    let target = target.to_ascii_lowercase();
    detected == target || (!detected.contains('-') && target.split('-').next() == Some(&detected))
}

/// Percent-encodes everything except RFC 3986 unreserved characters.
fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_languages() {
        assert!(same_language("EN", "en-US"));
        assert!(same_language("ja", "ja"));
        assert!(same_language("zh-CN", "zh-cn"));
        assert!(!same_language("zh-CN", "zh-TW"));
        assert!(!same_language("en", "ja"));
    }

    #[test]
    fn encodes_urls() {
        assert_eq!(percent_encode("a b/é~"), "a%20b%2F%C3%A9~");
    }

    #[test]
    fn parses_provider() {
        assert_eq!(
            "DeepL".parse::<ProviderKind>().unwrap(),
            ProviderKind::Deepl
        );
        assert!("bing".parse::<ProviderKind>().is_err());
    }
}
