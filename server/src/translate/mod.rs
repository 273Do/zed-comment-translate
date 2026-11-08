//! Translation with a persistent cache, in front of a [`Provider`].

mod cache;
mod deepl;
mod google;
mod placeholder;

use std::{
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{bail, Result};

use cache::Cache;
pub use cache::{clear as clear_cache, default_dir as default_cache_dir};
pub use deepl::API_PAGE as DEEPL_API_PAGE;
pub use google::API_PAGE as GOOGLE_API_PAGE;

/// Page text beyond this many characters is cut from "open in ..." links.
const MAX_PAGE_TEXT_CHARS: usize = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Google,
    Deepl,
}

impl ProviderKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Deepl => "DeepL",
        }
    }
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

/// Why [`Translator::translate_all`] has no translation to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// The provider could not be reached.
    Offline,
    /// The provider returned an error or an unexpected response.
    Failed,
    /// Still in flight after the timeout; the next hover shows the result.
    Pending,
    /// Neither provider has an API key.
    NoApiKey,
}

/// A translated text and, when the provider reports it, its detected source language.
struct Translated {
    text: String,
    detected: Option<String>,
}

pub enum Provider {
    Google(google::Google),
    Deepl(deepl::Deepl),
    /// Selected, but neither provider has an API key; only cached
    /// translations can be shown.
    NoApiKey(ProviderKind),
}

impl Provider {
    /// Falls back to the other provider when the selected one has no API key.
    pub fn new(
        kind: ProviderKind,
        google_api_key: Option<String>,
        deepl_api_key: Option<String>,
    ) -> Self {
        let google = google_api_key.map(|key| Self::Google(google::Google::new(key)));
        let deepl = deepl_api_key.map(|key| Self::Deepl(deepl::Deepl::new(key)));
        let (selected, other) = match kind {
            ProviderKind::Google => (google, deepl),
            ProviderKind::Deepl => (deepl, google),
        };
        selected
            .or_else(|| {
                let other = other?;
                log!(
                    "{} has no API key; using {} instead",
                    kind.name(),
                    other.name()
                );
                Some(other)
            })
            .unwrap_or_else(|| {
                log!(
                    "no API key: set google_api_key ({}) or deepl_api_key ({})",
                    GOOGLE_API_PAGE,
                    DEEPL_API_PAGE
                );
                Self::NoApiKey(kind)
            })
    }

    pub fn kind(&self) -> ProviderKind {
        match self {
            Self::Google(_) => ProviderKind::Google,
            Self::Deepl(_) => ProviderKind::Deepl,
            Self::NoApiKey(kind) => *kind,
        }
    }

    pub fn name(&self) -> &'static str {
        self.kind().name()
    }

    /// The provider's web page with `text` filled in.
    fn page_url(&self, from: &str, to: &str, text: &str) -> String {
        let text: String = text.chars().take(MAX_PAGE_TEXT_CHARS).collect();
        match self.kind() {
            ProviderKind::Google => google::page_url(from, to, &text),
            ProviderKind::Deepl => deepl::page_url(from, to, &text),
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
            Self::NoApiKey(_) => bail!("no API key"),
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
    cache: Mutex<Cache>,
}

impl Translator {
    /// Translations are cached in `cache_dir`, separately per provider and
    /// language pair; without it they are only kept in memory.
    pub fn new(
        provider: Provider,
        from: String,
        to: String,
        timeout: Duration,
        cache_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;
        let cache = Cache::open(
            cache_dir,
            &format!("{}-{from}-{to}", provider.name()).to_lowercase(),
        );
        Ok(Self {
            inner: Arc::new(Inner {
                client,
                provider,
                from,
                to,
                cache: Mutex::new(cache),
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

    /// Translates the texts not in the cache in one request. Each element is
    /// `None` when that text is already in the target language. A result that
    /// arrives after the timeout is still cached, so the next hover shows it.
    pub async fn translate_all(
        &self,
        texts: &[String],
    ) -> Result<Vec<Option<String>>, Unavailable> {
        let mut missing: Vec<String> = {
            let cache = self.inner.cache.lock().unwrap();
            texts
                .iter()
                .filter(|text| !cache.contains(text))
                .cloned()
                .collect()
        };
        missing.sort();
        missing.dedup();

        if !missing.is_empty() {
            if let Provider::NoApiKey(_) = self.inner.provider {
                return Err(Unavailable::NoApiKey);
            }
            let inner = self.inner.clone();
            let task = tokio::spawn(async move {
                match inner.fetch(&missing).await {
                    Ok(translations) => {
                        let mut cache = inner.cache.lock().unwrap();
                        cache.extend(missing.into_iter().zip(translations));
                        Ok(())
                    }
                    Err(error) => {
                        log!("translation failed: {error:#}");
                        Err(if is_offline(&error) {
                            Unavailable::Offline
                        } else {
                            Unavailable::Failed
                        })
                    }
                }
            });
            match tokio::time::timeout(self.timeout, task).await {
                Ok(Ok(result)) => result?,
                Ok(Err(_)) => return Err(Unavailable::Failed),
                Err(_) => {
                    log!("translation timed out; it will appear on the next hover");
                    return Err(Unavailable::Pending);
                }
            }
        }

        let cache = self.inner.cache.lock().unwrap();
        texts
            .iter()
            .map(|text| cache.get(text).cloned().ok_or(Unavailable::Failed))
            .collect()
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

/// Whether the request never reached the provider (no network, DNS failure, ...).
fn is_offline(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(|error| error.is_connect() || error.is_timeout())
    })
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
    fn falls_back_to_the_provider_with_a_key() {
        let key = || Some("key".to_string());
        assert_eq!(
            Provider::new(ProviderKind::Google, key(), key()).name(),
            "Google"
        );
        assert_eq!(
            Provider::new(ProviderKind::Google, None, key()).name(),
            "DeepL"
        );
        assert_eq!(
            Provider::new(ProviderKind::Deepl, key(), None).name(),
            "Google"
        );
        assert!(matches!(
            Provider::new(ProviderKind::Deepl, None, None),
            Provider::NoApiKey(ProviderKind::Deepl)
        ));
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
