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
/// The caret blinks by toggling its alpha based on wall-clock time.
pub fn caret_span(
    kind: CaretKind,
    color: Color,
    _now: Instant,
    blink_interval: Duration,
) -> text::Span<'static, Uri> {
    let cycle_ms = blink_interval.as_millis() * 2;
    let visible = if cycle_ms == 0 {
        true
    } else {
        let since_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        (since_epoch % cycle_ms) < blink_interval.as_millis()
    };

    let alpha = if visible { color.a } else { 0.0 };

    let character = match kind {
        CaretKind::Block => BLOCK_CHAR,
        CaretKind::Circle => CIRCLE_CHAR,
    };

    text::Span::new(character.to_owned()).color(Color { a: alpha, ..color })
}
