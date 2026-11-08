//! Finding the comment under the cursor in source code, for the languages in
//! [`syntax`]. Rust is lexed exactly with rustc's lexer; other languages are
//! scanned using their comment and string delimiters.

mod rust;
mod syntax;

/// A comment in a source file: its byte range and its text without the comment markers.
#[derive(Debug, PartialEq, Eq)]
pub struct Comment {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// A comment token as found by a lexer.
struct Span {
    start: usize,
    end: usize,
    kind: Kind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A line comment and its marker (`//`, `///`, `#`, ...). Consecutive line
    /// comments with the same marker form one comment.
    Line(&'static str),
    /// A block comment, with the lengths of its delimiters (0 for a missing
    /// close) and whether its lines may start with `*`.
    Block {
        open: usize,
        close: usize,
        stars: bool,
    },
}

/// Returns the comment containing the byte `offset` of `source`, written in
/// the language with the LSP id `language_id`. Consecutive line comments of
/// the same kind, each on its own line, form one comment.
pub fn find_comment(language_id: &str, source: &str, offset: usize) -> Option<Comment> {
    let spans = match language_id {
        "rust" => rust::comments(source),
        _ => syntax::for_language(language_id)?.comments(source),
    };
    let index = spans
        .iter()
        .position(|span| span.start <= offset && offset < span.end)?;

    // Only whitespace precedes the span on its line.
    let starts_line = |span: &Span| {
        let line_start = source[..span.start].rfind('\n').map_or(0, |i| i + 1);
        source[line_start..span.start].trim().is_empty()
    };
    // `next` continues the comment of `previous`, one line below it.
    let joins = |previous: &Span, next: &Span| {
        let gap = &source[previous.end..next.start];
        matches!(next.kind, Kind::Line(_))
            && next.kind == previous.kind
            && starts_line(previous)
            && starts_line(next)
            && gap.trim().is_empty()
            && gap.matches('\n').count() <= 1
    };
    let (mut first, mut last) = (index, index);
    while first > 0 && joins(&spans[first - 1], &spans[first]) {
        first -= 1;
    }
    while last + 1 < spans.len() && joins(&spans[last], &spans[last + 1]) {
        last += 1;
    }

    let lines: Vec<&str> = spans[first..=last]
        .iter()
        .flat_map(|span| comment_lines(&source[span.start..span.end], span.kind))
        .collect();
    Some(Comment {
        start: spans[first].start,
        end: spans[last].end,
        // The first line of a block comment shares its line with the opening marker.
        text: dedent(&lines, matches!(spans[index].kind, Kind::Block { .. })),
    })
}

/// The lines of one comment, without its markers.
fn comment_lines(comment: &str, kind: Kind) -> Vec<&str> {
    match kind {
        Kind::Line(marker) => {
            let mut text = &comment[marker.len()..];
            // `////`, `##` and `---` are the plain marker repeated.
            let first = marker.as_bytes()[0];
            if marker.bytes().all(|byte| byte == first) {
                text = text.trim_start_matches(first as char);
            }
            vec![text.trim_end_matches('\r')]
        }
        Kind::Block { open, close, stars } => {
            let inner = comment.get(open..comment.len() - close).unwrap_or_default();
            // `/*!` (the `*` of `/**` goes with the other lines' stars).
            let inner = match inner.strip_prefix('!') {
                Some(rest) if stars => rest,
                _ => inner,
            };
            inner
                .lines()
                .map(|line| match line.trim_start().strip_prefix('*') {
                    Some(rest) if stars => rest,
                    _ => line,
                })
                .collect()
        }
    }
}

/// Removes the indentation shared by all non-blank lines (except the first
/// with `first_apart`, which loses all of its own), and blank lines at both ends.
fn dedent(lines: &[&str], first_apart: bool) -> String {
    let skip = usize::from(first_apart);
    let indent = lines
        .iter()
        .skip(skip)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| match i < skip {
            true => line.trim(),
            false => line.get(indent..).unwrap_or_default().trim_end(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn find(language_id: &str, source: &str, needle: &str) -> Option<String> {
        find_comment(language_id, source, source.find(needle).unwrap()).map(|comment| comment.text)
    }

    #[test]
    fn joins_consecutive_line_comments() {
        let source = "fn f() {\n    // Returns the\n    // number.\n    //\n    //   indented\n\n    // Other.\n}";
        let comment = find_comment("rust", source, source.find("number").unwrap()).unwrap();
        assert_eq!(comment.text, "Returns the\nnumber.\n\n  indented");
        assert_eq!(
            &source[comment.start..comment.end],
            "// Returns the\n    // number.\n    //\n    //   indented"
        );
        assert_eq!(find("rust", source, "Other").unwrap(), "Other.");
    }

    #[test]
    fn keeps_doc_and_plain_comments_apart() {
        let source = "/// Docs.\n// Plain.\nfn f() {}";
        assert_eq!(find("rust", source, "Docs").unwrap(), "Docs.");
        assert_eq!(find("rust", source, "Plain").unwrap(), "Plain.");
    }

    #[test]
    fn trailing_comments_stand_alone() {
        let source = "let a = 1; // One.\n// Two.";
        assert_eq!(find("rust", source, "One").unwrap(), "One.");
        assert_eq!(find("rust", source, "Two").unwrap(), "Two.");
    }

    #[test]
    fn strips_block_comment_markers() {
        let source = "/**\n * Block\n * docs.\n */\nfn f() {}";
        assert_eq!(find("rust", source, "Block").unwrap(), "Block\ndocs.");
        assert_eq!(
            find("rust", "/* a /* b */ c */", "c").unwrap(),
            "a /* b */ c"
        );
        assert_eq!(find("typescript", source, "docs").unwrap(), "Block\ndocs.");
    }

    #[test]
    fn ignores_code_and_strings() {
        assert_eq!(find("rust", "let s = \"// no\";", "no"), None);
        assert_eq!(find("rust", "fn f() {}", "f()"), None);
        assert_eq!(find("plaintext", "// unknown language", "unknown"), None);
    }
}
