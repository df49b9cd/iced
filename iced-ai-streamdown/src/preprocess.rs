//! Markdown preprocessing for custom and literal HTML tags.
//!
//! Ported from Streamdown's `preprocess-custom-tags.ts` and
//! `preprocess-literal-tag-content.ts`.

use std::borrow::Cow;

/// Preprocesses custom HTML tags to prevent blank lines within them from
/// causing CommonMark to split the block.
///
/// For each registered tag name, replaces `\n\n` inside the tag with
/// `\n<!---->\n` (HTML comment that acts as a spacer without splitting).
pub fn preprocess_custom_tags<'a>(markdown: &'a str, tag_names: &[&str]) -> Cow<'a, str> {
    if tag_names.is_empty() || markdown.is_empty() {
        return Cow::Borrowed(markdown);
    }

    let mut result = markdown.to_owned();
    let mut changed = false;

    for &tag_name in tag_names {
        let mut search_from = 0;
        loop {
            // Find opening tag (case-insensitive).
            let open_tag_start = match find_tag_open(&result[search_from..], tag_name) {
                Some(pos) => search_from + pos,
                None => break,
            };

            // Find end of opening tag.
            let open_tag_end = match result[open_tag_start..].find('>') {
                Some(pos) => open_tag_start + pos + 1,
                None => break,
            };

            // Find closing tag.
            let close_pattern = format!("</{}", tag_name);
            let close_tag_start =
                match find_case_insensitive(&result[open_tag_end..], &close_pattern) {
                    Some(pos) => open_tag_end + pos,
                    None => break,
                };

            let close_tag_end = match result[close_tag_start..].find('>') {
                Some(pos) => close_tag_start + pos + 1,
                None => break,
            };

            // Check if content between tags has blank lines.
            let has_blank_lines = result[open_tag_end..close_tag_start].contains("\n\n");
            if has_blank_lines {
                let content = result[open_tag_end..close_tag_start].to_owned();
                let fixed_content = content.replace("\n\n", "\n<!---->\n");
                let close_tag = result[close_tag_start..close_tag_end].to_owned();
                let after = result[close_tag_end..].to_owned();
                // Pad with newlines if needed.
                let padded = format!(
                    "{}{}{}{}{}{}",
                    &result[..open_tag_end],
                    if !fixed_content.starts_with('\n') {
                        "\n"
                    } else {
                        ""
                    },
                    fixed_content,
                    if !fixed_content.ends_with('\n') {
                        "\n"
                    } else {
                        ""
                    },
                    close_tag,
                    "\n\n",
                );
                search_from = padded.len() + after.len();
                result = format!("{}{}", padded, after);
                changed = true;
            } else {
                search_from = close_tag_end;
            }
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(markdown)
    }
}

/// Escapes markdown metacharacters inside specified HTML tags so their content
/// renders as plain text.
pub fn preprocess_literal_tag_content<'a>(markdown: &'a str, tag_names: &[&str]) -> Cow<'a, str> {
    if tag_names.is_empty() || markdown.is_empty() {
        return Cow::Borrowed(markdown);
    }

    let mut result = markdown.to_owned();
    let mut changed = false;

    for &tag_name in tag_names {
        let mut search_from = 0;
        loop {
            let open_tag_start = match find_tag_open(&result[search_from..], tag_name) {
                Some(pos) => search_from + pos,
                None => break,
            };

            let open_tag_end = match result[open_tag_start..].find('>') {
                Some(pos) => open_tag_start + pos + 1,
                None => break,
            };

            let close_pattern = format!("</{}", tag_name);
            let close_tag_start =
                match find_case_insensitive(&result[open_tag_end..], &close_pattern) {
                    Some(pos) => open_tag_end + pos,
                    None => break,
                };

            let content = &result[open_tag_end..close_tag_start];
            let escaped = escape_markdown(content);

            if escaped != content {
                let new_result = format!(
                    "{}{}{}",
                    &result[..open_tag_end],
                    escaped,
                    &result[close_tag_start..]
                );
                search_from = open_tag_end + escaped.len();
                result = new_result;
                changed = true;
            } else {
                search_from = close_tag_start + close_pattern.len();
            }
        }
    }

    if changed {
        Cow::Owned(result)
    } else {
        Cow::Borrowed(markdown)
    }
}

/// Escapes markdown metacharacters: `\`, `` ` ``, `*`, `_`, `~`, `[`, `]`, `|`.
/// Also replaces `\n\n` with `&#10;&#10;` to preserve blank lines.
fn escape_markdown(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 4);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Replace \n\n with &#10;&#10;
        if bytes[i] == b'\n' && i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
            out.push_str("&#10;&#10;");
            i += 2;
            continue;
        }
        match bytes[i] {
            b'\\' | b'`' | b'*' | b'_' | b'~' | b'[' | b']' | b'|' => {
                out.push('\\');
                out.push(bytes[i] as char);
            }
            _ => out.push(bytes[i] as char),
        }
        i += 1;
    }
    out
}

/// Case-insensitive search for an opening tag like `<tagname` followed by whitespace, `/`, or `>`.
fn find_tag_open(haystack: &str, tag_name: &str) -> Option<usize> {
    let haystack_lower = haystack.to_ascii_lowercase();
    let pattern = format!("<{}", tag_name.to_ascii_lowercase());
    let mut search_from = 0;
    loop {
        let pos = haystack_lower[search_from..].find(&pattern)?;
        let abs_pos = search_from + pos;
        let after_tag = abs_pos + pattern.len();
        if after_tag >= haystack.len() {
            return Some(abs_pos);
        }
        let next = haystack.as_bytes()[after_tag];
        if matches!(next, b' ' | b'\t' | b'\n' | b'>' | b'/') {
            return Some(abs_pos);
        }
        search_from = abs_pos + 1;
    }
}

/// Case-insensitive search for a string.
fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.to_ascii_lowercase();
    let n = needle.to_ascii_lowercase();
    let mut search_from = 0;
    loop {
        let pos = h[search_from..].find(&n)?;
        let abs_pos = search_from + pos;
        // For closing tags, check that next char is whitespace or >
        let after = abs_pos + n.len();
        if after >= haystack.len() {
            return Some(abs_pos);
        }
        let next = haystack.as_bytes()[after];
        if matches!(next, b' ' | b'\t' | b'\n' | b'>') {
            return Some(abs_pos);
        }
        search_from = abs_pos + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_tags_replaces_blank_lines() {
        let input = "<custom>\nfoo\n\nbar\n</custom>";
        let result = preprocess_custom_tags(input, &["custom"]);
        assert!(result.contains("<!---->")); // blank line replaced
        assert!(!result.contains("\n\n</custom>")); // no blank line before close
    }

    #[test]
    fn custom_tags_no_blank_lines_unchanged() {
        let input = "<custom>\nfoo\nbar\n</custom>";
        let result = preprocess_custom_tags(input, &["custom"]);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn custom_tags_empty_list() {
        let input = "<custom>\nfoo\n\nbar\n</custom>";
        assert!(matches!(
            preprocess_custom_tags(input, &[]),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn literal_tags_escapes_markdown() {
        let input = "<literal>**bold** and `code`</literal>";
        let result = preprocess_literal_tag_content(input, &["literal"]);
        assert!(result.contains("\\*\\*bold\\*\\*"));
        assert!(result.contains("\\`code\\`"));
    }

    #[test]
    fn literal_tags_preserves_blank_lines() {
        let input = "<literal>foo\n\nbar</literal>";
        let result = preprocess_literal_tag_content(input, &["literal"]);
        assert!(result.contains("&#10;&#10;"));
    }

    #[test]
    fn literal_tags_no_special_chars_unchanged() {
        let input = "<literal>plain text</literal>";
        let result = preprocess_literal_tag_content(input, &["literal"]);
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn escape_markdown_all_chars() {
        assert_eq!(
            escape_markdown("\\`*_~[]|"),
            "\\\\\\`\\*\\_\\~\\[\\]\\|"
        );
    }
}
