//! Rewriting hover contents as: a link to the translation provider, the
//! signature, the translated documentation, and (optionally) the original.
//! Hovers the server leaves empty over a comment show its translation.

mod comment;
mod compose;
mod markdown;

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::translate::{Translator, Unavailable};
pub use comment::find_comment;
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
        let original_label = self.show_original.then_some(ORIGINAL_LABEL);
        let value = match self.translate(&markdown, original_label).await {
            Some(Ok(value)) => value,
            Some(Err(reason)) => format!("*{}*\n\n{markdown}", unavailable_notice(reason)),
            None => return message,
        };
        *contents = json!({ "kind": "markdown", "value": value });
        message
    }

    /// Fills a hover the server left empty with the translation of the comment
    /// under the cursor, shown for `range`. The original is in the editor
    /// already, so it is never repeated.
    pub async fn comment(&self, mut message: Value, text: &str, range: Value) -> Value {
        let value = match self.translate(text, None).await {
            Some(Ok(value)) => value,
            Some(Err(reason)) => format!("*{}*", unavailable_notice(reason)),
            None => return message,
        };
        message["result"] = json!({
            "contents": { "kind": "markdown", "value": value },
            "range": range,
        });
        message
    }

    /// The translated markdown, the reason there is none, or `None` when there
    /// is nothing to translate.
    async fn translate(
        &self,
        markdown: &str,
        original_label: Option<&str>,
    ) -> Option<Result<String, Unavailable>> {
        let blocks = parse_blocks(markdown);

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
            return None;
        }

        let translations = match self.translator.translate_all(&texts).await {
            Ok(translations) => translations,
            Err(reason) => return Some(Err(reason)),
        };
        let translations: HashMap<&str, &str> = texts
            .iter()
            .zip(&translations)
            .filter_map(|(text, translation)| Some((text.as_str(), translation.as_deref()?)))
            .collect();
        // Everything was already in the target language.
        if translations.is_empty() {
            return None;
        }
        let url = self.translator.page_url(&texts.join("\n"));
        let link = provider_link(open_in_label(self.translator.provider().kind()), &url);
        Some(Ok(compose(
            &blocks,
            &translations,
            Some(&link),
            original_label,
        )))
    }
}
