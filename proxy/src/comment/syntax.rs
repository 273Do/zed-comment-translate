//! Comment and string delimiters of languages other than Rust, and a scanner
//! that finds comments with them, skipping string literals.

use super::{Kind, Span};

/// How a language writes comments and strings. Delimiters are ASCII and listed
/// longest first where one is a prefix of another.
pub(super) struct Syntax {
    line: &'static [&'static str],
    block: &'static [(&'static str, &'static str)],
    /// Whether block comments nest, like `/* /* */ */` in Swift.
    nested: bool,
    /// String delimiters. Single-character ones end at a line break, so that a
    /// stray quote does not hide the rest of the file.
    strings: &'static [&'static str],
}

const C_LINE: &[&str] = &["///", "//!", "//"];
const C_BLOCK: &[(&str, &str)] = &[("/*", "*/")];
const QUOTES: &[&str] = &["\"", "'"];

const C: Syntax = Syntax {
    line: C_LINE,
    block: C_BLOCK,
    nested: false,
    strings: QUOTES,
};
const C_NESTED: Syntax = Syntax {
    nested: true,
    strings: &["\"\"\"", "\"", "'"],
    ..C
};
const C_BACKTICK: Syntax = Syntax {
    strings: &["\"", "'", "`"],
    ..C
};
const HASH: Syntax = Syntax {
    line: &["#"],
    block: &[],
    nested: false,
    strings: QUOTES,
};
const PYTHON: Syntax = Syntax {
    strings: &["\"\"\"", "'''", "\"", "'"],
    ..HASH
};
const ML: Syntax = Syntax {
    line: &[],
    block: &[("(*", "*)")],
    nested: true,
    strings: &["\""],
};
const HTML: Syntax = Syntax {
    line: &[],
    block: &[("<!--", "-->")],
    nested: false,
    strings: &[],
};

pub(super) fn for_language(language_id: &str) -> Option<&'static Syntax> {
    Some(match language_id {
        "c" | "cpp" | "objective-c" | "objective-cpp" | "csharp" | "java" | "proto" | "glsl"
        | "hlsl" | "solidity" | "groovy" | "jsonc" => &C,
        "javascript" | "javascriptreact" | "typescript" | "typescriptreact" | "go" => &C_BACKTICK,
        "kotlin" | "swift" | "scala" | "dart" | "wgsl" => &C_NESTED,
        "zig" | "gleam" => &Syntax { block: &[], ..C },
        "css" => &Syntax { line: &[], ..C },
        "scss" | "less" => &C,
        "php" => &Syntax {
            line: &["///", "//", "#"],
            ..C
        },
        "vue" | "svelte" | "astro" => &Syntax {
            block: &[("<!--", "-->"), ("/*", "*/")],
            ..C_BACKTICK
        },
        "python" | "gdscript" => &PYTHON,
        "ruby" | "shellscript" | "fish" | "perl" | "r" | "yaml" | "dockerfile" | "makefile"
        | "cmake" | "nim" => &HASH,
        "toml" => &Syntax {
            strings: &["\"\"\"", "'''", "\"", "'"],
            ..HASH
        },
        "elixir" => &Syntax {
            strings: &["\"\"\"", "\"", "'"],
            ..HASH
        },
        "nix" | "terraform" | "hcl" => &Syntax {
            line: &["#", "//"],
            block: C_BLOCK,
            strings: &["\""],
            ..HASH
        },
        "julia" => &Syntax {
            block: &[("#=", "=#")],
            nested: true,
            strings: &["\"\"\"", "\""],
            ..HASH
        },
        "powershell" => &Syntax {
            block: &[("<#", "#>")],
            ..HASH
        },
        "sql" => &Syntax {
            line: &["--"],
            block: C_BLOCK,
            nested: false,
            strings: QUOTES,
        },
        "lua" => &Syntax {
            line: &["--"],
            block: &[("--[[", "]]")],
            nested: false,
            strings: QUOTES,
        },
        "haskell" | "elm" | "purescript" => &Syntax {
            line: &["--"],
            block: &[("{-", "-}")],
            nested: true,
            strings: &["\""],
        },
        "ocaml" => &ML,
        "fsharp" => &Syntax { line: C_LINE, ..ML },
        "clojure" | "scheme" | "racket" | "commonlisp" | "elisp" => &Syntax {
            line: &[";"],
            block: &[],
            nested: false,
            strings: &["\""],
        },
        "erlang" | "latex" | "tex" => &Syntax {
            line: &["%"],
            block: &[],
            nested: false,
            strings: &["\""],
        },
        "html" | "xml" | "svg" | "markdown" => &HTML,
        _ => return None,
    })
}

impl Syntax {
    pub(super) fn comments(&self, source: &str) -> Vec<Span> {
        let bytes = source.as_bytes();
        let at = |i: usize, delimiter: &str| bytes[i..].starts_with(delimiter.as_bytes());
        let mut spans = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            if let Some(&(open, close)) = self.block.iter().find(|(open, _)| at(i, open)) {
                let start = i;
                let mut depth = 0;
                let mut terminated = false;
                while i < bytes.len() {
                    if at(i, close) {
                        i += close.len();
                        depth -= 1;
                        if depth == 0 {
                            terminated = true;
                            break;
                        }
                    } else if (depth == 0 || self.nested) && at(i, open) {
                        i += open.len();
                        depth += 1;
                    } else {
                        i += 1;
                    }
                }
                let kind = Kind::Block {
                    open: open.len(),
                    close: if terminated { close.len() } else { 0 },
                    stars: open == "/*",
                };
                spans.push(Span {
                    start,
                    end: i,
                    kind,
                });
            } else if let Some(marker) = self.line.iter().find(|marker| at(i, marker)) {
                let start = i;
                i = source[i..].find('\n').map_or(bytes.len(), |end| i + end);
                spans.push(Span {
                    start,
                    end: i,
                    kind: Kind::Line(marker),
                });
            } else if let Some(quote) = self.strings.iter().find(|quote| at(i, quote)) {
                i += quote.len();
                while i < bytes.len() && !at(i, quote) {
                    if quote.len() == 1 && bytes[i] == b'\n' {
                        break;
                    }
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                i = (i + quote.len()).min(bytes.len());
            } else {
                i += 1;
            }
        }
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::find;

    #[test]
    fn finds_comments_in_other_languages() {
        let python = "def f():\n    # Returns the\n    # number.\n    return '# not a comment'  # trailing\n";
        assert_eq!(
            find("python", python, "number").unwrap(),
            "Returns the\nnumber."
        );
        assert_eq!(find("python", python, "not a"), None);
        assert_eq!(find("python", python, "trailing").unwrap(), "trailing");

        let go = "// Package a does b.\npackage a\n\nvar s = `// raw\nstring`";
        assert_eq!(find("go", go, "Package").unwrap(), "Package a does b.");
        assert_eq!(find("go", go, "raw"), None);

        let lua = "--[[ Block\n  comment ]]\n-- Line\nlocal s = \"-- no\"";
        assert_eq!(find("lua", lua, "comment").unwrap(), "Block\ncomment");
        assert_eq!(find("lua", lua, "Line").unwrap(), "Line");
        assert_eq!(find("lua", lua, "no"), None);

        let haskell = "{- outer {- inner -} still -}\nf = 1 -- note";
        assert_eq!(
            find("haskell", haskell, "still").unwrap(),
            "outer {- inner -} still"
        );
        assert_eq!(find("haskell", haskell, "note").unwrap(), "note");

        assert_eq!(
            find("html", "<p>a</p>\n<!-- Hidden. -->", "Hidden").unwrap(),
            "Hidden."
        );
    }

    #[test]
    fn stray_quotes_end_at_the_line() {
        let yaml = "key: it's\n# Comment\n";
        assert_eq!(find("yaml", yaml, "Comment").unwrap(), "Comment");
    }

    #[test]
    fn unterminated_block_comments_run_to_the_end() {
        assert_eq!(
            find("c", "int a; /* open\nstill", "still").unwrap(),
            "open\nstill"
        );
    }
}
