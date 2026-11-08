//! Zed extension that runs `comment-translate-proxy`, a language server that
//! translates the comment under the cursor on hover, alongside each
//! language's usual server.
//!
//! - `settings`: `lsp.comment-translate` settings → proxy arguments and environment
//! - `proxy_binary`: finding or downloading the proxy

mod proxy_binary;
mod settings;

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result, Worktree};

use proxy_binary::ProxyBinary;
use settings::{proxy_args, proxy_env};

struct CommentTranslateExtension {
    proxy: ProxyBinary,
}

impl zed::Extension for CommentTranslateExtension {
    fn new() -> Self {
        Self {
            proxy: ProxyBinary::default(),
        }
    }

    fn language_server_command(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<zed::Command> {
        let own = LspSettings::for_worktree(id.as_ref(), worktree).unwrap_or_default();
        let own_binary = own.binary.as_ref();

        let command = match own_binary.and_then(|binary| binary.path.clone()) {
            Some(path) => path,
            None => self.proxy.path(id, worktree)?,
        };

        let mut args = proxy_args(own.settings.as_ref());
        args.extend(
            own_binary
                .and_then(|binary| binary.arguments.clone())
                .unwrap_or_default(),
        );

        let mut env = worktree.shell_env();
        env.extend(proxy_env(own.settings.as_ref()));
        env.extend(
            own_binary
                .and_then(|binary| binary.env.clone())
                .unwrap_or_default(),
        );

        Ok(zed::Command { command, args, env })
    }
}

zed::register_extension!(CommentTranslateExtension);
