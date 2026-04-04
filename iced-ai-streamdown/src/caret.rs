use iced::time::{Duration, Instant};
use iced::widget::markdown::Uri;
use iced::widget::text;
use iced::Color;

use crate::settings::CaretKind;

/// The character used for a block caret.
const BLOCK_CHAR: &str = "\u{258B}";

/// The character used for a circle caret.
const CIRCLE_CHAR: &str = "\u{25CF}";

/// Creates a text span representing the streaming caret.
///
/// The caret blinks by toggling its alpha based on the provided `Instant`,
/// relative to a lazily-initialized epoch.
pub fn caret_span(
    kind: CaretKind,
    color: Color,
    now: Instant,
    blink_interval: Duration,
) -> text::Span<'static, Uri> {
    static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

    let cycle_ms = blink_interval.as_millis() * 2;
    let visible = if cycle_ms == 0 {
        true
    } else {
        let epoch = *EPOCH.get_or_init(|| now);
        let elapsed_ms = now.duration_since(epoch).as_millis();
        (elapsed_ms % cycle_ms) < blink_interval.as_millis()
    };

    let alpha = if visible { color.a } else { 0.0 };

    let character = match kind {
        CaretKind::Block => BLOCK_CHAR,
        CaretKind::Circle => CIRCLE_CHAR,
    };

    text::Span::new(character.to_owned()).color(Color { a: alpha, ..color })
}
