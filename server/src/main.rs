//! A language server that translates the comment under the cursor on hover,
//! in any language `comment` knows. It runs alongside each language's usual
//! server.
//!
//! Usage: comment-translate-server [--provider google|deepl] [--to ja] [--from auto]
//!        [--timeout-ms 1500] [--persist-cache true]
//!
//! Layers, from the outside in:
//! - `lsp`: framing, tracking open documents, answering requests
//! - `comment`: finding the comment under the cursor
//! - `hover`: composing the popup (parsing the comment's markdown, the link)
//! - `translate`: caching (in memory and on disk) and timeouts in front of a
//!   translation provider

mod comment;
mod config;
mod hover;
#[macro_use]
mod log;
mod lsp;
mod translate;

use std::sync::Arc;

use anyhow::Result;

use config::Config;
use hover::HoverTranslator;
use translate::{clear_cache, default_cache_dir, Provider, Translator};

#[tokio::main]
async fn main() -> Result<()> {
    log::init();
    let config = Config::from_env()?;

    let provider = Provider::new(config.provider, config.deepl_api_key);
    log!(
        "starting (translate {} -> {} with {})",
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
    let hover = Arc::new(HoverTranslator::new(translator));

    let shut_down = lsp::serve(hover).await?;
    log!("client closed the connection");
    std::process::exit(if shut_down { 0 } else { 1 });
}
