//! Text direction detection using the "first strong character" Unicode algorithm.
//!
//! Ported from Streamdown's `detect-direction.ts`.

/// Text direction: left-to-right or right-to-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDirection {
    /// Left-to-right (default for Latin, CJK, Cyrillic, etc.)
    #[default]
    Ltr,
    /// Right-to-left (Hebrew, Arabic, Syriac, Thaana, etc.)
    Rtl,
}

/// Returns `true` if the character is in a Unicode RTL "strong" range.
///
/// Covers: Hebrew, Arabic, Syriac, Thaana, NKo, Samaritan, Mandaic,
/// Arabic Supplement/Extended, and RTL presentation forms.
fn is_rtl_char(ch: char) -> bool {
    let cp = ch as u32;
    (0x0590..=0x08FF).contains(&cp)        // Hebrew through Arabic Extended
        || (0xFB1D..=0xFDFF).contains(&cp)  // Alphabetic Presentation Forms + Arabic Presentation Forms-A
        || (0xFE70..=0xFEFF).contains(&cp)  // Arabic Presentation Forms-B
}

/// Detects text direction using the "first strong character" algorithm.
///
/// Strips common markdown syntax (headings, bold/italic markers, inline code,
/// links, list markers) then finds the first Unicode letter with strong
/// directionality.
///
/// Returns [`TextDirection::Rtl`] if the first strong character is RTL,
/// [`TextDirection::Ltr`] otherwise.
pub fn detect_text_direction(text: &str) -> TextDirection {
    // We iterate character-by-character through a simple state machine that
    // skips markdown syntax. This avoids allocating a stripped copy.
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];

        // Skip heading markers at line start: #{1,6} followed by space.
        if ch == '#' && is_line_start(&chars, i) {
            while i < len && chars[i] == '#' {
                i += 1;
            }
            // Skip whitespace after #'s.
            while i < len && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }

        // Skip bold/italic markers (*, _).
        if ch == '*' || ch == '_' {
            i += 1;
            continue;
        }

        // Skip inline code: `...`
        if ch == '`' {
            i += 1;
            while i < len && chars[i] != '`' {
                i += 1;
            }
            if i < len {
                i += 1; // skip closing `
            }
            continue;
        }

        // Skip links: [text](url) — keep the text.
        // When we see `[`, just skip the bracket itself.
        if ch == '[' {
            i += 1;
            continue;
        }
        // When we see `](`, skip through the closing `)`.
        if ch == ']' && i + 1 < len && chars[i + 1] == '(' {
            i += 2;
            while i < len && chars[i] != ')' {
                i += 1;
            }
            if i < len {
                i += 1; // skip )
            }
            continue;
        }
        if ch == ']' {
            i += 1;
            continue;
        }

        // Skip line-start markers: >, -, +, digits followed by ., spaces.
        if is_line_start(&chars, i) && is_list_or_quote_char(ch) {
            while i < len && (is_list_or_quote_char(chars[i]) || chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // Skip trailing spaces.
            while i < len && chars[i] == ' ' {
                i += 1;
            }
            continue;
        }

        // Check if this is a strong directional character (any letter).
        if ch.is_alphabetic() {
            if is_rtl_char(ch) {
                return TextDirection::Rtl;
            }
            return TextDirection::Ltr;
        }

        i += 1;
    }

    TextDirection::Ltr
}

/// Returns true if position `i` is at the start of a line.
fn is_line_start(chars: &[char], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    // Check if preceded by newline (possibly with spaces in between).
    let mut j = i;
    while j > 0 {
        j -= 1;
        if chars[j] == '\n' {
            return true;
        }
        if chars[j] != ' ' && chars[j] != '\t' {
            return false;
        }
    }
    // Reached the start of the string through whitespace only.
    true
}

fn is_list_or_quote_char(ch: char) -> bool {
    matches!(ch, '>' | '-' | '+' | '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_text() {
        assert_eq!(detect_text_direction("Hello world"), TextDirection::Ltr);
    }

    #[test]
    fn hebrew_text() {
        assert_eq!(detect_text_direction("שלום עולם"), TextDirection::Rtl);
    }

    #[test]
    fn arabic_text() {
        assert_eq!(detect_text_direction("مرحبا بالعالم"), TextDirection::Rtl);
    }

    #[test]
    fn heading_with_hebrew() {
        assert_eq!(detect_text_direction("## שלום"), TextDirection::Rtl);
    }

    #[test]
    fn bold_english() {
        assert_eq!(detect_text_direction("**hello**"), TextDirection::Ltr);
    }

    #[test]
    fn bold_arabic() {
        assert_eq!(detect_text_direction("**مرحبا**"), TextDirection::Rtl);
    }

    #[test]
    fn link_with_hebrew_text() {
        assert_eq!(
            detect_text_direction("[שלום](https://example.com)"),
            TextDirection::Rtl
        );
    }

    #[test]
    fn inline_code_then_hebrew() {
        assert_eq!(detect_text_direction("`code` שלום"), TextDirection::Rtl);
    }

    #[test]
    fn numbers_then_english() {
        assert_eq!(detect_text_direction("123 hello"), TextDirection::Ltr);
    }

    #[test]
    fn numbers_then_arabic() {
        assert_eq!(detect_text_direction("123 مرحبا"), TextDirection::Rtl);
    }

    #[test]
    fn empty_string() {
        assert_eq!(detect_text_direction(""), TextDirection::Ltr);
    }

    #[test]
    fn only_numbers() {
        assert_eq!(detect_text_direction("12345"), TextDirection::Ltr);
    }

    #[test]
    fn list_item_hebrew() {
        assert_eq!(detect_text_direction("- שלום"), TextDirection::Rtl);
    }

    #[test]
    fn blockquote_arabic() {
        assert_eq!(detect_text_direction("> مرحبا"), TextDirection::Rtl);
    }

    #[test]
    fn mixed_ltr_first() {
        assert_eq!(
            detect_text_direction("Hello שלום"),
            TextDirection::Ltr
        );
    }

    #[test]
    fn mixed_rtl_first() {
        assert_eq!(
            detect_text_direction("שלום Hello"),
            TextDirection::Rtl
        );
    }

    #[test]
    fn cjk_is_ltr() {
        assert_eq!(detect_text_direction("你好世界"), TextDirection::Ltr);
    }

    #[test]
    fn cyrillic_is_ltr() {
        assert_eq!(detect_text_direction("Привет мир"), TextDirection::Ltr);
    }
}
