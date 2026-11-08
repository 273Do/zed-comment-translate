//! Command-line arguments and environment variables.

use std::{env, str::FromStr, time::Duration};

use anyhow::{bail, Context, Result};

use crate::translate::ProviderKind;

pub struct Config {
    pub provider: ProviderKind,
    pub google_api_key: Option<String>,
    pub deepl_api_key: Option<String>,
    pub from: String,
    pub to: String,
    pub timeout: Duration,
    pub persist_cache: bool,
}

impl Config {
    /// Parses the command-line options. Every option can also be set
    /// with a `COMMENT_TRANSLATE_*` environment variable; arguments win.
    pub fn from_env() -> Result<Self> {
        let mut provider = env_or("COMMENT_TRANSLATE_PROVIDER", ProviderKind::Google);
        let mut from = env_or("COMMENT_TRANSLATE_FROM", "auto".to_string());
        let mut to = env_or("COMMENT_TRANSLATE_TO", "ja".to_string());
        let mut timeout_ms = env_or("COMMENT_TRANSLATE_TIMEOUT_MS", 1500u64);
        let mut persist_cache = env_or("COMMENT_TRANSLATE_PERSIST_CACHE", true);
        let google_api_key = api_key(&["COMMENT_TRANSLATE_GOOGLE_API_KEY"]);
        let deepl_api_key = api_key(&["COMMENT_TRANSLATE_DEEPL_API_KEY", "DEEPL_API_KEY"]);

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            let mut value = || args.next().with_context(|| format!("{arg} needs a value"));
            match arg.as_str() {
                "--provider" => provider = parse(&arg, &value()?)?,
                "--from" => from = value()?,
                "--to" => to = value()?,
                "--timeout-ms" => timeout_ms = parse(&arg, &value()?)?,
                "--persist-cache" => persist_cache = parse(&arg, &value()?)?,
                other => bail!("unknown argument: {other}"),
            }
        }

        Ok(Self {
            provider,
            google_api_key,
            deepl_api_key,
            from,
            to,
            timeout: Duration::from_millis(timeout_ms),
            persist_cache,
        })
    }
}

/// The first of the environment variables `names` that is set and not empty.
fn api_key(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|key| !key.is_empty()))
}

fn env_or<T: FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn parse<T: FromStr>(flag: &str, value: &str) -> Result<T> {
    value
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid value for {flag}: {value}"))
}
