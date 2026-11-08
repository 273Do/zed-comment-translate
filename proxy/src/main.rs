//! A transparent LSP proxy that adds a translation of the documentation to
//! `textDocument/hover` responses.
//!
//! Usage: comment-translate-proxy [--provider google|deepl] [--to ja] [--from auto]
//!        [--timeout-ms 1500] [--show-original true] [--persist-cache true]
//!        -- <server> [args...]
//!
//! Layers, from the outside in:
//! - `lsp`: framing and relaying messages between Zed and the wrapped server
//! - `hover`: rewriting hover markdown (parsing, composing the new popup)
//! - `translate`: caching (in memory and on disk) and timeouts in front of a
//!   translation provider

mod config;
mod hover;
#[macro_use]
mod log;
mod lsp;
mod translate;

use std::{process::Stdio, sync::Arc};

use anyhow::{Context, Result};
use tokio::process::Command;

use config::Config;
use hover::HoverTranslator;
use translate::{clear_cache, default_cache_dir, Provider, Translator};

#[tokio::main]
async fn main() -> Result<()> {
    log::init();
    let config = Config::from_env()?;
    let server = lsp::server::resolve(&config.server);

    let provider = Provider::new(config.provider, config.deepl_api_key);
    log!(
        "starting {} (translate {} -> {} with {})",
        server.display(),
        config.from,
        config.to,
        provider.name()
    );
    // Without persistence, translations last until exit and leftover files are removed.
    let cache_dir = default_cache_dir();
    if let (false, Some(dir)) = (config.persist_cache, &cache_dir) {
        clear_cache(dir);
    }
    let translator = Translator::new(
        provider,
        config.from,
        config.to,
        config.timeout,
        cache_dir.filter(|_| config.persist_cache),
    )?;
    let hover = Arc::new(HoverTranslator::new(translator, config.show_original));

    let child = Command::new(&server)
        .args(&config.server_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("failed to start {}", server.display()))?;

    let status = lsp::run(child, hover).await?;
    log!("server exited with {status}");
    std::process::exit(status.code().unwrap_or(1));
}
