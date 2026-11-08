//! Building the new hover markdown and its user-facing labels.

use std::collections::HashMap;

use super::markdown::{split_marker, Block};
use crate::translate::{ProviderKind, Unavailable};

/// Text of the link to the provider's page.
pub fn open_in_label(provider: ProviderKind) -> &'static str {
    match provider {
        ProviderKind::Google => "Open in Google Translate",
        ProviderKind::Deepl => "Open in DeepL",
    }
}

/// Shown in place of the translation when there is none.
pub fn unavailable_notice(reason: Unavailable) -> &'static str {
    match reason {
        Unavailable::Offline => "Offline: could not translate",
        Unavailable::Failed => "Translation failed",
        Unavailable::Pending => "Still translating: hover again to see the translation",
    }
}

/// A markdown link to the provider's page, shown at the top of the popup.
pub fn provider_link(label: &str, url: &str) -> String {
    format!("[{label} ↗]({url})")
}

/// Builds the hover: the provider link, then the comment with its prose
/// translated and its code blocks unchanged.
pub fn compose(blocks: &[Block], translations: &HashMap<&str, &str>, link: &str) -> String {
    let translated = blocks
        .iter()
        .map(|block| match block {
            Block::Prose(lines) => lines
                .iter()
                .map(|line| {
                    let (marker, text) = split_marker(line);
                    format!("{marker}{}", translations.get(text).unwrap_or(&text))
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Verbatim(_) => block.source(),
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!("{link}\n\n{translated}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hover::markdown::parse_blocks;

    #[test]
    fn translates_prose_below_the_link() {
        let blocks = parse_blocks("Returns the number of\nelements.\n\n# Examples\n\n```\nlet a = vec![1];\n```\n\n- `capacity`\n- the `is_empty` method");
        let translations = HashMap::from([
            ("Returns the number of elements.", "要素数を返します。"),
            ("Examples", "例"),
            ("the `is_empty` method", "`is_empty` メソッド"),
        ]);
        let link = provider_link(open_in_label(ProviderKind::Google), "https://example.com");
        assert_eq!(
            compose(&blocks, &translations, &link),
            "[Open in Google Translate ↗](https://example.com)\n\n要素数を返します。\n\n# 例\n\n```\nlet a = vec![1];\n```\n\n- `capacity`\n- `is_empty` メソッド"
        );
    }
}
