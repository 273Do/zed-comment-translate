# Comment Translate for Zed

[![GitHub release](https://img.shields.io/github/v/release/273Do/zed-comment-translator)](https://github.com/273Do/zed-comment-translator/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

English | [日本語](./docs/README.ja.md)

Hover over a comment in [Zed](https://zed.dev/) to read it in your own
language, translated with [Google Translate](https://translate.google.com/) or
[DeepL](https://www.deepl.com/). Works the same way in 60+ languages.

<img src="./docs/imgs/tr-google.png">
<img src="./docs/imgs/tr-deepl.png">

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
- Supports the Google Cloud Translation API and DeepL (Free and Pro keys).
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
Zed ──stdio──▶ comment-translate-server   (alongside rust-analyzer, pyright, ...)
                 └ answers hovers over comments
```

The Zed extension (WASM) prepares `comment-translate-server` (native) and
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

Then set an API key for Google or DeepL (see [API keys](#api-keys)).
Comments are then translated in every supported language. Without a key,
hovers say that a key is needed.

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
        "google_api_key": "",
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

| Key               | Default    | Description                                                                                                                                                                                                       |
| ----------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `provider`        | `"google"` | `"google"` or `"deepl"`                                                                                                                                                                                           |
| `google_api_key`  | none       | Google Cloud Translation API key. Can also be set with the `COMMENT_TRANSLATE_GOOGLE_API_KEY` environment variable. Falls back to DeepL if unset                                                                   |
| `deepl_api_key`   | none       | [DeepL API key](https://www.deepl.com/pro-api). Free keys ending in `:fx` use the Free API. Can also be set with the `DEEPL_API_KEY` environment variable. Falls back to Google if unset                         |
| `target_language` | `"ja"`     | Language to translate into                                                                                                                                                                                        |
| `source_language` | `"auto"`   | Language to translate from                                                                                                                                                                                        |
| `timeout_ms`      | `1500`     | How long to wait for a translation. On timeout the hover says so; the translation is cached and shown on the next hover                                                                                           |
| `persist_cache`   | `true`     | Keep the translation cache in a file. With `false` it is kept in memory only and is gone when Zed exits (leftover cache files are deleted on startup)                                                             |

Run `editor: restart language server` after changing the settings.

### API keys

Set one of the two keys; with both, `provider` picks which one is used.

**Google Cloud Translation** (500,000 characters a month free, then paid):

1. Create a project in the [Google Cloud console](https://console.cloud.google.com/)
   and link a billing account to it. This is required even within the free
   tier.
2. Enable the [Cloud Translation API](https://console.cloud.google.com/apis/library/translate.googleapis.com).
3. In **APIs & Services → Credentials**, select **Create credentials → API
   key**, and restrict the key to the Cloud Translation API.

**DeepL**: sign up for the [DeepL API](https://www.deepl.com/pro-api) (the
Free plan allows 500,000 characters a month) and copy the key from your
account page.

> [!IMPORTANT]
> Treat your API keys like passwords: a leaked Google key can incur charges.
> Do not commit them to any repository. The keys are passed to the server
> through environment variables, so they do not appear in process listings.

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

## Privacy

> [!WARNING]
> The comments you hover over are sent to the selected translation service
> (Google or DeepL). No other code is sent, and comments already in the cache
> are not sent again. Do not use this extension on code whose comments must
> not leave your machine.

## Development

Requires [Rust](https://rustup.rs/).

1. Install the language server: `cargo install --path server`
2. In Zed, run `zed: install dev extension` and select this repository.

Contributions and bug reports are welcome.

## License

This project is available under the [MIT License](./LICENSE).
