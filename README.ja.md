# Comment Translate for Zed

[![GitHub release](https://img.shields.io/github/v/release/273Do/zed-comment-translator)](https://github.com/273Do/zed-comment-translator/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

[English](./README.md) | 日本語

[Zed](https://zed.dev/) のホバーポップアップで、ドキュメントを
[Google 翻訳](https://translate.google.com/) または [DeepL](https://www.deepl.com/)
で翻訳して表示する拡張機能。

Zed と rust-analyzer の間に入り、ホバーの応答に含まれるドキュメントだけを翻訳する。それ以外の通信はそのまま中継する。

```text
Open in Google Translate ↗   ← 翻訳サービスのページ (原文を入力済み)
シグネチャ
---
翻訳したドキュメント           (コード例はそのまま)
---
Original                      ← 原文。show_original: false で非表示
```

## 機能

- シグネチャ・コードブロック・インラインコードはそのままに、ホバーのドキュメントを翻訳する。
- Google 翻訳 (API キー不要) と DeepL (Free / Pro キー) に対応。
- 翻訳結果を原文をキーにしてファイルにキャッシュする。`String` のように同じドキュメントがいろいろな場所に出てきてもリクエストは 1 回だけで、キャッシュ済みの翻訳はオフライン時や再起動後にも使える。
- 翻訳できないとき (オフライン・失敗・翻訳中) は、理由を添えて原文を表示する。
- 翻訳できたときは、原文と言語を入力済みの翻訳サービスのページへリンクする。
- 既存の `lsp.rust-analyzer` の設定がそのまま使える。
- プロキシは自動でダウンロードされる。macOS・Linux・Windows 向けのビルド済みバイナリを用意している。

## 仕組み

```text
Zed ──stdio──▶ comment-translate-proxy ──stdio──▶ rust-analyzer
                 └ textDocument/hover の応答だけ書き換える
```

Zed 拡張 (WASM) が `comment-translate-proxy` を用意し、rust-analyzer の代わりに起動する。プロキシ (ネイティブ) は Zed と rust-analyzer の間のメッセージをすべて中継し、ホバーの応答に含まれるドキュメントを翻訳する。

## インストール

1. Zed の **Extensions** を開く。
2. **Comment Translate** を検索する。
3. **Install** を選ぶ。

> [!NOTE]
> 公開されるまでは、ソースからインストールする。[開発](#開発) を参照。

次に、Zed の設定で rust-analyzer をプロキシに置き換える。

```jsonc
{
  "languages": {
    "Rust": {
      "language_servers": ["comment-translate-rust", "!rust-analyzer", "..."]
    }
  }
}
```

プロキシは `lsp.comment-translate-rust.binary.path` → シェルの `PATH` → GitHub Releases からの自動ダウンロードの順に探す。
rust-analyzer 本体は `lsp.rust-analyzer.binary.path` → Zed がダウンロードしたもの → `PATH` の順に探す。`lsp.rust-analyzer` の `initialization_options` / `settings` / `binary` はそのまま引き継がれる。

## 設定

設定はすべて省略できる。

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

| キー | 既定値 | 説明 |
|---|---|---|
| `provider` | `"google"` | `"google"` または `"deepl"` |
| `deepl_api_key` | なし | DeepL の API キー。`:fx` で終わる Free キーは Free 用の API を使う。環境変数 `DEEPL_API_KEY` でも指定できる。未設定なら Google を使う |
| `target_language` | `"ja"` | 翻訳先の言語 |
| `source_language` | `"auto"` | 翻訳元の言語 |
| `show_original` | `true` | 翻訳の下に原文を表示する |
| `timeout_ms` | `1500` | 翻訳を待つ時間。超えたら原文を表示し、訳文はキャッシュして次回のホバーで表示する |
| `persist_cache` | `true` | 翻訳のキャッシュをファイルに残す。`false` ならメモリ上だけに持ち、Zed を終了すると消える (起動時に残っているキャッシュファイルも削除する) |

設定を変えたら `editor: restart language server` で反映する。

> [!IMPORTANT]
> DeepL の API キーはパスワードと同じように扱い、リポジトリにコミットしないこと。キーは環境変数でプロキシに渡すので、プロセス一覧には表示されない。

### キャッシュ

翻訳サービスと言語の組み合わせごとのファイル (例: `google-auto-ja.jsonl`) に保存される。

| OS | 保存先 |
|---|---|
| macOS | `~/Library/Caches/comment-translate/` |
| Linux | `$XDG_CACHE_HOME/comment-translate/` (未設定なら `~/.cache/comment-translate/`) |
| Windows | `%LOCALAPPDATA%\comment-translate\` |

環境変数 `COMMENT_TRANSLATE_CACHE_DIR` で保存先を変更できる。ファイルを消すとキャッシュがクリアされる。

## トラブルシューティング

- **ホバーが翻訳されない:** `language_servers` に `comment-translate-rust` があり、`!rust-analyzer` で組み込みのサーバーを無効にしているか確認し、言語サーバーを再起動する。
- **「Offline」や「Translation failed」と表示される:** ネットワーク接続を確認する。DeepL の場合は API キーと利用上限も確認する。キャッシュ済みの翻訳はこの場合も表示される。
- **「Still translating」と表示される:** 翻訳に `timeout_ms` 以上かかった。もう一度ホバーすると表示される。頻繁に起きるなら `timeout_ms` を大きくする。
- **プロキシをダウンロードできない:** GitHub Releases にアクセスできるか確認する。または `cargo install --path proxy` で自分でビルドし、`PATH` に置く。
- **詳しく調べたい:** `dev: open language server logs` を実行し、`comment-translate-rust` を選ぶ。

問題があれば、OS・Zed のバージョン・関連するログを添えて [Issue](https://github.com/273Do/zed-comment-translator/issues) を作成してほしい。

## 開発

リポジトリは 2 つのクレートからなる Cargo ワークスペース。

- `comment-translate` (リポジトリのルート): プロキシを用意して起動する Zed 拡張。
- `comment-translate-proxy` (`proxy/`): メッセージを中継し、ホバーのドキュメントを翻訳する LSP プロキシ。

手元の変更を試すには:

```bash
cargo install --path proxy   # PATH 上のプロキシはダウンロードより優先される
```

Zed で `zed: install dev extension` を実行し、このリポジトリのルートを選ぶ。

変更を送る前にワークスペースのチェックを実行する。

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

### リリース

1. `src/proxy_binary.rs` の `GITHUB_REPO` と `extension.toml` の `repository` をこのリポジトリにする。
2. `v0.1.0` のようなタグを push する。`.github/workflows/release.yml` が各プラットフォーム向けのプロキシをビルドし、GitHub の Release に添付する。
3. [zed-industries/extensions](https://github.com/zed-industries/extensions) に登録する。

コントリビューションやバグ報告を歓迎する。

## ライセンス

[MIT License](./LICENSE) で公開している。
