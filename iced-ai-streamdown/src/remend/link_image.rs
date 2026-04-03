use std::borrow::Cow;

use super::options::LinkMode;
use super::utils::{
    find_matching_closing_bracket, find_matching_opening_bracket, is_inside_code_block,
};

/// Handles incomplete URLs in links/images: `[text](partial-url`.
fn handle_incomplete_url(
    text: &str,
    bracket_paren_index: usize,
    link_mode: LinkMode,
) -> Option<Cow<'_, str>> {
    // `bracket_paren_index` points to `]` in `](`.
    let after_paren = &text[bracket_paren_index + 2..];
    if after_paren.contains(')') {
        return None; // URL is complete.
    }

    // Find matching `[` for the `]`.
    let open = find_matching_opening_bracket(text, bracket_paren_index)?;

    if is_inside_code_block(text, open) {
        return None;
    }

    let is_image = open > 0 && text.as_bytes()[open - 1] == b'!';
    let start = if is_image { open - 1 } else { open };
    let before = &text[..start];

    if is_image {
        // Incomplete images are removed entirely (trim trailing whitespace).
        return Some(Cow::Owned(before.trim_end().to_owned()));
    }

    let link_text = &text[open + 1..bracket_paren_index];

    match link_mode {
        LinkMode::TextOnly => {
            // Display only the link text without markup.
            let mut result = String::with_capacity(before.len() + link_text.len());
            result.push_str(before);
            result.push_str(link_text);
            Some(Cow::Owned(result))
        }
        LinkMode::Protocol => {
            // Replace URL with placeholder.
            let mut result = String::with_capacity(before.len() + link_text.len() + 32);
            result.push_str(before);
            result.push('[');
            result.push_str(link_text);
            result.push_str("](streamdown:incomplete-link)");
            Some(Cow::Owned(result))
        }
    }
}

/// Handles incomplete link text: `[partial-text` without closing `]`.
fn handle_incomplete_text(
    text: &str,
    open_index: usize,
    link_mode: LinkMode,
) -> Option<Cow<'_, str>> {
    let is_image = open_index > 0 && text.as_bytes()[open_index - 1] == b'!';
    let start = if is_image { open_index - 1 } else { open_index };

    // Check if there's a closing bracket after this.
    let after = &text[open_index + 1..];
    if !after.contains(']') {
        // Incomplete link/image.
        let before = &text[..start];

        if is_image {
            return Some(Cow::Owned(before.trim_end().to_owned()));
        }

        return Some(make_incomplete_link(text, open_index, link_mode));
    }

    // Check if the closing bracket actually matches (accounting for nesting).
    let closing = find_matching_closing_bracket(text, open_index);
    if closing.is_none() {
        let before = &text[..start];
        if is_image {
            return Some(Cow::Owned(before.trim_end().to_owned()));
        }
        return Some(make_incomplete_link(text, open_index, link_mode));
    }

    None
}

/// Creates the appropriate incomplete link output based on link mode.
fn make_incomplete_link<'a>(text: &str, open_index: usize, link_mode: LinkMode) -> Cow<'a, str> {
    match link_mode {
        LinkMode::TextOnly => {
            // Find the first incomplete `[` and strip just that bracket.
            let mut result = String::with_capacity(text.len());
            result.push_str(&text[..open_index]);
            result.push_str(&text[open_index + 1..]);
            Cow::Owned(result)
        }
        LinkMode::Protocol => {
            let mut result = String::with_capacity(text.len() + 32);
            result.push_str(text);
            result.push_str("](streamdown:incomplete-link)");
            Cow::Owned(result)
        }
    }
}

/// Handles incomplete links and images by auto-completing or removing them.
pub fn handle(text: &str, link_mode: LinkMode) -> Cow<'_, str> {
    let bytes = text.as_bytes();

    // Phase 1: Look for `](` pattern — incomplete URL.
    if let Some(pos) = text.rfind("](")
        && !is_inside_code_block(text, pos)
        && let Some(result) = handle_incomplete_url(text, pos, link_mode)
    {
        return result;
    }

    // Phase 2: Scan backward for unmatched `[`.
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'['
            && !is_inside_code_block(text, i)
            && let Some(result) = handle_incomplete_text(text, i, link_mode)
        {
            return result;
        }
    }

    Cow::Borrowed(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(text: &str) -> Cow<'_, str> {
        handle(text, LinkMode::Protocol)
    }

    fn h_text_only(text: &str) -> Cow<'_, str> {
        handle(text, LinkMode::TextOnly)
    }

    #[test]
    fn completes_incomplete_link_url() {
        assert_eq!(
            h("[Click here](http://exam").as_ref(),
            "[Click here](streamdown:incomplete-link)"
        );
    }

    #[test]
    fn completes_incomplete_link_text() {
        assert_eq!(
            h("[Click here").as_ref(),
            "[Click here](streamdown:incomplete-link)"
        );
    }

    #[test]
    fn removes_incomplete_image() {
        assert_eq!(h("text ![alt](http://").as_ref(), "text");
    }

    #[test]
    fn removes_incomplete_image_text() {
        assert_eq!(h("text ![alt").as_ref(), "text");
    }

    #[test]
    fn leaves_complete_link() {
        assert!(matches!(
            h("[text](http://example.com)"),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn inside_code_block() {
        assert!(matches!(h("```\n[incomplete\n```"), Cow::Borrowed(_)));
    }

    // Text-only mode tests
    #[test]
    fn text_only_incomplete_url() {
        assert_eq!(
            h_text_only("[Click here](http://exam").as_ref(),
            "Click here"
        );
    }

    #[test]
    fn text_only_incomplete_text() {
        assert_eq!(h_text_only("Text [partial").as_ref(), "Text partial");
    }

    #[test]
    fn text_only_complete_unchanged() {
        assert_eq!(
            h_text_only("[text](http://example.com)").as_ref(),
            "[text](http://example.com)"
        );
    }
}
