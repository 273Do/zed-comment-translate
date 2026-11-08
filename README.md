# Comment Translate for Zed

[![GitHub release](https://img.shields.io/github/v/release/273Do/zed-comment-translator)](https://github.com/273Do/zed-comment-translator/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

English | [日本語](./README.ja.md)

Hover over a comment in [Zed](https://zed.dev/) to read it in your own
language, translated with [Google Translate](https://translate.google.com/) or
[DeepL](https://www.deepl.com/). Works the same way in 60+ languages.

```text
Open in Google Translate ↗   ← the translation page, prefilled with the original
Translated comment            (code blocks and inline code left as is)
```

## Features

- Translates the comment under the cursor: line comments (`//`, `#`, `--`,
  ...), block comments (`/* */`, `<!-- -->`, ...) and doc comments (`///`,
  `/** */`, ...).
- Supports Rust, Python, JavaScript/TypeScript, Go, C/C++, Java, Ruby, Lua,
  Haskell, HTML and many more (see `languages` in
  [extension.toml](./extension.toml)).
- Translates consecutive line comments together, and never mistakes strings
  for comments.
- Keeps code blocks and inline code in comments as is.
- Supports Google Translate (no API key needed) and DeepL (Free and Pro keys).
- Caches translations on disk, keyed by the source text: cached translations
  are shown instantly, offline and after a restart.
- Shows the reason when a translation is unavailable (offline, failed, or
  still in progress).
- Links to the translation service's page with the text and languages filled
  in.
- Downloads the language server automatically, with prebuilt binaries for
  macOS, Linux and Windows.

To read the documentation of a library, such as a language's standard
library, jump to its source with **Go to Definition** (cmd/ctrl+click) and
hover over the comments there.

## How it works

```text
Zed ──stdio──▶ comment-translate-proxy   (alongside rust-analyzer, pyright, ...)
                 └ answers hovers over comments
```

The Zed extension (WASM) prepares `comment-translate-proxy` (native) and
starts it as a language server of its own, alongside each language's usual
server. It keeps track of open files, and answers hovers only: when the cursor
is on a comment, with its translation. Zed shows it together with the other
servers' hovers, which are left untouched.

## Installation

1. Open Zed's **Extensions** view.
2. Search for **Comment Translate**.
3. Select **Install**.

> [!NOTE]
> Until the extension is published, install it from source instead: see
> [Development](#development).

No other setup is needed. The server is enabled for every supported language;
if you set `language_servers` for a language, keep `"..."` in the list (or
add `"comment-translate"`). To turn it off for a language:

```jsonc
{
  "languages": {
    "Markdown": {
      "language_servers": ["!comment-translate", "..."],
    },
  },
}
```

The server is looked up in this order: `lsp.comment-translate.binary.path`,
your shell `PATH`, then an automatic download from GitHub Releases.

## Configuration

All settings are optional:

```jsonc
{
  "lsp": {
    "comment-translate": {
      "settings": {
        "provider": "google",
        "deepl_api_key": "",
        "target_language": "ja",
        "source_language": "auto",
        "timeout_ms": 1500,
        "persist_cache": true,
      },
    },
  },
}
```

| Key               | Default    | Description                                                                                                                                             |
| ----------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `provider`        | `"google"` | `"google"` or `"deepl"`                                                                                                                                 |
| `deepl_api_key`   | none       | DeepL API key. Free keys ending in `:fx` use the Free API. Can also be set with the `DEEPL_API_KEY` environment variable. Falls back to Google if unset |
| `target_language` | `"ja"`     | Language to translate into                                                                                                                              |
| `source_language` | `"auto"`   | Language to translate from                                                                                                                              |
| `timeout_ms`      | `1500`     | How long to wait for a translation. On timeout the hover says so; the translation is cached and shown on the next hover                                 |
| `persist_cache`   | `true`     | Keep the translation cache in a file. With `false` it is kept in memory only and is gone when Zed exits (leftover cache files are deleted on startup)   |

Run `editor: restart language server` after changing the settings.

> [!IMPORTANT]
> Treat your DeepL API key like a password. Do not commit it to any
> repository. The key is passed to the server through an environment variable,
> so it does not appear in process listings.

### Cache

Translations are stored per provider and language pair (e.g.
`google-auto-ja.jsonl`) in:

| Platform | Location                                                                |
| -------- | ----------------------------------------------------------------------- |
| macOS    | `~/Library/Caches/comment-translate/`                                   |
| Linux    | `$XDG_CACHE_HOME/comment-translate/` (or `~/.cache/comment-translate/`) |
| Windows  | `%LOCALAPPDATA%\comment-translate\`                                     |

Set the `COMMENT_TRANSLATE_CACHE_DIR` environment variable to use another
directory. Delete the files to clear the cache.

## Troubleshooting

- **Hovers over comments are not translated:** make sure the language's
  `language_servers` does not leave out `comment-translate` (keep `"..."` in
  the list), and that the language is listed in `extension.toml`. Comments
  already in the target language get no hover.
- **The hover says "Offline" or "Translation failed":** check your network
  connection, and for DeepL, your API key and quota. Cached translations are
  still shown.
- **The hover says "Still translating":** the translation took longer than
  `timeout_ms`. Hover again to see it, or raise `timeout_ms`.
- **The server cannot be downloaded:** check access to GitHub Releases, or build
  it yourself with `cargo install --path proxy` so that it is on your `PATH`.
- **Need more detail:** run `dev: open language server logs` and select
  `comment-translate`.

Please [open an issue](https://github.com/273Do/zed-comment-translator/issues)
with your OS, Zed version and the relevant logs if you run into a problem.

## Development

The repository is a Cargo workspace with two crates:

- `comment-translate` (repository root): the Zed extension that prepares and
  starts the language server.
- `comment-translate-proxy` (`proxy/`): the language server that finds the
  comment under the cursor and translates it.

To try local changes:

```bash
cargo install --path proxy   # a server on PATH takes precedence over the download
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
   server for each platform and attaches it to the GitHub release.
3. Register the extension in
   [zed-industries/extensions](https://github.com/zed-industries/extensions).

Contributions and bug reports are welcome.

## License

This project is available under the [MIT License](./LICENSE).
