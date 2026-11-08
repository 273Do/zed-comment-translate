//! Rewriting hover contents as: a link to the translation provider, the
//! signature, the translated documentation, and (optionally) the original.

mod compose;
mod markdown;

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::translate::Translator;
use compose::{compose, open_in_label, provider_link, unavailable_notice, ORIGINAL_LABEL};
use markdown::{hover_markdown, parse_blocks, Block};

/// Documentation beyond this many characters is left untranslated.
const MAX_CHARS: usize = 4500;

pub struct HoverTranslator {
    translator: Translator,
    show_original: bool,
}

impl HoverTranslator {
    pub fn new(translator: Translator, show_original: bool) -> Self {
        Self {
            translator,
            show_original,
        }
    }

    /// Rewrites a hover response in place. Documentation already in the target
    /// language is left alone; if the translation is unavailable (offline,
    /// failed or timed out), the original is shown with the reason.
    pub async fn augment(&self, mut message: Value) -> Value {
        let Some(contents) = message.pointer_mut("/result/contents") else {
            return message;
        };
        let markdown = hover_markdown(contents);
        let blocks = parse_blocks(&markdown);

        let mut texts = Vec::new();
        let mut total = 0;
        for text in blocks.iter().flat_map(Block::translatable_texts) {
            total += text.len();
            if total > MAX_CHARS {
                break;
            }
            texts.push(text.to_owned());
        }
        if texts.is_empty() {
            return message;
        }

        let url = self.translator.page_url(&texts.join("\n"));
        let link = provider_link(open_in_label(self.translator.provider().kind()), &url);

        let value = match self.translator.translate_all(&texts).await {
            Ok(translations) => {
                let translations: HashMap<&str, &str> = texts
                    .iter()
                    .zip(&translations)
                    .filter_map(|(text, translation)| {
                        Some((text.as_str(), translation.as_deref()?))
                    })
                    .collect();
                // Everything was already in the target language.
                if translations.is_empty() {
                    return message;
                }
                let original_label = self.show_original.then_some(ORIGINAL_LABEL);
                compose(&blocks, &translations, Some(&link), original_label)
            }
            Err(reason) => format!("{link}\n\n*{}*\n\n{markdown}", unavailable_notice(reason)),
        };
        *contents = json!({ "kind": "markdown", "value": value });
        message
    }
}
