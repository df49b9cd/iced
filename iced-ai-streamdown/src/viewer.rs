use iced::advanced::text as core_text;
use iced::time::Instant;
use iced::widget::{markdown, rich_text, text};
use iced::{Color, Element, Font};

use crate::animation::AnimationState;
use crate::caret;
use crate::settings::StreamSettings;

/// Creates an animated paragraph element where each word fades in based on
/// animation state.
pub fn animated_paragraph<'a, Message, Theme, Renderer>(
    md_text: &markdown::Text,
    settings: &StreamSettings,
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    show_caret: bool,
    on_link_click: impl Fn(markdown::Uri) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    let base_spans = md_text.spans(settings.markdown.style);
    let animated = animate_spans(
        &base_spans,
        animation,
        global_word_offset,
        now,
        show_caret.then_some(settings),
    );

    rich_text(animated)
        .size(settings.markdown.text_size)
        .on_link_click(on_link_click)
        .into()
}

/// Creates an animated heading element.
pub fn animated_heading<'a, Message, Theme, Renderer>(
    md_text: &markdown::Text,
    level: &iced::widget::markdown::HeadingLevel,
    settings: &StreamSettings,
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    show_caret: bool,
    on_link_click: impl Fn(markdown::Uri) -> Message + 'a,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    let base_spans = md_text.spans(settings.markdown.style);
    let animated = animate_spans(
        &base_spans,
        animation,
        global_word_offset,
        now,
        show_caret.then_some(settings),
    );

    let size = match level {
        markdown::HeadingLevel::H1 => settings.markdown.h1_size,
        markdown::HeadingLevel::H2 => settings.markdown.h2_size,
        markdown::HeadingLevel::H3 => settings.markdown.h3_size,
        markdown::HeadingLevel::H4 => settings.markdown.h4_size,
        markdown::HeadingLevel::H5 => settings.markdown.h5_size,
        markdown::HeadingLevel::H6 => settings.markdown.h6_size,
    };

    rich_text(animated)
        .size(size)
        .on_link_click(on_link_click)
        .into()
}

/// Splits text spans into per-word sub-spans with animation opacity (no caret).
///
/// Used by code blocks and other contexts where the caret should not appear.
pub fn animate_spans_no_caret(
    base_spans: &[text::Span<'static, markdown::Uri>],
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
) -> Vec<text::Span<'static, markdown::Uri>> {
    animate_spans(base_spans, animation, global_word_offset, now, None)
}

/// Splits text spans into per-word sub-spans with animation opacity applied.
fn animate_spans(
    base_spans: &[text::Span<'static, markdown::Uri>],
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    caret_settings: Option<&StreamSettings>,
) -> Vec<text::Span<'static, markdown::Uri>> {
    let mut animated = Vec::new();
    let mut word_idx = global_word_offset;

    for span in base_spans {
        let text_content: &str = span.text.as_ref();

        if text_content.is_empty() {
            animated.push(span.clone());
            continue;
        }

        // Split the span's text into words, preserving whitespace as separate spans.
        let mut chars = text_content.char_indices().peekable();
        while chars.peek().is_some() {
            // Collect leading whitespace.
            let ws_start = chars.peek().map(|(i, _)| *i).unwrap_or(text_content.len());
            while chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
                chars.next();
            }
            let ws_end = chars.peek().map(|(i, _)| *i).unwrap_or(text_content.len());
            if ws_end > ws_start {
                let ws = &text_content[ws_start..ws_end];
                // Whitespace inherits the opacity of the preceding word.
                let opacity = if word_idx > global_word_offset {
                    animation.word_opacity(word_idx - 1, now)
                } else {
                    animation.word_opacity(word_idx, now)
                };
                animated.push(make_animated_span(span, ws.to_owned(), opacity));
            }

            // Collect the word.
            let word_start = chars.peek().map(|(i, _)| *i).unwrap_or(text_content.len());
            while chars.peek().is_some_and(|(_, c)| !c.is_whitespace()) {
                chars.next();
            }
            let word_end = chars.peek().map(|(i, _)| *i).unwrap_or(text_content.len());
            if word_end > word_start {
                let word = &text_content[word_start..word_end];
                let opacity = animation.word_opacity(word_idx, now);
                animated.push(make_animated_span(span, word.to_owned(), opacity));
                word_idx += 1;
            }
        }
    }

    // Append caret if requested.
    if let Some(settings) = caret_settings
        && let Some(caret_kind) = settings.caret
    {
        let caret_color = settings.caret_color.unwrap_or(Color::WHITE);

        animated.push(caret::caret_span(
            caret_kind,
            caret_color,
            now,
            settings.caret_blink_interval,
        ));
    }

    animated
}

/// Creates a clone of the given span with different text and opacity applied.
fn make_animated_span(
    base: &text::Span<'static, markdown::Uri>,
    text_content: String,
    opacity: f32,
) -> text::Span<'static, markdown::Uri> {
    let color = base.color.unwrap_or(Color::WHITE);
    let animated_color = Color {
        a: color.a * opacity,
        ..color
    };

    text::Span {
        text: text_content.into(),
        size: base.size,
        line_height: base.line_height,
        font: base.font,
        color: Some(animated_color),
        link: base.link.clone(),
        highlight: base.highlight,
        padding: base.padding,
        underline: base.underline,
        strikethrough: base.strikethrough,
    }
}
