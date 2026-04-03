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
            viewer::animate_spans_no_caret(&base_spans, animation, word_offset, now);
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

    let bullet_elements = bullets.iter().enumerate().map(|(i, bullet)| {
        let is_last_bullet = i + 1 == bullets.len();

        let marker: Element<'a, markdown::Uri, Theme, Renderer> = match bullet {
            markdown::Bullet::Point { .. } => text("•").size(md.text_size).into(),
            markdown::Bullet::Task { done, .. } => Element::from(
                container(checkbox(*done).size(md.text_size))
                    .center_y(text::LineHeight::default().to_absolute(md.text_size)),
            ),
        };

        let items = match bullet {
                markdown::Bullet::Point { items } | markdown::Bullet::Task { items, .. } => items,
            };
        let inner_settings = StreamSettings {
            markdown: markdown::Settings {
                spacing: md.spacing * 0.6,
                ..md
            },
            ..*settings
        };

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
            // Advance word offset by the words in this sub-item.
            word_offset += crate::content::count_words_in_item_public(sub_item);
            elem
        }))
        .spacing(inner_settings.markdown.spacing);

        row![marker, inner].spacing(md.spacing).into()
    });

    if start.is_some() {
        // Ordered list with numbers.
        let start_num = start.unwrap_or(1);
        let digits = (start_num + bullets.len() as u64).max(1).ilog10() + 1;
        let _ = digits; // used for width calculation in the standard renderer

        column(bullets.iter().enumerate().map(|(i, bullet)| {
            let is_last_bullet = i + 1 == bullets.len();

            let items = match bullet {
                markdown::Bullet::Point { items } | markdown::Bullet::Task { items, .. } => items,
            };
            let inner_settings = StreamSettings {
                markdown: markdown::Settings {
                    spacing: md.spacing * 0.6,
                    ..md
                },
                ..*settings
            };

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
                word_offset += crate::content::count_words_in_item_public(sub_item);
                elem
            }))
            .spacing(inner_settings.markdown.spacing);

            row![
                text!("{}.", i as u64 + start_num)
                    .size(md.text_size)
                    .align_x(alignment::Horizontal::Right),
                inner
            ]
            .spacing(md.spacing)
            .into()
        }))
        .spacing(md.spacing * 0.75)
        .into()
    } else {
        column(bullet_elements)
            .spacing(md.spacing * 0.75)
            .padding([0.0, md.spacing.0])
            .into()
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
        word_offset += crate::content::count_words_in_item_public(sub_item);
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
