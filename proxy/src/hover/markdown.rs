//! Splitting a comment's markdown into blocks, and finding the prose to translate.

pub enum Block {
    /// Code blocks, rules and code-only paragraphs, kept as-is.
    Verbatim(String),
    /// A paragraph split into lines that must stay separate (list items,
    /// headings, ...), with hard-wrapped lines joined.
    Prose(Vec<String>),
}

impl Block {
    pub fn source(&self) -> String {
        match self {
            Block::Verbatim(text) => text.clone(),
            Block::Prose(lines) => lines.join("\n"),
        }
    }

    pub fn translatable_texts(&self) -> impl Iterator<Item = &str> {
        let lines = match self {
            Block::Prose(lines) => lines.as_slice(),
            Block::Verbatim(_) => &[],
        };
        lines
            .iter()
            .map(|line| split_marker(line).1)
            .filter(|text| has_words(text))
    }
}

pub fn parse_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut fence: Option<(&str, Vec<&str>)> = None;

    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>| {
        if paragraph.is_empty() {
            return;
        }
        if paragraph.iter().any(|line| has_words(line)) {
            let mut lines: Vec<String> = Vec::new();
            for line in paragraph.iter() {
                match lines.last_mut() {
                    Some(last) if !starts_block(line) => {
                        last.push(' ');
                        last.push_str(line);
                    }
                    _ => lines.push(line.to_string()),
                }
            }
            blocks.push(Block::Prose(lines));
        } else {
            blocks.push(Block::Verbatim(paragraph.join("\n")));
        }
        paragraph.clear();
    };

    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some((marker, code)) = &mut fence {
            code.push(line);
            if trimmed.starts_with(*marker) {
                blocks.push(Block::Verbatim(code.join("\n")));
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut paragraph, &mut blocks);
            fence = Some((&trimmed[..3], vec![line]));
        } else if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks);
        } else if trimmed == "---" {
            flush(&mut paragraph, &mut blocks);
            blocks.push(Block::Verbatim("---".into()));
        } else {
            paragraph.push(trimmed);
        }
    }
    flush(&mut paragraph, &mut blocks);
    if let Some((_, code)) = fence {
        blocks.push(Block::Verbatim(code.join("\n")));
    }
    blocks
}

/// Whether a line starts a new markdown block (and so must not be joined to the previous one).
fn starts_block(line: &str) -> bool {
    !split_marker(line).0.is_empty() || line.starts_with('|')
}

/// Splits a leading list, heading or quote marker off a line, so that only the
/// text after it is sent for translation.
pub fn split_marker(line: &str) -> (&str, &str) {
    let after_hashes = line.trim_start_matches('#');
    let after_digits = line.trim_start_matches(|c: char| c.is_ascii_digit());
    let marker_len = if after_hashes.len() < line.len() && after_hashes.starts_with(' ') {
        line.len() - after_hashes.len() + 1
    } else if after_digits.len() < line.len() && after_digits.starts_with(". ") {
        line.len() - after_digits.len() + 2
    } else if ["- ", "* ", "+ ", "> "]
        .iter()
        .any(|marker| line.starts_with(marker))
    {
        2
    } else {
        0
    };
    line.split_at(marker_len)
}

/// Whether there is anything to translate outside inline code spans, so that
/// lines like "`T` = `i32`, `A` = `Global`" are left alone.
fn has_words(text: &str) -> bool {
    text.split('`')
        .step_by(2)
        .any(|outside| outside.chars().any(char::is_alphabetic))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOVER: &str = "```rust\nalloc::vec::Vec\n```\n\n```rust\npub fn len(&self) -> usize\n```\n\n---\n\n`T` = `i32`, `A` = `Global`\n\n---\n\nReturns the number of elements, also referred to\nas its 'length'.\n\n# Examples\n\n```\nlet a = vec![1];\n```\n\nSee also:\n- `capacity`\n- the `is_empty` method";

    #[test]
    fn collects_prose_to_translate() {
        let blocks = parse_blocks(HOVER);
        let texts: Vec<&str> = blocks.iter().flat_map(Block::translatable_texts).collect();
        assert_eq!(
            texts,
            [
                "Returns the number of elements, also referred to as its 'length'.",
                "Examples",
                "See also:",
                "the `is_empty` method",
            ]
        );
    }
}
