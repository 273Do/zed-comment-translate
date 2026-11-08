//! Hovers over comments: a link to the translation provider and the
//! translated comment. The original is in the editor already, so it is never
//! repeated.

mod compose;
mod markdown;

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::translate::{Translator, Unavailable};
use compose::{compose, open_in_label, provider_link, unavailable_notice};
use markdown::{parse_blocks, Block};

/// Comments beyond this many characters are left untranslated.
const MAX_CHARS: usize = 4500;

pub struct HoverTranslator {
    translator: Translator,
}

impl HoverTranslator {
    pub fn new(translator: Translator) -> Self {
        Self { translator }
    }

    /// Fills an empty hover response with the translation of the comment
    /// `text`, shown for `range`. Comments already in the target language get
    /// no hover; if the translation is unavailable (offline, failed or timed
    /// out), the reason is shown.
    pub async fn comment(&self, mut message: Value, text: &str, range: Value) -> Value {
        let value = match self.translate(text).await {
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
    async fn translate(&self, markdown: &str) -> Option<Result<String, Unavailable>> {
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
        Some(Ok(compose(&blocks, &translations, &link)))
    }
}
