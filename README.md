# Comment Translate for Zed

[![GitHub release](https://img.shields.io/github/v/release/273Do/zed-comment-translator)](https://github.com/273Do/zed-comment-translator/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

English | [日本語](./README.ja.md)

Read documentation in your own language in [Zed](https://zed.dev/)'s hover
popups, translated with [Google Translate](https://translate.google.com/) or
[DeepL](https://www.deepl.com/).

The extension sits between Zed and rust-analyzer, translates the
documentation in hover responses, and leaves everything else untouched.

```text
Open in Google Translate ↗   ← the translation page, prefilled with the original
Signature
---
Translated documentation      (code examples left as is)
---
Original                      ← hidden with show_original: false
```

## Features

- Translates hover documentation while keeping signatures, code blocks and
  inline code as is.
- Translates comments in the editor (`//`, `///`, `/* */`) when you hover
  over them; consecutive line comments are translated together.
- Supports Google Translate (no API key needed) and DeepL (Free and Pro keys).
- Caches translations on disk, keyed by the source text: documentation that
  appears in many places, such as `String`'s, is requested only once, and
  cached translations work offline and after a restart.
- Shows the original with the reason when a translation is unavailable
  (offline, failed, or still in progress).
- Links to the translation service's page with the text and languages filled
  in, when the translation succeeds.
- Keeps your existing `lsp.rust-analyzer` configuration working.
- Downloads the proxy automatically, with prebuilt binaries for macOS, Linux
  and Windows.

## How it works

```text
Zed ──stdio──▶ comment-translate-proxy ──stdio──▶ rust-analyzer
                 └ rewrites only textDocument/hover responses
```

The Zed extension (WASM) prepares `comment-translate-proxy` and starts it in
place of rust-analyzer. The proxy (native) relays every message between Zed and
rust-analyzer, and translates the documentation in hover responses.

## Installation

1. Open Zed's **Extensions** view.
2. Search for **Comment Translate**.
3. Select **Install**.

> [!NOTE]
> Until the extension is published, install it from source instead: see
> [Development](#development).

Then replace rust-analyzer with the proxy in your Zed settings:

```jsonc
{
  "languages": {
    "Rust": {
      "language_servers": ["comment-translate-rust", "!rust-analyzer", "..."]
    }
  }
}
```

The proxy is looked up in this order: `lsp.comment-translate-rust.binary.path`,
your shell `PATH`, then an automatic download from GitHub Releases.
rust-analyzer itself is looked up in this order: `lsp.rust-analyzer.binary.path`,
the copy Zed downloaded, then `PATH`. The `initialization_options`, `settings`
and `binary` under `lsp.rust-analyzer` are carried over as is.

## Configuration

All settings are optional:

```jsonc
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
| `timeout_ms` | `1500` | How long to wait for a translation. On timeout the original is shown; the translation is cached and shown on the next hover |
| `persist_cache` | `true` | Keep the translation cache in a file. With `false` it is kept in memory only and is gone when Zed exits (leftover cache files are deleted on startup) |

Run `editor: restart language server` after changing the settings.

> [!IMPORTANT]
> Treat your DeepL API key like a password. Do not commit it to any
> repository. The key is passed to the proxy through an environment variable,
> so it does not appear in process listings.

### Cache

Translations are stored per provider and language pair (e.g.
`google-auto-ja.jsonl`) in:

| Platform | Location |
|---|---|
| macOS | `~/Library/Caches/comment-translate/` |
| Linux | `$XDG_CACHE_HOME/comment-translate/` (or `~/.cache/comment-translate/`) |
| Windows | `%LOCALAPPDATA%\comment-translate\` |

Set the `COMMENT_TRANSLATE_CACHE_DIR` environment variable to use another
directory. Delete the files to clear the cache.

## Troubleshooting

- **Hovers are not translated:** make sure `comment-translate-rust` is listed
  and `!rust-analyzer` disables the built-in server in `language_servers`, then
  restart the language server.
- **The hover says "Offline" or "Translation failed":** check your network
  connection, and for DeepL, your API key and quota. Cached translations are
  still shown.
- **The hover says "Still translating":** the translation took longer than
  `timeout_ms`. Hover again to see it, or raise `timeout_ms`.
- **The proxy cannot be downloaded:** check access to GitHub Releases, or build
  it yourself with `cargo install --path proxy` so that it is on your `PATH`.
- **Need more detail:** run `dev: open language server logs` and select
  `comment-translate-rust`.

Please [open an issue](https://github.com/273Do/zed-comment-translator/issues)
with your OS, Zed version and the relevant logs if you run into a problem.

## Development

The repository is a Cargo workspace with two crates:

- `comment-translate` (repository root): the Zed extension that prepares and
  starts the proxy.
- `comment-translate-proxy` (`proxy/`): the LSP proxy that relays messages and
  translates hover documentation.

To try local changes:

```bash
cargo install --path proxy   # a proxy on PATH takes precedence over the download
```

Then run `zed: install dev extension` in Zed and select the root of this
repository.

Run the workspace checks before submitting a change:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

### Release

1. Set `GITHUB_REPO` in `src/proxy_binary.rs` and `repository` in
   `extension.toml` to this repository.
2. Push a tag such as `v0.1.0`. `.github/workflows/release.yml` builds the
   proxy for each platform and attaches it to the GitHub release.
3. Register the extension in
   [zed-industries/extensions](https://github.com/zed-industries/extensions).

Contributions and bug reports are welcome.

## License

This project is available under the [MIT License](./LICENSE).
