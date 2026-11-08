//! Zed extension that runs the wrapped language server behind
//! `comment-translate-proxy`, which translates hover documentation.
//!
//! - `settings`: `lsp.*` settings → proxy arguments and environment
//! - `proxy_binary`: finding or downloading the proxy

mod proxy_binary;
mod settings;

use zed_extension_api::{
    self as zed, serde_json::Value, settings::LspSettings, LanguageServerId, Result, Worktree,
};

use proxy_binary::ProxyBinary;
use settings::{backend_server, backend_settings, proxy_args, proxy_env};

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
        let backend = backend_settings(id, worktree)?;

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
        args.push("--".into());
        match backend
            .binary
            .as_ref()
            .and_then(|binary| binary.path.clone())
        {
            Some(path) => {
                args.push(path);
                args.extend(
                    backend
                        .binary
                        .and_then(|binary| binary.arguments)
                        .unwrap_or_default(),
                );
            }
            // Let the proxy locate the backend (Zed's downloaded copy first, then PATH).
            None => args.push(backend_server(id)?.into()),
        }

        let mut env = worktree.shell_env();
        env.extend(proxy_env(own.settings.as_ref()));
        env.extend(
            own_binary
                .and_then(|binary| binary.env.clone())
                .unwrap_or_default(),
        );

        Ok(zed::Command { command, args, env })
    }

    fn language_server_initialization_options(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        Ok(backend_settings(id, worktree)?.initialization_options)
    }

    fn language_server_workspace_configuration(
        &mut self,
        id: &LanguageServerId,
        worktree: &Worktree,
    ) -> Result<Option<Value>> {
        Ok(backend_settings(id, worktree)?.settings)
    }
}

zed::register_extension!(CommentTranslateExtension);
