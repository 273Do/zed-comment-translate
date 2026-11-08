//! Turning `lsp.<id>` and `lsp.<backend>` settings into the proxy command.

use zed_extension_api::{
    serde_json::Value, settings::LspSettings, LanguageServerId, Result, Worktree,
};

/// The real language server wrapped by each proxy server declared in extension.toml.
pub fn backend_server(id: &LanguageServerId) -> Result<&'static str> {
    match id.as_ref() {
        "comment-translate-rust" => Ok("rust-analyzer"),
        other => Err(format!("unknown language server: {other}")),
    }
}

/// Settings of the wrapped server, so that existing `lsp.rust-analyzer`
/// configuration keeps working.
pub fn backend_settings(id: &LanguageServerId, worktree: &Worktree) -> Result<LspSettings> {
    Ok(LspSettings::for_worktree(backend_server(id)?, worktree).unwrap_or_default())
}

/// Converts `lsp.<id>.settings` into proxy arguments.
pub fn proxy_args(settings: Option<&Value>) -> Vec<String> {
    let Some(settings) = settings else {
        return Vec::new();
    };
    let options = [
        ("provider", "--provider"),
        ("target_language", "--to"),
        ("source_language", "--from"),
        ("show_original", "--show-original"),
        ("timeout_ms", "--timeout-ms"),
    ];
    options
        .into_iter()
        .filter_map(|(key, flag)| Some([flag.to_string(), setting_string(settings, key)?]))
        .flatten()
        .collect()
}

/// Settings passed through the environment rather than arguments, so that
/// secrets do not show up in process listings.
pub fn proxy_env(settings: Option<&Value>) -> Vec<(String, String)> {
    let Some(settings) = settings else {
        return Vec::new();
    };
    [("deepl_api_key", "COMMENT_TRANSLATE_DEEPL_API_KEY")]
        .into_iter()
        .filter_map(|(key, name)| Some((name.to_string(), setting_string(settings, key)?)))
        .collect()
}

fn setting_string(settings: &Value, key: &str) -> Option<String> {
    Some(match settings.get(key)? {
        Value::String(value) => value.clone(),
        value => value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use zed_extension_api::serde_json::json;

    use super::*;

    #[test]
    fn converts_settings_to_args() {
        let settings = json!({
            "provider": "deepl",
            "target_language": "en",
            "show_original": false,
            "timeout_ms": 800,
            "deepl_api_key": "secret",
            "unrelated": 1,
        });
        assert_eq!(
            proxy_args(Some(&settings)),
            [
                "--provider",
                "deepl",
                "--to",
                "en",
                "--show-original",
                "false",
                "--timeout-ms",
                "800"
            ]
        );
        assert_eq!(
            proxy_env(Some(&settings)),
            [(
                "COMMENT_TRANSLATE_DEEPL_API_KEY".to_string(),
                "secret".to_string()
            )]
        );
    }
}
