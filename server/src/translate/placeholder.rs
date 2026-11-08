//! Shielding inline code and links from translation.
//!
//! Text is sent as escaped HTML with each protected span replaced by a
//! `<code>N</code>` placeholder, which providers leave untranslated.

/// Escapes `text` as HTML and replaces inline code and links with
/// `<code>N</code>` placeholders.
pub fn protect(text: &str) -> (String, Vec<String>) {
    let mut html = String::new();
    let mut tokens = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if let Some(len) = protected_span(rest) {
            html.push_str(&format!("<code>{}</code>", tokens.len()));
            tokens.push(rest[..len].to_owned());
            rest = &rest[len..];
            continue;
        }
        match c {
            '&' => html.push_str("&amp;"),
            '<' => html.push_str("&lt;"),
            '>' => html.push_str("&gt;"),
            _ => html.push(c),
        }
        rest = &rest[c.len_utf8()..];
    }
    (html, tokens)
}

/// Length of the inline code span, `[link](url)` or `<autolink>` at the start of `s`.
fn protected_span(s: &str) -> Option<usize> {
    if s.starts_with('`') {
        let ticks = s.len() - s.trim_start_matches('`').len();
        return s[ticks..].find(&s[..ticks]).map(|end| end + 2 * ticks);
    }
    let link = s.strip_prefix('!').unwrap_or(s);
    if link.starts_with('[') {
        let label_end = link.find(']')?;
        if !link[label_end + 1..].starts_with('(') {
            // An intra-doc link like [`String`], which has no URL in source code.
            let label = &link[1..label_end];
            let is_code = label.len() >= 2 && label.starts_with('`') && label.ends_with('`');
            return is_code.then_some(label_end + 1 + s.len() - link.len());
        }
        let url_end = link[label_end..].find(')')? + label_end + 1;
        return Some(url_end + s.len() - link.len());
    }
    if s.starts_with("<http") {
        return s.find('>').map(|end| end + 1);
    }
    None
}

pub fn restore(html: &str, tokens: &[String]) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find("<code>") {
        out.push_str(&unescape(&rest[..start]));
        let after = &rest[start + "<code>".len()..];
        let token = after.find("</code>").and_then(|end| {
            let index: usize = after[..end].trim().parse().ok()?;
            Some((end, tokens.get(index)?))
        });
        match token {
            Some((end, token)) => {
                out.push_str(token);
                rest = &after[end + "</code>".len()..];
            }
            None => {
                out.push_str("<code>");
                rest = after;
            }
        }
    }
    out.push_str(&unescape(rest));
    out
}

fn unescape(html: &str) -> String {
    html.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protects_code_and_links() {
        let text = "Use `len` or [`Vec::capacity`](https://doc.rust-lang.org/x.html) <https://a.b> & ``a`b`` [`String`] [note].";
        let (html, tokens) = protect(text);
        assert_eq!(
            html,
            "Use <code>0</code> or <code>1</code> <code>2</code> &amp; <code>3</code> <code>4</code> [note]."
        );
        assert_eq!(
            tokens,
            [
                "`len`",
                "[`Vec::capacity`](https://doc.rust-lang.org/x.html)",
                "<https://a.b>",
                "``a`b``",
                "[`String`]"
            ]
        );
        assert_eq!(restore(&html, &tokens), text);
        assert_eq!(
            restore("<code> 1 </code>を使う &lt;T&gt;", &tokens),
            "[`Vec::capacity`](https://doc.rust-lang.org/x.html)を使う <T>"
        );
    }
}
