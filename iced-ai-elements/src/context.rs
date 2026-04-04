//! Context window usage display component
//!
//! Displays AI model context window usage as a compact trigger (pill with
//! percentage and tiny ring) that expands on hover to a detail card with a
//! progress bar, token breakdown, and cost estimation.

use crate::model::{default_pricing, ModelId, ModelPricing};
use crate::usage::Usage;

use iced::advanced::layout;
use iced::advanced::renderer;
use iced::advanced::widget::tree::{self, Tree};
use iced::advanced::{self, Layout, Shell, Widget};
use iced::mouse;
use iced::widget::canvas::{self, Cache, Path, Stroke};
use iced::{
    alignment, Color, Element, Event, Length, Pixels, Point, Radians, Rectangle, Renderer, Size,
    Vector,
};
use std::cell::Cell;
use std::f32::consts::PI;
use std::marker::PhantomData;

/// Formats a token count with K/M/B suffixes.
pub fn format_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        tokens.to_string()
    } else if tokens < 1_000_000 {
        let v = tokens as f64 / 1_000.0;
        if v == v.round() {
            format!("{}K", v as u64)
        } else {
            format!("{:.1}K", v)
        }
    } else if tokens < 1_000_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else {
        format!("{:.1}B", tokens as f64 / 1_000_000_000.0)
    }
}

/// Formats a cost value as "$X.XX".
pub fn format_cost(cost: f64) -> String {
    if cost < 0.01 {
        format!("${:.4}", cost)
    } else {
        format!("${:.2}", cost)
    }
}

// ============================================================================
// Style & StyleSheet
// ============================================================================

/// The appearance of a [`Context`] widget.
#[derive(Debug, Clone, Copy)]
pub struct Style {
    /// Color for the unfilled portion of the ring/progress bar.
    pub track_color: Color,
    /// Color for the filled portion of the ring/progress bar.
    pub fill_color: Color,
    /// Primary text color.
    pub text_color: Color,
    /// Card and pill background color.
    pub background_color: Color,
    /// Border color for card and pill.
    pub border_color: Color,
    /// Secondary/dimmed text color (labels, token counts).
    pub secondary_text_color: Color,
    /// Background color for the footer section showing total cost.
    pub footer_background_color: Color,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            track_color: Color::from_rgb(0.88, 0.88, 0.88),
            fill_color: Color::from_rgb(0.25, 0.52, 0.96),
            text_color: Color::from_rgb(0.1, 0.1, 0.1),
            background_color: Color::WHITE,
            border_color: Color::from_rgb(0.85, 0.85, 0.85),
            secondary_text_color: Color::from_rgb(0.45, 0.45, 0.45),
            footer_background_color: Color::from_rgb(0.965, 0.965, 0.965),
        }
    }
}

/// Styling catalog for the [`Context`] widget.
pub trait Catalog {
    /// The item class of the [`Catalog`].
    type Class<'a>;

    /// The default class produced by the [`Catalog`].
    fn default<'a>() -> Self::Class<'a>;

    /// The [`Style`] of a class.
    fn style(&self, class: &Self::Class<'_>) -> Style;
}

/// A styling function for a [`Context`].
pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme) -> Style + 'a>;

impl Catalog for iced::Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(|_theme| Style::default())
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

// ============================================================================
// Widget State
// ============================================================================

#[derive(Debug)]
struct State {
    /// Whether the card overlay is visible.
    /// Uses two flags: trigger_hovered and card_hovered. Card shows when either
    /// is true, hides when both are false.
    trigger_hovered: Cell<bool>,
    card_hovered: Cell<bool>,
    ring_cache: Cache,
    cached_style: Cell<Option<Style>>,
    last_percentage: Cell<f32>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            trigger_hovered: Cell::new(false),
            card_hovered: Cell::new(false),
            ring_cache: Cache::default(),
            cached_style: Cell::new(None),
            last_percentage: Cell::new(-1.0),
        }
    }
}

impl State {
    fn is_open(&self) -> bool {
        self.trigger_hovered.get() || self.card_hovered.get()
    }
}

// ============================================================================
// Trigger constants
// ============================================================================

const TRIGGER_HEIGHT: f32 = 32.0;
const TRIGGER_RING_SIZE: f32 = 18.0;
const TRIGGER_PADDING_H: f32 = 10.0;
const TRIGGER_GAP: f32 = 6.0;
const TRIGGER_TEXT_WIDTH: f32 = 40.0;
const TRIGGER_WIDTH: f32 =
    TRIGGER_PADDING_H + TRIGGER_TEXT_WIDTH + TRIGGER_GAP + TRIGGER_RING_SIZE + TRIGGER_PADDING_H;

// ============================================================================
// Context Widget
// ============================================================================

/// A widget displaying AI context window usage.
///
/// Shows a compact pill trigger. On hover, reveals a detail card with progress
/// bar, token breakdown, and cost estimation. The card stays visible while the
/// cursor is over the trigger or the card, and disappears when the cursor
/// leaves both.
pub struct Context<'a, Message, Theme = iced::Theme>
where
    Theme: Catalog,
{
    max_tokens: u64,
    used_tokens: u64,
    usage: Usage,
    model_id: ModelId,
    pricing: Option<ModelPricing>,
    on_hover: Option<Box<dyn Fn(bool) -> Message + 'a>>,
    class: Theme::Class<'a>,
}

impl<'a, Message, Theme> Context<'a, Message, Theme>
where
    Theme: Catalog,
{
    /// Creates a new [`Context`] widget with default values.
    pub fn new() -> Self {
        Self {
            max_tokens: 0,
            used_tokens: 0,
            usage: Usage::default(),
            model_id: ModelId::parse("custom:unknown"),
            pricing: None,
            on_hover: None,
            class: Theme::default(),
        }
    }

    /// Sets the maximum token capacity of the context window.
    pub fn max_tokens(mut self, v: u64) -> Self {
        self.max_tokens = v;
        self
    }
    /// Sets the number of tokens currently used.
    pub fn used_tokens(mut self, v: u64) -> Self {
        self.used_tokens = v;
        self
    }
    /// Sets the token usage breakdown (input, output, reasoning, cached).
    pub fn usage(mut self, v: Usage) -> Self {
        self.usage = v;
        self
    }
    /// Sets the model identifier and auto-populates pricing if not set.
    pub fn model_id(mut self, v: &ModelId) -> Self {
        self.model_id = v.clone();
        if self.pricing.is_none() {
            self.pricing = Some(default_pricing(v));
        }
        self
    }
    /// Overrides the model pricing used for cost estimation.
    pub fn pricing(mut self, v: ModelPricing) -> Self {
        self.pricing = Some(v);
        self
    }
    /// Sets a callback invoked when the hover state changes.
    pub fn on_hover(mut self, f: impl Fn(bool) -> Message + 'a) -> Self {
        self.on_hover = Some(Box::new(f));
        self
    }
    /// Sets the styling class for this widget.
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    fn percentage(&self) -> f32 {
        if self.max_tokens > 0 {
            (self.used_tokens as f32 / self.max_tokens as f32).min(1.0)
        } else {
            0.0
        }
    }
}

impl<'a, Message, Theme> Default for Context<'a, Message, Theme>
where
    Theme: Catalog,
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Widget Trait Implementation
// ============================================================================

impl<'a, Message, Theme> Widget<Message, Theme, Renderer> for Context<'a, Message, Theme>
where
    Message: 'a,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size {
            width: Length::Fixed(TRIGGER_WIDTH),
            height: Length::Fixed(TRIGGER_HEIGHT),
        }
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::atomic(limits, TRIGGER_WIDTH, TRIGGER_HEIGHT)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use advanced::Renderer as _;

        let state = tree.state.downcast_ref::<State>();
        let bounds = layout.bounds();
        let percentage = self.percentage();
        let style = theme.style(&self.class);
        state.cached_style.set(Some(style));

        if (state.last_percentage.get() - percentage).abs() > f32::EPSILON {
            state.ring_cache.clear();
            state.last_percentage.set(percentage);
        }

        // --- Pill background ---
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: iced::Border {
                    color: style.border_color,
                    width: 1.0,
                    radius: (TRIGGER_HEIGHT / 2.0).into(),
                },
                ..renderer::Quad::default()
            },
            style.background_color,
        );

        // --- Percentage text (left side of pill) ---
        {
            use iced::advanced::text::Renderer as _;

            let pct_text = format!("{:.1}%", percentage * 100.0);
            renderer.fill_text(
                iced::advanced::text::Text {
                    content: pct_text,
                    bounds: Size::new(TRIGGER_TEXT_WIDTH, bounds.height),
                    size: Pixels(13.0),
                    line_height: iced::advanced::text::LineHeight::default(),
                    font: renderer.default_font(),
                    align_x: iced::advanced::text::Alignment::Right,
                    align_y: alignment::Vertical::Center,
                    shaping: iced::advanced::text::Shaping::Basic,
                    wrapping: iced::advanced::text::Wrapping::None,
                    ellipsis: iced::advanced::text::Ellipsis::None,
                    hint_factor: None,
                },
                Point::new(
                    bounds.x + TRIGGER_PADDING_H + TRIGGER_TEXT_WIDTH,
                    bounds.center_y(),
                ),
                style.text_color,
                *viewport,
            );
        }

        // --- Tiny ring (right side of pill) ---
        let ring_x = bounds.x + TRIGGER_PADDING_H + TRIGGER_TEXT_WIDTH + TRIGGER_GAP;
        let ring_y = bounds.center_y() - TRIGGER_RING_SIZE / 2.0;
        let ring_bounds = Size::new(TRIGGER_RING_SIZE, TRIGGER_RING_SIZE);

        let geometry = state.ring_cache.draw(renderer, ring_bounds, |frame| {
            let center = frame.center();
            let radius = (TRIGGER_RING_SIZE / 2.0) - 2.0;
            let stroke_width = 2.0;

            frame.stroke(
                &Path::circle(center, radius),
                Stroke::default()
                    .with_color(style.track_color)
                    .with_width(stroke_width),
            );

            if percentage > 0.0 {
                let mut builder = canvas::path::Builder::new();
                let start = -PI / 2.0;
                let end = start + 2.0 * PI * percentage;
                builder.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: Radians(start),
                    end_angle: Radians(end),
                });
                frame.stroke(
                    &builder.build(),
                    Stroke::default()
                        .with_color(style.fill_color)
                        .with_width(stroke_width),
                );
            }
        });

        renderer.with_translation(Vector::new(ring_x, ring_y), |renderer| {
            use iced::advanced::graphics::geometry::Renderer as _;
            renderer.draw_geometry(geometry);
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let state = tree.state.downcast_mut::<State>();
                let was_open = state.is_open();
                let over = cursor.position_over(layout.bounds()).is_some();
                state.trigger_hovered.set(over);

                let now_open = state.is_open();
                if was_open != now_open {
                    shell.invalidate_layout();
                    if let Some(cb) = &self.on_hover {
                        shell.publish(cb(now_open));
                    }
                }
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                let state = tree.state.downcast_mut::<State>();
                let was_open = state.is_open();
                state.trigger_hovered.set(false);
                if was_open && !state.is_open() {
                    shell.invalidate_layout();
                    if let Some(cb) = &self.on_hover {
                        shell.publish(cb(false));
                    }
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.position_over(layout.bounds()).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let state = tree.state.downcast_ref::<State>();
        if !state.is_open() {
            return None;
        }

        let bounds = layout.bounds();
        let style = state.cached_style.get().unwrap_or_default();

        Some(iced::advanced::overlay::Element::new(Box::new(
            ContextCard {
                anchor: Point::new(bounds.x + translation.x, bounds.y + translation.y),
                anchor_size: bounds.size(),
                style,
                percentage: self.percentage(),
                used_tokens: self.used_tokens,
                max_tokens: self.max_tokens,
                usage: self.usage,
                pricing: self.pricing,
                card_hovered: &state.card_hovered,
                _phantom: PhantomData,
            },
        )))
    }
}

impl<'a, Message, Theme> From<Context<'a, Message, Theme>> for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
{
    fn from(ctx: Context<'a, Message, Theme>) -> Self {
        Self::new(ctx)
    }
}

// ============================================================================
// Overlay Card
// ============================================================================

const CARD_WIDTH: f32 = 240.0;
const CARD_PADDING: f32 = 16.0;
const CARD_RADIUS: f32 = 10.0;
const HEADER_HEIGHT: f32 = 20.0;
const BAR_HEIGHT: f32 = 6.0;
const BAR_RADIUS: f32 = 3.0;
const ROW_HEIGHT: f32 = 20.0;
const SECTION_GAP: f32 = 12.0;
const ROW_GAP: f32 = 2.0;
const FONT_HEADER: f32 = 12.0;
const FONT_BODY: f32 = 11.5;
const FONT_FOOTER: f32 = 12.0;

struct ContextCard<'a, Message, Theme>
where
    Theme: Catalog,
{
    anchor: Point,
    anchor_size: Size,
    style: Style,
    percentage: f32,
    used_tokens: u64,
    max_tokens: u64,
    usage: Usage,
    pricing: Option<ModelPricing>,
    card_hovered: &'a Cell<bool>,
    _phantom: PhantomData<(Message, Theme)>,
}

impl<Message, Theme> ContextCard<'_, Message, Theme>
where
    Theme: Catalog,
{
    fn usage_rows(&self) -> Vec<(&'static str, u64, Option<f64>)> {
        let mut rows = Vec::new();
        if self.usage.input > 0 {
            let cost = self
                .pricing
                .map(|p| (self.usage.input as f64 / 1e6) * p.input);
            rows.push(("Input", self.usage.input, cost));
        }
        if self.usage.output > 0 {
            let cost = self
                .pricing
                .map(|p| (self.usage.output as f64 / 1e6) * p.output);
            rows.push(("Output", self.usage.output, cost));
        }
        if self.usage.reasoning > 0 {
            let cost = self
                .pricing
                .map(|p| (self.usage.reasoning as f64 / 1e6) * p.reasoning);
            rows.push(("Reasoning", self.usage.reasoning, cost));
        }
        if self.usage.cached > 0 {
            let cost = self
                .pricing
                .map(|p| (self.usage.cached as f64 / 1e6) * p.cached);
            rows.push(("Cached", self.usage.cached, cost));
        }
        rows
    }

    fn card_height(&self) -> f32 {
        let rows = self.usage_rows();
        let body_h =
            rows.len() as f32 * ROW_HEIGHT + (rows.len().saturating_sub(1)) as f32 * ROW_GAP;

        let mut h = CARD_PADDING + HEADER_HEIGHT + SECTION_GAP + BAR_HEIGHT + SECTION_GAP
            + 1.0 + SECTION_GAP  // divider between bar and rows
            + body_h
            + CARD_PADDING;

        let has_cost = self.pricing.map_or(false, |p| {
            p.input > 0.0 || p.output > 0.0 || p.reasoning > 0.0 || p.cached > 0.0
        });
        if has_cost {
            h += SECTION_GAP + 1.0 + SECTION_GAP + ROW_HEIGHT;
        }

        h
    }

    /// Helper to draw a text label. `anchor_x` is the alignment anchor:
    /// for Left-aligned text, it's the left edge; for Right-aligned, it's
    /// the right edge.
    fn draw_text(
        &self,
        renderer: &mut Renderer,
        content: &str,
        size: Pixels,
        font: iced::Font,
        text_bounds: Size,
        anchor_x: f32,
        center_y: f32,
        align_x: iced::advanced::text::Alignment,
        color: Color,
        clip: Rectangle,
    ) {
        use iced::advanced::text::Renderer as _;
        renderer.fill_text(
            iced::advanced::text::Text {
                content: content.to_owned(),
                bounds: text_bounds,
                size,
                line_height: iced::advanced::text::LineHeight::default(),
                font,
                align_x,
                align_y: alignment::Vertical::Center,
                shaping: iced::advanced::text::Shaping::Basic,
                wrapping: iced::advanced::text::Wrapping::None,
                ellipsis: iced::advanced::text::Ellipsis::None,
                hint_factor: None,
            },
            Point::new(anchor_x, center_y),
            color,
            clip,
        );
    }
}

impl<Message, Theme> iced::advanced::Overlay<Message, Theme, Renderer>
    for ContextCard<'_, Message, Theme>
where
    Theme: Catalog,
{
    fn layout(&mut self, _renderer: &Renderer, _bounds: Size) -> layout::Node {
        let h = self.card_height();
        let gap = 4.0;
        layout::Node::new(Size::new(CARD_WIDTH, h)).translate(Vector::new(
            self.anchor.x,
            self.anchor.y + self.anchor_size.height + gap,
        ))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        _theme: &Theme,
        _renderer_style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
    ) {
        use advanced::Renderer as _;

        let bounds = layout.bounds();
        let s = &self.style;
        let font = {
            use iced::advanced::text::Renderer as _;
            renderer.default_font()
        };

        // --- Card background ---
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: iced::Border {
                    color: s.border_color,
                    width: 1.0,
                    radius: CARD_RADIUS.into(),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.08),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..renderer::Quad::default()
            },
            s.background_color,
        );

        let left = bounds.x + CARD_PADDING;
        let right = bounds.x + bounds.width - CARD_PADDING;
        let cw = right - left;
        let mut y = bounds.y + CARD_PADDING;

        // =====================================================================
        // HEADER: "XX.X%" left, "40K / 128K" right
        // =====================================================================
        let pct_str = format!("{:.1}%", self.percentage * 100.0);
        let tokens_str = format!(
            "{}  /  {}",
            format_tokens(self.used_tokens),
            format_tokens(self.max_tokens),
        );
        let cy = y + HEADER_HEIGHT / 2.0;

        self.draw_text(
            renderer,
            &pct_str,
            Pixels(FONT_HEADER),
            font,
            Size::new(cw * 0.4, HEADER_HEIGHT),
            left,
            cy,
            iced::advanced::text::Alignment::Left,
            s.text_color,
            bounds,
        );
        self.draw_text(
            renderer,
            &tokens_str,
            Pixels(FONT_HEADER),
            font,
            Size::new(cw, HEADER_HEIGHT),
            right,
            cy,
            iced::advanced::text::Alignment::Right,
            s.secondary_text_color,
            bounds,
        );
        y += HEADER_HEIGHT + SECTION_GAP;

        // =====================================================================
        // PROGRESS BAR
        // =====================================================================
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: left,
                    y,
                    width: cw,
                    height: BAR_HEIGHT,
                },
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: BAR_RADIUS.into(),
                },
                ..renderer::Quad::default()
            },
            s.track_color,
        );
        let fill_w = (cw * self.percentage).max(0.0);
        if fill_w > 0.0 {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: left,
                        y,
                        width: fill_w,
                        height: BAR_HEIGHT,
                    },
                    border: iced::Border {
                        color: Color::TRANSPARENT,
                        width: 0.0,
                        radius: BAR_RADIUS.into(),
                    },
                    ..renderer::Quad::default()
                },
                s.fill_color,
            );
        }
        y += BAR_HEIGHT + SECTION_GAP;

        // Divider between progress bar and usage rows
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    x: left,
                    y,
                    width: cw,
                    height: 1.0,
                },
                ..renderer::Quad::default()
            },
            s.border_color,
        );
        y += 1.0 + SECTION_GAP;

        // =====================================================================
        // USAGE ROWS
        // =====================================================================
        let rows = self.usage_rows();
        for (label, tokens, cost) in &rows {
            let cy = y + ROW_HEIGHT / 2.0;

            // Label on left
            self.draw_text(
                renderer,
                label,
                Pixels(FONT_BODY),
                font,
                Size::new(cw * 0.4, ROW_HEIGHT),
                left,
                cy,
                iced::advanced::text::Alignment::Left,
                s.secondary_text_color,
                bounds,
            );

            // Value on right: "32K · $0.04" or just "32K"
            let value_str = match cost {
                Some(c) if *c > 0.0 => {
                    format!(
                        "{}  \u{00B7}  {}",
                        format_tokens(*tokens),
                        format_cost(*c)
                    )
                }
                _ => format_tokens(*tokens),
            };
            self.draw_text(
                renderer,
                &value_str,
                Pixels(FONT_BODY),
                font,
                Size::new(cw, ROW_HEIGHT),
                right,
                cy,
                iced::advanced::text::Alignment::Right,
                s.text_color,
                bounds,
            );

            y += ROW_HEIGHT + ROW_GAP;
        }

        // =====================================================================
        // FOOTER: Total cost
        // =====================================================================
        let has_cost = self.pricing.map_or(false, |p| {
            p.input > 0.0 || p.output > 0.0 || p.reasoning > 0.0 || p.cached > 0.0
        });
        if has_cost {
            y += SECTION_GAP - ROW_GAP;

            // Footer background (gray, rounded bottom corners)
            let footer_top = y;
            let footer_height = bounds.y + bounds.height - footer_top;
            let footer_bg = s.footer_background_color;

            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: footer_top,
                        width: bounds.width,
                        height: footer_height,
                    },
                    border: iced::Border {
                        color: s.border_color,
                        width: 1.0,
                        radius: iced::border::Radius {
                            top_left: 0.0,
                            top_right: 0.0,
                            bottom_right: CARD_RADIUS,
                            bottom_left: CARD_RADIUS,
                        },
                    },
                    ..renderer::Quad::default()
                },
                footer_bg,
            );

            // Divider at top of footer
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle {
                        x: bounds.x,
                        y: footer_top,
                        width: bounds.width,
                        height: 1.0,
                    },
                    ..renderer::Quad::default()
                },
                s.border_color,
            );
            y = footer_top + 1.0 + SECTION_GAP;

            let cy = y + ROW_HEIGHT / 2.0;

            self.draw_text(
                renderer,
                "Total cost",
                Pixels(FONT_FOOTER),
                font,
                Size::new(cw * 0.5, ROW_HEIGHT),
                left,
                cy,
                iced::advanced::text::Alignment::Left,
                s.secondary_text_color,
                bounds,
            );

            if let Some(pricing) = &self.pricing {
                let total = pricing.calculate_cost(&self.usage);
                self.draw_text(
                    renderer,
                    &format_cost(total),
                    Pixels(FONT_FOOTER),
                    font,
                    Size::new(cw, ROW_HEIGHT),
                    right,
                    cy,
                    iced::advanced::text::Alignment::Right,
                    s.text_color,
                    bounds,
                );
            }
        }
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
        _shell: &mut Shell<'_, Message>,
    ) {
        // Track whether cursor is over the card.
        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let over_card = cursor.position_over(layout.bounds()).is_some();
                self.card_hovered.set(over_card);
            }
            Event::Mouse(mouse::Event::CursorLeft) => {
                self.card_hovered.set(false);
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.position_over(layout.bounds()).is_some() {
            mouse::Interaction::Idle
        } else {
            mouse::Interaction::None
        }
    }
}
