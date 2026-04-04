use iced::advanced::text as core_text;
use iced::time::Instant;
use iced::widget::{checkbox, column, container, markdown, rich_text, row, rule, scrollable, text};
use iced::{alignment, Element, Font, Length};

use crate::animation::AnimationState;
use crate::content::StreamContent;
use crate::custom_renderer::CustomRendererRegistry;
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
    stream_view_with_renderers(content, animation, settings, now, None)
}

/// Like [`stream_view`], but with optional custom code block renderers.
///
/// When a [`CustomRendererRegistry`] is provided, code blocks are checked
/// against registered renderers before falling back to the default display.
pub fn stream_view_with_renderers<'a, Theme, Renderer>(
    content: &'a StreamContent,
    animation: &AnimationState,
    settings: &StreamSettings,
    now: Instant,
    custom_renderers: Option<&'a CustomRendererRegistry<markdown::Uri, Theme, Renderer>>,
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

    // Words below the watermark have finished animating (opacity = 1.0) and
    // can use standard rendering, avoiding per-word String allocations.
    let watermark = animation.fully_revealed_watermark(now);

    let blocks = items.iter().enumerate().map(|(i, item)| {
        let is_last = i + 1 == items.len();
        let needs_animation = is_streaming && i >= changed_from;

        // Check for custom code block renderer.
        if let Some(registry) = custom_renderers {
            if let markdown::Item::CodeBlock { language, code, .. } = item {
                if let Some(lang) = language {
                    if let Some(element) = registry.try_render(
                        lang,
                        code,
                        is_streaming && is_last,
                    ) {
                        return element;
                    }
                }
            }
        }

        if needs_animation {
            let global_offset = content.global_word_offset(i);
            let block_end = content.global_word_offset(i + 1);

            // Skip per-word animation for blocks where all words are fully
            // revealed — use standard rendering instead (no String allocations).
            if block_end <= watermark && !is_last {
                return markdown::item(
                    &DefaultViewer,
                    settings.markdown,
                    item,
                    i,
                );
            }

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
        markdown::Item::CodeBlock { lines, .. } => animated_code_block(
            settings,
            animation,
            global_word_offset,
            now,
            lines,
        ),
        markdown::Item::List { start, bullets } => animated_list(
            settings,
            animation,
            global_word_offset,
            now,
            show_caret,
            *start,
            bullets,
        ),
        markdown::Item::Quote(items) => animated_quote(
            settings,
            animation,
            global_word_offset,
            now,
            show_caret,
            items,
        ),
        // For other item types (Image, Rule, Table), fall back to standard rendering.
        other => markdown::item(
            &DefaultViewer,
            settings.markdown,
            other,
            _index,
        ),
    }
}

/// Renders a code block with line-level animation.
fn animated_code_block<'a, Theme, Renderer>(
    settings: &StreamSettings,
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    lines: &'a [markdown::Text],
) -> Element<'a, markdown::Uri, Theme, Renderer>
where
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    let md = settings.markdown;
    let mut word_offset = global_word_offset;

    let animated_lines = lines.iter().map(|line| {
        let base_spans = line.spans(md.style);
        let animated =
            viewer::animate_spans_no_caret(&base_spans, animation, word_offset, now, settings);
        // Count words in this line for offset tracking.
        let line_words: usize = base_spans
            .iter()
            .map(|s| {
                let t: &str = s.text.as_ref();
                t.split_whitespace().count()
            })
            .sum();
        word_offset += line_words;

        rich_text(animated)
            .on_link_click(std::convert::identity)
            .font(md.style.code_block_font)
            .size(md.code_size)
            .into()
    });

    container(
        scrollable(
            container(column(animated_lines)).padding(md.code_size),
        )
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
                .width(md.code_size / 2)
                .scroller_width(md.code_size / 2),
        )),
    )
    .width(Length::Fill)
    .padding(md.code_size / 4)
    .class(Theme::code_block())
    .into()
}

/// Renders a list with animated text within each bullet.
fn animated_list<'a, Theme, Renderer>(
    settings: &StreamSettings,
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    show_caret: bool,
    start: Option<u64>,
    bullets: &'a [markdown::Bullet],
) -> Element<'a, markdown::Uri, Theme, Renderer>
where
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    let md = settings.markdown;
    let mut word_offset = global_word_offset;

    let is_ordered = start.is_some();
    let start_num = start.unwrap_or(1);
    let digits = (start_num + bullets.len().saturating_sub(1) as u64)
        .max(1)
        .ilog10()
        + 1;

    let elements = bullets.iter().enumerate().map(|(i, bullet)| {
        let is_last_bullet = i + 1 == bullets.len();

        let marker: Element<'a, markdown::Uri, Theme, Renderer> = if is_ordered {
            text!("{}.", i as u64 + start_num)
                .size(md.text_size)
                .align_x(alignment::Horizontal::Right)
                .width(md.text_size * ((digits as f32 / 2.0).ceil() + 1.0))
                .into()
        } else {
            match bullet {
                markdown::Bullet::Point { .. } => text("•").size(md.text_size).into(),
                markdown::Bullet::Task { done, .. } => Element::from(
                    container(checkbox(*done).size(md.text_size))
                        .center_y(text::LineHeight::default().to_absolute(md.text_size)),
                ),
            }
        };

        let items = match bullet {
            markdown::Bullet::Point { items } | markdown::Bullet::Task { items, .. } => items,
        };
        let mut inner_settings = settings.clone();
        inner_settings.markdown.spacing = md.spacing * 0.6;

        let inner = column(items.iter().enumerate().map(|(j, sub_item)| {
            let is_last_in_bullet = j + 1 == items.len() && is_last_bullet;
            let elem = render_animated_item(
                sub_item,
                &inner_settings,
                animation,
                word_offset,
                now,
                show_caret && is_last_in_bullet,
                0,
            );
            word_offset += crate::content::count_words_in_item(sub_item);
            elem
        }))
        .spacing(inner_settings.markdown.spacing);

        row![marker, inner].spacing(md.spacing).into()
    });

    let col = column(elements).spacing(md.spacing * 0.75);

    if is_ordered {
        col.into()
    } else {
        col.padding([0.0, md.spacing.0]).into()
    }
}

/// Renders a blockquote with animated inner content.
fn animated_quote<'a, Theme, Renderer>(
    settings: &StreamSettings,
    animation: &AnimationState,
    global_word_offset: usize,
    now: Instant,
    show_caret: bool,
    items: &'a [markdown::Item],
) -> Element<'a, markdown::Uri, Theme, Renderer>
where
    Theme: markdown::Catalog + 'a,
    Renderer: core_text::Renderer<Font = Font> + 'a,
{
    let md = settings.markdown;
    let mut word_offset = global_word_offset;

    let inner = column(items.iter().enumerate().map(|(i, sub_item)| {
        let is_last = i + 1 == items.len();
        let elem = render_animated_item(
            sub_item,
            settings,
            animation,
            word_offset,
            now,
            show_caret && is_last,
            0,
        );
        word_offset += crate::content::count_words_in_item(sub_item);
        elem
    }))
    .spacing(md.spacing.0);

    row![rule::vertical(4), inner]
        .height(Length::Shrink)
        .spacing(md.spacing.0)
        .into()
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
