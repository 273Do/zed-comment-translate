# Comment Translate for Zed

[![GitHub release](https://img.shields.io/github/v/release/273Do/zed-comment-translator)](https://github.com/273Do/zed-comment-translator/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

English | [日本語](./docs/README.ja.md)

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

No other setup is needed: comments are translated in every supported
language.

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

Contributions and bug reports are welcome.

## License

This project is available under the [MIT License](./LICENSE).
