//! Building the new hover markdown and its user-facing labels.

use std::collections::HashMap;

use super::markdown::{split_marker, Block};
use crate::translate::{ProviderKind, Unavailable};

/// Heading above the original documentation.
pub const ORIGINAL_LABEL: &str = "Original";

/// Text of the link to the provider's page.
pub fn open_in_label(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Google => "Open in Google Translate",
        ProviderKind::Deepl => "Open in DeepL",
    }
}

/// Shown above the original documentation when there is no translation.
pub fn unavailable_notice(reason: Unavailable) -> &'static str {
    match reason {
        Unavailable::Offline => "Offline: showing the original",
        Unavailable::Failed => "Translation failed: showing the original",
        Unavailable::Pending => "Still translating: hover again to see the translation",
    }
}

/// A markdown link to the provider's page, shown at the top of the popup.
pub fn provider_link(label: &str, url: &str) -> String {
    format!("[{label} ↗]({url})")
}

/// Builds the new hover: the provider link, everything before the first prose
/// paragraph (the signature) unchanged, then the translated documentation, then
/// optionally the original documentation under `original_label`.
pub fn compose(
    blocks: &[Block],
    translations: &HashMap<&str, &str>,
    link: Option<&str>,
    original_label: Option<&str>,
) -> String {
    let body_start = blocks
        .iter()
        .position(|block| matches!(block, Block::Prose(_)))
        .unwrap_or(blocks.len());
    let (header, body) = blocks.split_at(body_start);
    let join = |blocks: &[Block], render: &dyn Fn(&Block) -> String| {
        blocks.iter().map(render).collect::<Vec<_>>().join("\n\n")
    };

    let translated = join(body, &|block| match block {
        Block::Prose(lines) => lines
            .iter()
            .map(|line| {
                let (marker, text) = split_marker(line);
                format!("{marker}{}", translations.get(text).unwrap_or(&text))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Block::Verbatim(_) => block.source(),
    });

    let mut sections = vec![
        link.unwrap_or_default().to_owned(),
        join(header, &Block::source),
        translated,
    ];
    if let Some(label) = original_label {
        sections.push(format!(
            "---\n\n*{label}*\n\n{}",
            join(body, &Block::source)
        ));
    }
    sections.retain(|section| !section.is_empty());
    sections.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hover::markdown::{parse_blocks, tests::HOVER};

    #[test]
    fn composes_translation_above_original() {
        let blocks = parse_blocks(HOVER);
        let translations = HashMap::from([
            (
                "Returns the number of elements, also referred to as its 'length'.",
                "要素数を返します。",
            ),
            ("Examples", "例"),
            ("See also:", "関連:"),
            ("the `is_empty` method", "`is_empty` メソッド"),
        ]);
        let signature = "```rust\nalloc::vec::Vec\n```\n\n```rust\npub fn len(&self) -> usize\n```\n\n---\n\n`T` = `i32`, `A` = `Global`\n\n---";
        let translated = "要素数を返します。\n\n# 例\n\n```\nlet a = vec![1];\n```\n\n関連:\n- `capacity`\n- `is_empty` メソッド";
        assert_eq!(
            compose(&blocks, &translations, None, None),
            format!("{signature}\n\n{translated}")
        );

        let original = "Returns the number of elements, also referred to as its 'length'.\n\n# Examples\n\n```\nlet a = vec![1];\n```\n\nSee also:\n- `capacity`\n- the `is_empty` method";
        assert_eq!(
            compose(&blocks, &translations, None, Some(ORIGINAL_LABEL)),
            format!("{signature}\n\n{translated}\n\n---\n\n*Original*\n\n{original}")
        );
    }

    #[test]
    fn puts_link_on_top() {
        let blocks = parse_blocks("```rust\nfn f()\n```\n\nDocs.");
        let translations = HashMap::from([("Docs.", "ドキュメント。")]);
        let link = provider_link(open_in_label(ProviderKind::Google), "https://example.com");
        assert_eq!(
            compose(&blocks, &translations, Some(&link), None),
            "[Open in Google Translate ↗](https://example.com)\n\n```rust\nfn f()\n```\n\nドキュメント。"
        );
    }
}
