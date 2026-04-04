use std::borrow::Cow;

/// Returns `true` if `ch` is a Unicode letter, digit, or underscore.
/// Matches the TS `isWordChar` / `[\p{L}\p{N}_]`.
pub fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// Returns `true` if `s` contains only whitespace and emphasis marker characters
/// (`_`, `~`, `*`, `` ` ``). Matches the TS `whitespaceOrMarkersPattern`.
pub fn is_empty_or_markers(s: &str) -> bool {
    s.bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'_' | b'~' | b'*' | b'`'))
}

/// Returns `true` if the line is a list item marker line (e.g. `  - `, `  * `, `  + `).
pub fn is_list_marker_line(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    // Skip leading whitespace.
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    // Expect one of `-`, `*`, `+`.
    if i >= bytes.len() || !matches!(bytes[i], b'-' | b'*' | b'+') {
        return false;
    }
    i += 1;
    // Must be followed by at least one space or tab, then only whitespace.
    if i >= bytes.len() || !matches!(bytes[i], b' ' | b'\t') {
        return false;
    }
    bytes[i..].iter().all(|&b| matches!(b, b' ' | b'\t'))
}

/// Returns `true` if the position is inside a fenced code block (between ``` markers)
/// or an inline code span (between `` ` `` markers).
pub fn is_inside_code_block(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();
    let mut in_code_block = false;
    let mut in_inline_code = false;
    let mut i = 0;

    while i < position && i < bytes.len() {
        // Skip escaped backticks.
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'`' {
            i += 2;
            continue;
        }
        // Check for triple backticks (multiline code blocks).
        if i + 2 < bytes.len() && bytes[i] == b'`' && bytes[i + 1] == b'`' && bytes[i + 2] == b'`'
        {
            in_code_block = !in_code_block;
            i += 3;
            continue;
        }
        // Check for triple tildes (CommonMark also allows ~~~ fences).
        if i + 2 < bytes.len() && bytes[i] == b'~' && bytes[i + 1] == b'~' && bytes[i + 2] == b'~'
        {
            in_code_block = !in_code_block;
            i += 3;
            continue;
        }
        // Only check for inline code if not in multiline code.
        if !in_code_block && bytes[i] == b'`' {
            in_inline_code = !in_inline_code;
        }
        i += 1;
    }

    in_inline_code || in_code_block
}

/// Returns `true` if the position is inside a *complete* inline code span
/// (both opening and closing backtick present). Returns `false` for incomplete
/// spans so emphasis markers can still be completed during streaming.
pub fn is_within_complete_inline_code(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();
    let mut in_inline_code = false;
    let mut in_multiline_code = false;
    let mut inline_code_start: Option<usize> = None;
    let mut i = 0;

    while i < bytes.len() {
        // Skip escaped backticks.
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'`' {
            i += 2;
            continue;
        }
        // Check for triple backticks.
        if i + 2 < bytes.len() && bytes[i] == b'`' && bytes[i + 1] == b'`' && bytes[i + 2] == b'`'
        {
            in_multiline_code = !in_multiline_code;
            i += 3;
            continue;
        }
        // Only check for inline code if not in multiline code.
        if !in_multiline_code && bytes[i] == b'`' {
            if in_inline_code {
                // Found closing backtick — check if position is inside this complete span.
                if let Some(start) = inline_code_start
                    && start < position
                    && position < i
                {
                    return true;
                }
                in_inline_code = false;
                inline_code_start = None;
            } else {
                in_inline_code = true;
                inline_code_start = Some(i);
            }
        }
        i += 1;
    }

    false
}

/// Returns `true` if the backtick at `pos` is part of a ``` sequence.
pub fn is_part_of_triple_backtick(text: &str, pos: usize) -> bool {
    let bytes = text.as_bytes();
    let len = bytes.len();

    // Check: is this the start of ```?
    if pos + 2 < len && bytes[pos] == b'`' && bytes[pos + 1] == b'`' && bytes[pos + 2] == b'`' {
        return true;
    }
    // Check: is this the middle of ```?
    if pos >= 1
        && pos + 1 < len
        && bytes[pos - 1] == b'`'
        && bytes[pos] == b'`'
        && bytes[pos + 1] == b'`'
    {
        return true;
    }
    // Check: is this the end of ```?
    if pos >= 2 && bytes[pos - 2] == b'`' && bytes[pos - 1] == b'`' && bytes[pos] == b'`' {
        return true;
    }
    false
}

/// Counts single backticks that are not part of triple backticks or escaped.
pub fn count_single_backticks(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i < bytes.len() {
        // Skip escaped backticks.
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'`' {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' && !is_part_of_triple_backtick(text, i) {
            count += 1;
        }
        i += 1;
    }
    count
}

/// Returns `true` if `position` is inside a math block (`$` or `$$`).
pub fn is_within_math_block(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();
    let mut in_inline_math = false;
    let mut in_block_math = false;
    let mut i = 0;

    while i < bytes.len() && i < position {
        // Skip escaped dollar signs.
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'$' {
            i += 2;
            continue;
        }
        if bytes[i] == b'$' {
            // Check for block math ($$).
            if i + 1 < bytes.len() && bytes[i + 1] == b'$' {
                in_block_math = !in_block_math;
                i += 2;
                in_inline_math = false; // Block math takes precedence.
                continue;
            } else if !in_block_math {
                // Only toggle inline math if not in block math.
                in_inline_math = !in_inline_math;
            }
        }
        i += 1;
    }

    in_inline_math || in_block_math
}

/// Returns `true` if `position` is inside the URL portion of a link/image `](url)`.
pub fn is_within_link_or_image_url(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();

    // Search backwards from position for `(` preceded by `]`.
    let mut i = position.saturating_sub(1);
    loop {
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b')' => return false,
            b'(' => {
                if i > 0 && bytes[i - 1] == b']' {
                    // We're potentially inside a link/image URL.
                    // Check if we're before the closing `)`.
                    for &b in &bytes[position..] {
                        if b == b')' {
                            return true;
                        }
                        if b == b'\n' {
                            return false;
                        }
                    }
                }
                return false;
            }
            b'\n' => return false,
            _ => {}
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }

    false
}

/// Returns `true` if `position` is inside an HTML tag (between `<` and `>`).
pub fn is_within_html_tag(text: &str, position: usize) -> bool {
    let bytes = text.as_bytes();

    let mut i = position.saturating_sub(1);
    loop {
        if i >= bytes.len() {
            break;
        }
        match bytes[i] {
            b'>' => return false,
            b'<' => {
                // Check that it starts a valid tag (followed by letter or /).
                let next = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
                if next.is_ascii_alphabetic() || next == b'/' {
                    return true;
                }
                return false;
            }
            b'\n' => return false,
            _ => {}
        }
        if i == 0 {
            break;
        }
        i -= 1;
    }

    false
}

/// Returns `true` if the marker at `marker_index` is on a line that forms a
/// horizontal rule (3+ of the same marker with optional spaces, nothing else).
pub fn is_horizontal_rule(text: &str, marker_index: usize, marker: u8) -> bool {
    let bytes = text.as_bytes();

    // Find line start.
    let line_start = bytes[..marker_index]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|p| p + 1)
        .unwrap_or(0);

    // Find line end.
    let line_end = bytes[marker_index..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| marker_index + p)
        .unwrap_or(bytes.len());

    let line = &bytes[line_start..line_end];

    let mut marker_count = 0;
    let mut has_other = false;

    for &b in line {
        if b == marker {
            marker_count += 1;
        } else if b != b' ' && b != b'\t' {
            has_other = true;
            break;
        }
    }

    marker_count >= 3 && !has_other
}

/// Finds the matching opening bracket `[` for a closing bracket at `close_index`,
/// handling nested brackets.
pub fn find_matching_opening_bracket(text: &str, close_index: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth: i32 = 1;
    let mut i = close_index;
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b']' => depth += 1,
            b'[' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Finds the matching closing bracket `]` for an opening bracket at `open_index`,
/// handling nested brackets.
pub fn find_matching_closing_bracket(text: &str, open_index: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth: i32 = 1;
    let mut i = open_index + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Helper: make an owned Cow by appending a suffix.
pub(crate) fn cow_append<'a>(text: &str, suffix: &str) -> Cow<'a, str> {
    let mut s = String::with_capacity(text.len() + suffix.len());
    s.push_str(text);
    s.push_str(suffix);
    Cow::Owned(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('0'));
        assert!(is_word_char('_'));
        assert!(!is_word_char(' '));
        assert!(!is_word_char('*'));
        // Unicode
        assert!(is_word_char('é'));
        assert!(is_word_char('中'));
    }

    #[test]
    fn test_is_empty_or_markers() {
        assert!(is_empty_or_markers(""));
        assert!(is_empty_or_markers("  "));
        assert!(is_empty_or_markers("*_~`"));
        assert!(!is_empty_or_markers("hello"));
        assert!(!is_empty_or_markers("*a"));
    }

    #[test]
    fn test_is_inside_code_block() {
        assert!(is_inside_code_block("```code", 5));
        assert!(!is_inside_code_block("```code```after", 12));
        assert!(is_inside_code_block("`code", 3));
        assert!(!is_inside_code_block("`code`after", 8));
    }

    #[test]
    fn test_is_within_math_block() {
        assert!(is_within_math_block("$x+y", 2));
        assert!(!is_within_math_block("$x+y$z", 6));
        assert!(is_within_math_block("$$x+y", 3));
        assert!(!is_within_math_block("\\$x", 2));
    }

    #[test]
    fn test_find_matching_brackets() {
        assert_eq!(find_matching_opening_bracket("[hello]", 6), Some(0));
        assert_eq!(find_matching_closing_bracket("[hello]", 0), Some(6));
        assert_eq!(
            find_matching_opening_bracket("a[b[c]]", 6),
            Some(1)
        );
        assert_eq!(find_matching_opening_bracket("hello]", 5), None);
    }

    #[test]
    fn test_is_horizontal_rule() {
        assert!(is_horizontal_rule("---", 0, b'-'));
        assert!(is_horizontal_rule("***", 0, b'*'));
        assert!(is_horizontal_rule("- - -", 0, b'-'));
        assert!(!is_horizontal_rule("--", 0, b'-'));
        assert!(!is_horizontal_rule("--x", 0, b'-'));
    }

    #[test]
    fn test_count_single_backticks() {
        assert_eq!(count_single_backticks("`hello`"), 2);
        assert_eq!(count_single_backticks("```hello```"), 0);
        assert_eq!(count_single_backticks("`hello"), 1);
        assert_eq!(count_single_backticks("\\`hello"), 0);
    }

    #[test]
    fn test_is_within_link_or_image_url() {
        assert!(is_within_link_or_image_url("[text](http://example.com)", 15));
        assert!(!is_within_link_or_image_url("[text](http://example.com)", 3));
        assert!(!is_within_link_or_image_url("just text", 3));
    }

    #[test]
    fn test_is_within_html_tag() {
        assert!(is_within_html_tag("<a href=\"test\">", 5));
        assert!(!is_within_html_tag("<a href=\"test\">after", 16));
        assert!(!is_within_html_tag("text", 2));
    }
}
