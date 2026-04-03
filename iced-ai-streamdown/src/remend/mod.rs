//! Streaming markdown preprocessor that auto-completes incomplete syntax.
//!
//! A Rust port of Vercel's [remend](https://github.com/vercel/streamdown/tree/main/packages/remend).
//! Runs on raw markdown strings **before** the pulldown-cmark parser, detecting
//! and closing unterminated formatting markers so content renders correctly
//! during token-by-token streaming.

mod options;
mod utils;

mod comparison_operators;
mod emphasis;
mod html_tags;
mod inline_code;
mod katex;
mod link_image;
mod setext_heading;
mod single_tilde;
mod strikethrough;

pub use options::{LinkMode, RemendOptions};

use std::borrow::Cow;

const INCOMPLETE_LINK_MARKER: &str = "](streamdown:incomplete-link)";

/// Preprocesses streaming markdown text, auto-completing any incomplete syntax.
///
/// Returns `Cow::Borrowed` when no changes are needed (zero-allocation fast path).
pub fn remend<'a>(text: &'a str, options: &RemendOptions) -> Cow<'a, str> {
    if text.is_empty() {
        return Cow::Borrowed(text);
    }

    // Strip trailing single space (preserve double space for line breaks).
    let mut result: Cow<'a, str> = if text.ends_with(' ') && !text.ends_with("  ") {
        Cow::Borrowed(&text[..text.len() - 1])
    } else {
        Cow::Borrowed(text)
    };

    // Fixed-order pipeline matching the TypeScript handler priorities.

    if options.single_tilde {
        result = apply(result, single_tilde::handle);
    }
    if options.comparison_operators {
        result = apply(result, comparison_operators::handle);
    }
    if options.html_tags {
        result = apply(result, html_tags::handle);
    }
    if options.setext_headings {
        result = apply(result, setext_heading::handle);
    }
    if options.links || options.images {
        let link_mode = options.link_mode;
        result = apply_with(result, move |text| link_image::handle(text, link_mode));
        // Early return: prevent further handlers from mangling the synthetic URL.
        // Only applies in Protocol mode (text-only won't end with the marker).
        if result.ends_with(INCOMPLETE_LINK_MARKER) {
            return result;
        }
    }
    if options.bold_italic {
        result = apply(result, emphasis::handle_bold_italic);
    }
    if options.bold {
        result = apply(result, emphasis::handle_bold);
    }
    if options.italic {
        result = apply(result, emphasis::handle_double_underscore);
        result = apply(result, emphasis::handle_italic_asterisk);
        result = apply(result, emphasis::handle_italic_underscore);
    }
    if options.inline_code {
        result = apply(result, inline_code::handle);
    }
    if options.strikethrough {
        result = apply(result, strikethrough::handle);
    }
    if options.katex {
        result = apply(result, katex::handle_block);
    }
    if options.inline_katex {
        result = apply(result, katex::handle_inline);
    }

    result
}

/// Applies a handler to a `Cow<str>`, threading ownership efficiently.
fn apply<'a>(input: Cow<'a, str>, handler: fn(&str) -> Cow<'_, str>) -> Cow<'a, str> {
    apply_with(input, handler)
}

/// Applies a closure handler to a `Cow<str>`, threading ownership efficiently.
fn apply_with<'a>(
    input: Cow<'a, str>,
    handler: impl FnOnce(&str) -> Cow<'_, str>,
) -> Cow<'a, str> {
    match handler(&input) {
        Cow::Borrowed(b) if std::ptr::eq(b, input.as_ref() as &str) => {
            // Handler returned its input unchanged — preserve the original Cow.
            input
        }
        Cow::Borrowed(b) => {
            // Handler returned a borrowed sub-slice (e.g. trimmed) — must own it.
            Cow::Owned(b.to_owned())
        }
        Cow::Owned(s) => Cow::Owned(s),
    }
}

#[cfg(test)]
mod tests;
