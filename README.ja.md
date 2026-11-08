# comment-translate (Zed)

[English](README.md) | 日本語

Zed のホバーポップアップに、ドキュメントの翻訳 (Google 翻訳 / DeepL) を表示する拡張機能。

```
Open in Google Translate ↗   ← 翻訳サービスのページへのリンク (原文を入力済み)
シグネチャ
---
翻訳したドキュメント (コード例はそのまま)
---
Original                     ← 原文。show_original: false で非表示
```

## 仕組み

```
Zed ──stdio──▶ comment-translate-proxy ──stdio──▶ rust-analyzer
                 └ textDocument/hover の応答だけ書き換える
```

- `extension.toml` / `src/` … Zed 拡張 (WASM)。プロキシを用意して起動コマンドを返す。
  - `settings.rs` … `lsp.*` の設定をプロキシの引数と環境変数に変換
  - `proxy_binary.rs` … プロキシの検出とダウンロード
- `proxy/` … LSP プロキシ (ネイティブ)。中継・翻訳・キャッシュを担当。
  - `lsp/` … メッセージの読み書きと中継、ラップするサーバーの検出
  - `hover/` … ホバーの Markdown の解析 (`markdown.rs`) と組み立て (`compose.rs`)
  - `translate/` … キャッシュとタイムアウト (`mod.rs`)、プロバイダー (`google.rs` / `deepl.rs`)、コード片の保護 (`placeholder.rs`)
  - `config.rs` … 引数と環境変数

プロキシは次の順で探す: `lsp.comment-translate-rust.binary.path` → PATH → GitHub Releases から自動ダウンロード。

## 使い方

`settings.json` で rust-analyzer をプロキシに置き換える。

```json
{
  "languages": {
    "Rust": {
      "language_servers": ["comment-translate-rust", "!rust-analyzer", "..."]
    }
  }
}
```

`lsp.rust-analyzer` の `initialization_options` / `settings` / `binary` はそのまま引き継がれる。
rust-analyzer 本体は `lsp.rust-analyzer.binary.path` → Zed がダウンロードしたもの → PATH の順に探す。

### 設定

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
        "timeout_ms": 1500
      }
    }
  }
}
```

| キー              | 既定値     | 説明                                                                                                                                  |
| ----------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `provider`        | `"google"` | `"google"` または `"deepl"`                                                                                                           |
| `deepl_api_key`   | なし       | DeepL の API キー。`:fx` で終わる Free キーは Free 用の API を使う。環境変数 `DEEPL_API_KEY` でも指定できる。未設定なら Google を使う |
| `target_language` | `"ja"`     | 翻訳先の言語                                                                                                                          |
| `source_language` | `"auto"`   | 翻訳元の言語                                                                                                                          |
| `show_original`   | `true`     | 翻訳の下に原文を表示する                                                                                                              |
| `timeout_ms`      | `1500`     | 翻訳を待つ時間。超えたら原文だけ表示し、訳文はキャッシュして次回のホバーで表示する                                                    |

設定を変えたら `editor: restart language server` で反映する。

ログは `dev: open language server logs` で `comment-translate-rust` を選ぶと見られる。

## 開発

```sh
cargo install --path proxy   # PATH 上のプロキシが優先される
```

Zed で `zed: install dev extension` を実行し、このリポジトリのルートを選ぶ。

## リリース

1. `src/proxy_binary.rs` の `GITHUB_REPO` と `extension.toml` の `repository` を実際のリポジトリにする。
2. `v0.1.0` のようなタグを push すると、`.github/workflows/release.yml` が各プラットフォーム向けのプロキシをビルドして Release に添付する。
3. [zed-industries/extensions](https://github.com/zed-industries/extensions) に登録する。

## ライセンス

MIT
