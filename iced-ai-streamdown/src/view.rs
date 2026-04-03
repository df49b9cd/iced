use iced::advanced::text as core_text;
use iced::time::Instant;
use iced::widget::{column, markdown};
use iced::{Element, Font};

use crate::animation::AnimationState;
use crate::content::StreamContent;
use crate::settings::StreamSettings;
use crate::viewer;

/// Renders streaming markdown content with word-level animation and a caret.
///
/// Stable (non-changing) blocks are rendered via the standard markdown
/// renderer. Only the actively-changing blocks get the animation treatment.
///
/// # Usage
///
/// ```ignore
/// stream_view(&content, &animation, settings, now)
///     .map(Message::LinkClicked)
/// ```
pub fn stream_view<'a, Theme, Renderer>(
    content: &'a StreamContent,
    animation: &AnimationState,
    settings: &StreamSettings,
    now: Instant,
) -> Element<'a, markdown::Uri, Theme, Renderer>
where
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    let items = content.items();
    let is_streaming = content.is_streaming();
    let previous_count = content.previous_item_count();

    // The index from which blocks may have changed.
    let changed_from = if is_streaming && previous_count > 0 {
        previous_count - 1
    } else if is_streaming {
        0
    } else {
        items.len() // nothing to animate
    };

    let blocks = items.iter().enumerate().map(|(i, item)| {
        let is_last = i + 1 == items.len();
        let needs_animation = is_streaming && i >= changed_from;

        if needs_animation {
            let global_offset = content.global_word_offset(i);
            let show_caret = is_last;

            render_animated_item(
                item,
                settings,
                animation,
                global_offset,
                now,
                show_caret,
                i,
            )
        } else {
            // Stable block — use standard markdown rendering.
            markdown::item(
                &DefaultViewer,
                settings.markdown,
                item,
                i,
            )
        }
    });

    Element::new(column(blocks).spacing(settings.markdown.spacing))
}

/// Renders a single item with animation applied to its text content.
fn render_animated_item<'a, Theme, Renderer>(
    item: &'a markdown::Item,
    settings: &StreamSettings,
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    show_caret: bool,
    _index: usize,
) -> Element<'a, markdown::Uri, Theme, Renderer>
where
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    match item {
        markdown::Item::Paragraph(text) => viewer::animated_paragraph(
            text,
            settings,
            animation,
            global_word_offset,
            now,
            show_caret,
            std::convert::identity,
        ),
        markdown::Item::Heading(level, text) => viewer::animated_heading(
            text,
            level,
            settings,
            animation,
            global_word_offset,
            now,
            show_caret,
            std::convert::identity,
        ),
        // For other item types, fall back to standard rendering.
        // Animation on code blocks, lists, tables, etc. would require
        // more complex treatment that can be added incrementally.
        other => markdown::item(
            &DefaultViewer,
            settings.markdown,
            other,
            _index,
        ),
    }
}

/// A default viewer that passes link URIs through unchanged.
#[derive(Debug, Clone, Copy)]
struct DefaultViewer;

impl<'a, Theme, Renderer> markdown::Viewer<'a, markdown::Uri, Theme, Renderer> for DefaultViewer
where
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    fn on_link_click(url: markdown::Uri) -> markdown::Uri {
        url
    }
}
