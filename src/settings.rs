//! Turning `lsp.comment-translate` settings into the server command.

use zed_extension_api::serde_json::Value;

/// Converts `lsp.<id>.settings` into server arguments.
pub fn server_args(settings: Option<&Value>) -> Vec<String> {
    let Some(settings) = settings else {
        return Vec::new();
    };
    let options = [
        ("provider", "--provider"),
        ("target_language", "--to"),
        ("source_language", "--from"),
        ("timeout_ms", "--timeout-ms"),
        ("persist_cache", "--persist-cache"),
    ];
    options
        .into_iter()
        .filter_map(|(key, flag)| Some([flag.to_string(), setting_string(settings, key)?]))
        .flatten()
        .collect()
}

/// Settings passed through the environment rather than arguments, so that
/// secrets do not show up in process listings.
pub fn server_env(settings: Option<&Value>) -> Vec<(String, String)> {
    let Some(settings) = settings else {
        return Vec::new();
    };
    [
        ("google_api_key", "COMMENT_TRANSLATE_GOOGLE_API_KEY"),
        ("deepl_api_key", "COMMENT_TRANSLATE_DEEPL_API_KEY"),
    ]
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
            "timeout_ms": 800,
            "persist_cache": false,
            "google_api_key": "google-secret",
            "deepl_api_key": "secret",
            "unrelated": 1,
        });
        assert_eq!(
            server_args(Some(&settings)),
            [
                "--provider",
                "deepl",
                "--to",
                "en",
                "--timeout-ms",
                "800",
                "--persist-cache",
                "false"
            ]
        );
        assert_eq!(
            server_env(Some(&settings)),
            [
                (
                    "COMMENT_TRANSLATE_GOOGLE_API_KEY".to_string(),
                    "google-secret".to_string()
                ),
                (
                    "COMMENT_TRANSLATE_DEEPL_API_KEY".to_string(),
                    "secret".to_string()
                )
            ]
        );
    }
}
