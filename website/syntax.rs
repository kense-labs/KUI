//! Shared escaped syntax markup for static examples and fetched source files.
use std::fmt::Write;

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// shortcut: line-oriented Rust and shell tokens; use a full lexer when multiline token fidelity is needed.
fn highlighted_line(mut line: &str, shell: bool) -> String {
    let mut result = String::new();
    let mut command_start = true;
    while let Some(ch) = line.chars().next() {
        let (end, kind) = if shell && matches!(ch, '|' | '&' | ';' | '(' | ')' | '<' | '>') {
            command_start = true;
            (ch.len_utf8(), "keyword")
        } else if (!shell && line.starts_with("//")) || (shell && ch == '#') {
            (line.len(), "comment")
        } else if ch == '"' || (shell && ch == '\'') {
            let mut escaped = false;
            let end = line[1..]
                .char_indices()
                .find_map(|(i, c)| {
                    let close = c == ch && !escaped;
                    escaped = c == '\\' && !escaped;
                    close.then_some(i + 2)
                })
                .unwrap_or(line.len());
            (end, "string")
        } else if shell && !ch.is_whitespace() {
            let end = line
                .find(|c: char| {
                    c.is_whitespace()
                        || matches!(c, '"' | '\'' | '|' | '&' | ';' | '(' | ')' | '<' | '>')
                })
                .unwrap_or(line.len());
            let word = &line[..end];
            let assignment = word.split_once('=').is_some_and(|(name, _)| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
            let kind = if word.starts_with('-') || assignment {
                "keyword"
            } else if word.starts_with('$') || ch.is_ascii_digit() {
                "number"
            } else if command_start {
                command_start = false;
                "tag"
            } else if word.contains('/') || word.starts_with('@') {
                "string"
            } else {
                ""
            };
            (end, kind)
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            let end = line
                .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .unwrap_or(line.len());
            let kind = match &line[..end] {
                "use" | "let" | "mut" | "fn" | "pub" | "move" | "if" | "else" | "match"
                | "impl" | "crate" | "self" | "Self" | "struct" | "enum" | "mod" | "trait"
                | "type" | "where" | "for" | "in" | "while" | "loop" | "break" | "continue"
                | "return" | "const" | "static" | "async" | "await" | "dyn" | "ref" | "as"
                | "super" | "unsafe" | "extern" => "keyword",
                "true" | "false" => "number",
                _ if ch.is_ascii_digit() => "number",
                _ if ch.is_ascii_uppercase() || line[end..].starts_with('(') => "tag",
                _ => "",
            };
            (end, kind)
        } else {
            (ch.len_utf8(), "")
        };
        let token = escape(&line[..end]);
        if kind.is_empty() {
            result.push_str(&token);
        } else {
            write!(result, r#"<span class="syn-{kind}">{token}</span>"#).unwrap();
        }
        line = &line[end..];
    }
    result
}

pub fn highlight(source: &str, shell: bool) -> String {
    source
        .lines()
        .map(|line| {
            format!(
                r#"<span class="line">{}</span>"#,
                highlighted_line(line, shell)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetched_rust_source_is_highlighted_without_becoming_html() {
        let html = highlight(
            "pub struct Button;\nlet label = \"<script>alert(1)</script>\"; // comment",
            false,
        );
        assert!(html.contains(r#"class="syn-keyword">struct</span>"#));
        assert!(html.contains(r#"class="syn-tag">Button</span>"#));
        assert!(html.contains(r#"class="syn-string">&quot;&lt;script&gt;"#));
        assert!(html.contains(r#"class="syn-comment">// comment</span>"#));
        assert!(!html.contains("<script>"));
        assert_eq!(html.matches(r#"class="line""#).count(), 2);
    }

    #[test]
    fn shell_commands_keep_flags_paths_variables_and_comments_distinct() {
        let html = highlight(
            "cargo add kense-kui --rename kui\ncurl https://example.test/r.json | sh\ncd $CARGO_TARGET_DIR # output\nprintf '<script>'",
            true,
        );
        for token in [
            r#"class="syn-tag">cargo</span>"#,
            r#"class="syn-keyword">--rename</span>"#,
            r#"class="syn-string">https://example.test/r.json</span>"#,
            r#"class="syn-tag">sh</span>"#,
            r#"class="syn-number">$CARGO_TARGET_DIR</span>"#,
            r#"class="syn-comment"># output</span>"#,
        ] {
            assert!(html.contains(token), "missing shell token: {token}");
        }
        assert!(!html.contains("<script>"));
    }
}
