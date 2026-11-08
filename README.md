# comment-translate (Zed)

English | [日本語](README.ja.md)

A Zed extension that shows translated documentation (Google Translate / DeepL) in hover popups.

```
Open in Google Translate ↗   ← link to the translation service, prefilled with the original text
Signature
---
Translated documentation (code examples left as is)
---
Original                     ← the original text; hidden with show_original: false
```

## How it works

```
Zed ──stdio──▶ comment-translate-proxy ──stdio──▶ rust-analyzer
                 └ rewrites only textDocument/hover responses
```

- `extension.toml` / `src/` … the Zed extension (WASM). Prepares the proxy and returns the command that starts it.
  - `settings.rs` … turns `lsp.*` settings into proxy arguments and environment variables
  - `proxy_binary.rs` … finding and downloading the proxy
- `proxy/` … the LSP proxy (native). Handles relaying, translation and caching.
  - `lsp/` … reading, writing and relaying messages; locating the wrapped server
  - `hover/` … parsing (`markdown.rs`) and composing (`compose.rs`) hover Markdown
  - `translate/` … timeouts (`mod.rs`), the persistent cache (`cache.rs`), providers (`google.rs` / `deepl.rs`), protecting code fragments (`placeholder.rs`)
  - `config.rs` … arguments and environment variables

The proxy is looked up in this order: `lsp.comment-translate-rust.binary.path` → PATH → automatic download from GitHub Releases.

## Usage

Replace rust-analyzer with the proxy in `settings.json`.

```json
{
  "languages": {
    "Rust": {
      "language_servers": ["comment-translate-rust", "!rust-analyzer", "..."]
    }
  }
}
```

`initialization_options` / `settings` / `binary` under `lsp.rust-analyzer` are carried over as is.
rust-analyzer itself is looked up in this order: `lsp.rust-analyzer.binary.path` → the copy Zed downloaded → PATH.

### Settings

```json
{
  "lsp": {
    "comment-translate-rust": {
      "settings": {
        "provider": "google",
        "deepl_api_key": "",
        "target_language": "ja",
        "source_language": "auto",
        "show_original": true,
        "timeout_ms": 1500,
        "persist_cache": true
      }
    }
  }
}
```

| Key | Default | Description |
|---|---|---|
| `provider` | `"google"` | `"google"` or `"deepl"` |
| `deepl_api_key` | none | DeepL API key. Free keys ending in `:fx` use the Free API. Can also be set with the `DEEPL_API_KEY` environment variable. Falls back to Google if unset |
| `target_language` | `"ja"` | Language to translate into |
| `source_language` | `"auto"` | Language to translate from |
| `show_original` | `true` | Show the original text below the translation |
| `timeout_ms` | `1500` | How long to wait for a translation. On timeout only the original is shown; the translation is cached and shown on the next hover |
| `persist_cache` | `true` | Keep the translation cache in a file. With `false` it is kept in memory only and is gone when Zed exits (leftover cache files are deleted on startup) |

Run `editor: restart language server` after changing settings.

### Cache

Translations are saved to a file keyed by the source text (with `persist_cache: true`) and reused after a restart and while offline. Documentation that shows up in many places, such as `String`'s, is requested only once.

- Location: `~/Library/Caches/comment-translate/` on macOS, `$XDG_CACHE_HOME/comment-translate/` (or `~/.cache/comment-translate/`) on Linux, `%LOCALAPPDATA%\comment-translate\` on Windows. Override with the `COMMENT_TRANSLATE_CACHE_DIR` environment variable
- There is one file per provider and language pair (e.g. `google-auto-ja.jsonl`). Delete it to clear the cache
- When a translation is not cached and cannot be fetched, the original is shown with the reason at the top of the hover: `Offline` (could not connect), `Translation failed` (an error), or `Still translating` (exceeded `timeout_ms`; the next hover shows it)

Logs are available via `dev: open language server logs` → `comment-translate-rust`.

## Development

```sh
cargo install --path proxy   # a proxy on PATH takes precedence
```

In Zed, run `zed: install dev extension` and select the root of this repository.

## Release

1. Set `GITHUB_REPO` in `src/proxy_binary.rs` and `repository` in `extension.toml` to the actual repository.
2. Pushing a tag such as `v0.1.0` makes `.github/workflows/release.yml` build the proxy for each platform and attach it to the release.
3. Register the extension in [zed-industries/extensions](https://github.com/zed-industries/extensions).

## License

MIT
