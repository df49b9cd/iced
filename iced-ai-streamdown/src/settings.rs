use iced::time::Duration;
use iced::widget::markdown;
use iced::Color;

/// The kind of animation to apply when revealing new words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationKind {
    /// Fade in from transparent to opaque.
    #[default]
    FadeIn,
    /// Blur-to-sharp effect. Currently degrades to [`FadeIn`](AnimationKind::FadeIn)
    /// since iced has no blur primitive.
    BlurIn,
    /// Fade in with upward motion. Currently degrades to [`FadeIn`](AnimationKind::FadeIn)
    /// since rich text does not support per-span vertical offset.
    SlideUp,
    /// No animation; words appear instantly.
    None,
}

/// The kind of caret to display at the streaming insertion point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretKind {
    /// A vertical block cursor: `▋`
    Block,
    /// A filled circle: `●`
    Circle,
}

/// Configuration for the streaming markdown renderer.
#[derive(Debug, Clone)]
pub struct StreamSettings {
    /// The underlying markdown rendering settings.
    pub markdown: markdown::Settings,
    /// The animation to apply to newly revealed words.
    pub animation: AnimationKind,
    /// How long each word's animation takes.
    pub animation_duration: Duration,
    /// Delay between each word's animation start.
    pub animation_stagger: Duration,
    /// The caret to show at the insertion point, if any.
    pub caret: Option<CaretKind>,
    /// Override color for the caret. If `None`, uses the text color.
    pub caret_color: Option<Color>,
    /// How fast the caret blinks (half-period).
    pub caret_blink_interval: Duration,
}

impl StreamSettings {
    /// Creates new [`StreamSettings`] from the given markdown settings.
    pub fn new(markdown: impl Into<markdown::Settings>) -> Self {
        Self {
            markdown: markdown.into(),
            animation: AnimationKind::FadeIn,
            animation_duration: Duration::from_millis(300),
            animation_stagger: Duration::from_millis(30),
            caret: Some(CaretKind::Block),
            caret_color: None,
            caret_blink_interval: Duration::from_millis(530),
        }
    }

    /// Sets the animation kind.
    pub fn animation(mut self, kind: AnimationKind) -> Self {
        self.animation = kind;
        self
    }

    /// Sets the animation duration per word.
    pub fn animation_duration(mut self, duration: Duration) -> Self {
        self.animation_duration = duration;
        self
    }

    /// Sets the stagger between word animation starts.
    pub fn animation_stagger(mut self, stagger: Duration) -> Self {
        self.animation_stagger = stagger;
        self
    }

    /// Sets the caret kind.
    pub fn caret(mut self, kind: Option<CaretKind>) -> Self {
        self.caret = kind;
        self
    }

    /// Sets the caret color.
    pub fn caret_color(mut self, color: Option<Color>) -> Self {
        self.caret_color = color;
        self
    }

    /// Sets the caret blink interval.
    pub fn caret_blink_interval(mut self, interval: Duration) -> Self {
        self.caret_blink_interval = interval;
        self
    }
}
