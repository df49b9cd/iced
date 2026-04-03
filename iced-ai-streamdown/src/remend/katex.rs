use std::borrow::Cow;

use super::utils::{cow_append, is_part_of_triple_backtick};

/// Counts `$$` pairs outside of inline code blocks.
fn count_dollar_pairs(text: &str) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut pairs = 0;
    let mut in_inline_code = false;
    let mut i = 0;

    while i < len {
        if bytes[i] == b'`' && !is_part_of_triple_backtick(text, i) {
            in_inline_code = !in_inline_code;
            i += 1;
            continue;
        }
        if !in_inline_code && i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'$' {
            pairs += 1;
            i += 2;
            continue;
        }
        i += 1;
    }
    pairs
}

/// Counts single `$` signs (excluding `$$`) outside of code blocks.
fn count_single_dollars(text: &str) -> usize {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut count = 0;
    let mut in_inline_code = false;
    let mut i = 0;

    while i < len {
        // Skip escaped $.
        if bytes[i] == b'\\' && i + 1 < len {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' && !is_part_of_triple_backtick(text, i) {
            in_inline_code = !in_inline_code;
            i += 1;
            continue;
        }
        if !in_inline_code && bytes[i] == b'$' {
            if i + 1 < len && bytes[i + 1] == b'$' {
                i += 2; // Skip $$.
            } else {
                count += 1;
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    count
}

/// Completes incomplete block KaTeX formatting (`$$`).
pub fn handle_block(text: &str) -> Cow<'_, str> {
    let pairs = count_dollar_pairs(text);
    if pairs.is_multiple_of(2) {
        return Cow::Borrowed(text);
    }

    // If text already ends with a single $ (but not $$), just add one more.
    if text.ends_with('$') && !text.ends_with("$$") {
        return cow_append(text, "$");
    }

    // If there's a newline after the opening $$ and text doesn't end with newline,
    // add newline before closing $$.
    if let Some(first_dollar) = text.find("$$") {
        let has_newline_after = text[first_dollar..].contains('\n');
        if has_newline_after && !text.ends_with('\n') {
            return cow_append(text, "\n$$");
        }
    }

    cow_append(text, "$$")
}

/// Completes incomplete inline KaTeX formatting (`$`).
pub fn handle_inline(text: &str) -> Cow<'_, str> {
    let count = count_single_dollars(text);
    if count % 2 == 1 {
        return cow_append(text, "$");
    }
    Cow::Borrowed(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completes_block_katex() {
        assert_eq!(handle_block("$$x + y").as_ref(), "$$x + y$$");
    }

    #[test]
    fn completes_block_katex_multiline() {
        assert_eq!(handle_block("$$\nx + y").as_ref(), "$$\nx + y\n$$");
    }

    #[test]
    fn leaves_complete_block_katex() {
        assert!(matches!(handle_block("$$x + y$$"), Cow::Borrowed(_)));
    }

    #[test]
    fn half_complete_dollar() {
        assert_eq!(handle_block("$$x + y$").as_ref(), "$$x + y$$");
    }

    #[test]
    fn completes_inline_katex() {
        assert_eq!(handle_inline("$x + y").as_ref(), "$x + y$");
    }

    #[test]
    fn leaves_complete_inline_katex() {
        assert!(matches!(handle_inline("$x + y$"), Cow::Borrowed(_)));
    }
}
