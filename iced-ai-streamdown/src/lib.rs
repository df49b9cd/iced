//! # iced-ai-streamdown
//!
//! A streaming markdown renderer for AI chat applications, built on iced.
//!
//! This crate wraps and extends [`iced::widget::markdown`] with streaming-specific
//! features inspired by [Streamdown](https://streamdown.ai):
//!
//! - **Word-level fade-in animation** as tokens arrive from the AI model
//! - **Blinking caret** at the insertion point (block `▋` or circle `●`)
//! - **Block-level change tracking** so only actively-changing blocks are re-rendered
//! - **Custom code block renderers** for languages like mermaid, vega-lite, etc.
//!
//! # Example
//!
//! ```ignore
//! use iced_ai_streamdown::{StreamContent, AnimationState, StreamSettings, stream_view};
//! use iced_ai_streamdown::AnimationKind;
//! use iced::time::Instant;
//! use iced::Theme;
//!
//! struct Chat {
//!     content: StreamContent,
//!     animation: AnimationState,
//!     now: Instant,
//! }
//!
//! // In update(), when a new token arrives:
//! // let prev_words = content.total_word_count();
//! // content.push_str(&token);
//! // let new_words = content.total_word_count() - prev_words;
//! // animation.reveal_words(new_words, now);
//!
//! // In view():
//! // let settings = StreamSettings::new(&theme);
//! // stream_view(&content, &animation, &settings, now)
//! //     .map(Message::LinkClicked)
//! ```
//!
//! # Limitations
//!
//! - `BlurIn` animation degrades to `FadeIn` (iced has no blur primitive)
//! - `SlideUp` animation degrades to `FadeIn` (rich text has no per-span vertical offset)
//! - Table row cells are estimated for word counting (private field in upstream)

pub mod animation;
pub mod caret;
pub mod content;
pub mod custom_renderer;
pub mod settings;
pub mod view;
pub mod viewer;

pub use animation::AnimationState;
pub use content::StreamContent;
pub use settings::{AnimationKind, AnimationSep, CaretKind, EasingFunction};
pub use custom_renderer::{CustomBlockRenderer, CustomRendererRegistry};
pub use remend::detect_direction::TextDirection;
pub use remend::{LinkMode, RemendHandler, RemendOptions};
pub use settings::StreamSettings;
pub use view::{stream_view, stream_view_with_renderers};

// Re-export types from iced that users will commonly need alongside this crate.
pub use iced::widget::markdown::{self, Item, Uri};
