//! The text of open documents, kept in sync from `textDocument/did*`
//! notifications, and conversion between LSP positions and byte offsets.

use std::collections::HashMap;

use serde_json::{json, Value};

/// How `Position.character` counts, as negotiated in `initialize`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    #[default]
    Utf16,
    Utf32,
}

impl Encoding {
    pub fn from_lsp(name: &str) -> Self {
        match name {
            "utf-8" => Self::Utf8,
            "utf-32" => Self::Utf32,
            _ => Self::Utf16,
        }
    }

    fn units(self, c: char) -> usize {
        match self {
            Self::Utf8 => c.len_utf8(),
            Self::Utf16 => c.len_utf16(),
            Self::Utf32 => 1,
        }
    }
}

#[derive(Default)]
pub struct Documents {
    pub encoding: Encoding,
    texts: HashMap<String, String>,
}

impl Documents {
    pub fn text(&self, uri: &str) -> Option<&str> {
        self.texts.get(uri).map(String::as_str)
    }

    /// Applies a `textDocument/didOpen`, `didChange` or `didClose` notification.
    pub fn apply(&mut self, message: &Value) {
        let params = &message["params"];
        let Some(uri) = params["textDocument"]["uri"].as_str() else {
            return;
        };
        match message["method"].as_str() {
            Some("textDocument/didOpen") => {
                let text = params["textDocument"]["text"].as_str().unwrap_or_default();
                self.texts.insert(uri.to_owned(), text.to_owned());
            }
            Some("textDocument/didChange") => {
                let encoding = self.encoding;
                let Some(text) = self.texts.get_mut(uri) else {
                    return;
                };
                for change in params["contentChanges"].as_array().into_iter().flatten() {
                    let new_text = change["text"].as_str().unwrap_or_default();
                    match change.get("range") {
                        Some(range) => {
                            let start = offset(text, &range["start"], encoding);
                            let end = offset(text, &range["end"], encoding).max(start);
                            text.replace_range(start..end, new_text);
                        }
                        None => *text = new_text.to_owned(),
                    }
                }
            }
            Some("textDocument/didClose") => {
                self.texts.remove(uri);
            }
            _ => {}
        }
    }
}

/// The byte offset of an LSP `Position`, clamped to the line and the text.
pub fn offset(text: &str, position: &Value, encoding: Encoding) -> usize {
    let line = position["line"].as_u64().unwrap_or(0) as usize;
    let character = position["character"].as_u64().unwrap_or(0) as usize;
    let line_start = match line {
        0 => 0,
        _ => match text.match_indices('\n').nth(line - 1) {
            Some((index, _)) => index + 1,
            None => return text.len(),
        },
    };
    let mut units = 0;
    for (index, c) in text[line_start..].char_indices() {
        if units >= character || c == '\n' {
            return line_start + index;
        }
        units += encoding.units(c);
    }
    text.len()
}

/// The LSP `Position` of a byte offset.
pub fn position(text: &str, offset: usize, encoding: Encoding) -> Value {
    let before = &text[..offset];
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let character: usize = before[line_start..]
        .chars()
        .map(|c| encoding.units(c))
        .sum();
    json!({ "line": before.matches('\n').count(), "character": character })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_positions() {
        let text = "ab\n日本語x\n";
        let x = text.find('x').unwrap();
        let at = |line, character| json!({ "line": line, "character": character });
        assert_eq!(offset(text, &at(1, 3), Encoding::Utf16), x);
        assert_eq!(offset(text, &at(1, 9), Encoding::Utf8), x);
        assert_eq!(offset(text, &at(0, 99), Encoding::Utf16), 2);
        assert_eq!(offset(text, &at(9, 0), Encoding::Utf16), text.len());
        assert_eq!(position(text, x, Encoding::Utf16), at(1, 3));
        assert_eq!(position(text, x, Encoding::Utf32), at(1, 3));
        assert_eq!(position(text, x, Encoding::Utf8), at(1, 9));
    }

    #[test]
    fn applies_changes() {
        let mut documents = Documents::default();
        let uri = json!({ "uri": "file:///a.rs", "text": "// hello\nfn f() {}" });
        documents
            .apply(&json!({ "method": "textDocument/didOpen", "params": { "textDocument": uri } }));
        documents.apply(&json!({
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": "file:///a.rs" },
                "contentChanges": [{
                    "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 8 } },
                    "text": "こんにちは"
                }, {
                    "range": { "start": { "line": 1, "character": 3 }, "end": { "line": 1, "character": 4 } },
                    "text": "g"
                }]
            }
        }));
        assert_eq!(
            documents.text("file:///a.rs"),
            Some("// こんにちは\nfn g() {}")
        );
        documents.apply(&json!({ "method": "textDocument/didClose", "params": { "textDocument": { "uri": "file:///a.rs" } } }));
        assert_eq!(documents.text("file:///a.rs"), None);
    }
}
