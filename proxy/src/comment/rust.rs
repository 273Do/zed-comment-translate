//! Rust comments, found with rustc's own lexer.

use ra_ap_rustc_lexer::{strip_shebang, tokenize, DocStyle, FrontmatterAllowed, TokenKind};

use super::{Kind, Span};

pub(super) fn comments(source: &str) -> Vec<Span> {
    let shebang = strip_shebang(source).unwrap_or(0);
    let mut start = shebang;
    tokenize(&source[shebang..], FrontmatterAllowed::Yes)
        .filter_map(|token| {
            let span_start = start;
            start += token.len as usize;
            let kind = match token.kind {
                TokenKind::LineComment { doc_style } => Kind::Line(match doc_style {
                    None => "//",
                    Some(DocStyle::Outer) => "///",
                    Some(DocStyle::Inner) => "//!",
                }),
                TokenKind::BlockComment {
                    doc_style,
                    terminated,
                } => Kind::Block {
                    open: if doc_style.is_some() { 3 } else { 2 },
                    close: if terminated { 2 } else { 0 },
                    stars: true,
                },
                _ => return None,
            };
            Some(Span {
                start: span_start,
                end: start,
                kind,
            })
        })
        .collect()
}
