//! Zed extension that runs `comment-translate-server`, a language server that
//! translates the comment under the cursor on hover, alongside each
//! language's usual server.
//!
//! - `settings`: `lsp.comment-translate` settings → server arguments and environment
//! - `server_binary`: finding or downloading the server

mod server_binary;
mod settings;

use zed_extension_api::{self as zed, settings::LspSettings, LanguageServerId, Result, Worktree};

use server_binary::ServerBinary;
use settings::{server_args, server_env};

struct CommentTranslateExtension {
    server: ServerBinary,
}

impl zed::Extension for CommentTranslateExtension {
    fn new() -> Self {
        Self {
            server: ServerBinary::default(),
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
            None => self.server.path(id, worktree)?,
        };

        let mut args = server_args(own.settings.as_ref());
        args.extend(
            own_binary
                .and_then(|binary| binary.arguments.clone())
                .unwrap_or_default(),
        );

        let mut env = worktree.shell_env();
        env.extend(server_env(own.settings.as_ref()));
        env.extend(
            own_binary
                .and_then(|binary| binary.env.clone())
                .unwrap_or_default(),
        );

        Ok(zed::Command { command, args, env })
    }
}

zed::register_extension!(CommentTranslateExtension);
