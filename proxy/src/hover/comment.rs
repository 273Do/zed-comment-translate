//! Finding the Rust comment under the cursor, for hovers the server leaves empty.

use ra_ap_rustc_lexer::{strip_shebang, tokenize, FrontmatterAllowed, TokenKind};

/// A comment in a source file: its byte range and its text without the comment markers.
#[derive(Debug, PartialEq, Eq)]
pub struct Comment {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

/// Returns the comment containing the byte `offset`. Consecutive line comments
/// of the same kind, each on its own line, form one comment.
pub fn find_comment(source: &str, offset: usize) -> Option<Comment> {
    let shebang = strip_shebang(source).unwrap_or(0);
    let mut start = shebang;
    let tokens: Vec<Token> = tokenize(&source[shebang..], FrontmatterAllowed::Yes)
        .map(|token| {
            let end = start + token.len as usize;
            let token = Token {
                kind: token.kind,
                start,
                end,
            };
            start = end;
            token
        })
        .collect();

    let index = tokens
        .iter()
        .position(|token| token.start <= offset && offset < token.end)?;
    let kind = tokens[index].kind;
    let newlines = |token: &Token| source[token.start..token.end].matches('\n').count();
    // Only whitespace precedes the token on its line.
    let starts_line = |i: usize| {
        i == 0
            || (tokens[i - 1].kind == TokenKind::Whitespace
                && (i == 1 || newlines(&tokens[i - 1]) > 0))
    };
    // Token `j` continues the comment from token `i`, one line above or below it.
    let joins = |i: usize, j: usize| {
        let gap = &tokens[i.min(j) + 1];
        tokens[j].kind == kind
            && starts_line(j)
            && gap.kind == TokenKind::Whitespace
            && newlines(gap) == 1
    };

    let (mut first, mut last) = (index, index);
    match kind {
        TokenKind::LineComment { .. } if starts_line(index) => {
            while first >= 2 && joins(first, first - 2) {
                first -= 2;
            }
            while last + 2 < tokens.len() && joins(last, last + 2) {
                last += 2;
            }
        }
        TokenKind::LineComment { .. } | TokenKind::BlockComment { .. } => {}
        _ => return None,
    }

    let lines: Vec<&str> = (first..=last)
        .step_by(2)
        .flat_map(|i| comment_lines(&source[tokens[i].start..tokens[i].end], kind))
        .collect();
    Some(Comment {
        start: tokens[first].start,
        end: tokens[last].end,
        text: dedent(&lines),
    })
}

/// The lines of one comment token, without `//`, `///`, `/*`, `*/` and leading `*`s.
fn comment_lines(comment: &str, kind: TokenKind) -> Vec<&str> {
    match kind {
        TokenKind::LineComment { doc_style } => vec![match doc_style {
            Some(_) => &comment[3..],
            None => comment.trim_start_matches('/'),
        }],
        TokenKind::BlockComment {
            doc_style,
            terminated,
        } => {
            let open = if doc_style.is_some() { 3 } else { 2 };
            let close = if terminated { 2 } else { 0 };
            comment
                .get(open..comment.len() - close)
                .unwrap_or_default()
                .lines()
                .map(|line| line.trim_start().strip_prefix('*').unwrap_or(line))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Removes the indentation shared by all non-blank lines, and blank lines at both ends.
fn dedent(lines: &[&str]) -> String {
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .map(|line| line.get(indent..).unwrap_or_default().trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(source: &str, needle: &str) -> Option<Comment> {
        find_comment(source, source.find(needle).unwrap())
    }

    #[test]
    fn joins_consecutive_line_comments() {
        let source = "fn f() {\n    // Returns the\n    // number.\n    //\n    //   indented\n\n    // Other.\n}";
        let comment = find(source, "number").unwrap();
        assert_eq!(comment.text, "Returns the\nnumber.\n\n  indented");
        assert_eq!(
            &source[comment.start..comment.end],
            "// Returns the\n    // number.\n    //\n    //   indented"
        );
        assert_eq!(find(source, "Other").unwrap().text, "Other.");
    }

    #[test]
    fn keeps_doc_and_plain_comments_apart() {
        let source = "/// Docs.\n// Plain.\nfn f() {}";
        assert_eq!(find(source, "Docs").unwrap().text, "Docs.");
        assert_eq!(find(source, "Plain").unwrap().text, "Plain.");
    }

    #[test]
    fn trailing_comments_stand_alone() {
        let source = "let a = 1; // One.\n// Two.";
        assert_eq!(find(source, "One").unwrap().text, "One.");
        assert_eq!(find(source, "Two").unwrap().text, "Two.");
    }

    #[test]
    fn strips_block_comment_markers() {
        let source = "/**\n * Block\n * docs.\n */\nfn f() {}";
        assert_eq!(find(source, "Block").unwrap().text, "Block\ndocs.");
        assert_eq!(find("/* a /* b */ c */", "c").unwrap().text, "a /* b */ c");
    }

    #[test]
    fn ignores_code_and_strings() {
        assert_eq!(find("let s = \"// no\";", "no"), None);
        assert_eq!(find("fn f() {}", "f()"), None);
    }
}
