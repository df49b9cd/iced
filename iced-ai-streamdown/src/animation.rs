use iced::animation::Easing;
use iced::time::{Duration, Instant};

use crate::settings::{AnimationKind, StreamSettings};

/// Tracks word-level animation state for streaming content.
///
/// Uses a flat [`Vec<Instant>`] of reveal timestamps rather than per-word
/// [`Animation`](iced::Animation) instances, keeping overhead minimal even for
/// thousands of words.
#[derive(Debug, Clone)]
pub struct AnimationState {
    /// The instant each word was scheduled to start revealing.
    word_reveal_times: Vec<Instant>,
    /// What kind of animation to run.
    kind: AnimationKind,
    /// How long each word's animation takes.
    duration: Duration,
    /// Delay between successive word animation starts.
    stagger: Duration,
    /// Easing function applied to the opacity curve.
    easing: Easing,
}

impl AnimationState {
    /// Creates a new [`AnimationState`] with the given animation kind.
    pub fn new(kind: AnimationKind) -> Self {
        Self {
            word_reveal_times: Vec::new(),
            kind,
            duration: Duration::from_millis(300),
            stagger: Duration::from_millis(30),
            easing: Easing::EaseOut,
        }
    }

    /// Creates an [`AnimationState`] from [`StreamSettings`], inheriting the
    /// animation kind, duration, and stagger so defaults aren't duplicated.
    pub fn from_settings(settings: &StreamSettings) -> Self {
        Self::new(settings.animation)
            .duration(settings.animation_duration)
            .stagger(settings.animation_stagger)
    }

    /// Sets the duration per word.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Sets the stagger between word reveals.
    pub fn stagger(mut self, stagger: Duration) -> Self {
        self.stagger = stagger;
        self
    }

    /// Sets the easing function.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Records `count` new words as revealed starting at `now`, each
    /// staggered by the configured stagger interval.
    pub fn reveal_words(&mut self, count: usize, now: Instant) {
        for i in 0..count {
            let reveal_at = now + self.stagger * i as u32;
            self.word_reveal_times.push(reveal_at);
        }
    }

    /// Computes the opacity (0.0..=1.0) for the word at the given global index.
    ///
    /// Returns 1.0 if the animation kind is [`AnimationKind::None`] or if the
    /// word has finished animating.
    pub fn word_opacity(&self, global_word_index: usize, now: Instant) -> f32 {
        if matches!(self.kind, AnimationKind::None) {
            return 1.0;
        }

        let Some(reveal_time) = self.word_reveal_times.get(global_word_index) else {
            // Word not yet registered — fully transparent.
            return 0.0;
        };

        if now < *reveal_time {
            return 0.0;
        }

        let elapsed = now.duration_since(*reveal_time);
        let raw_t = if self.duration.as_nanos() == 0 {
            1.0
        } else {
            (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
        };

        apply_easing(raw_t, self.easing)
    }

    /// Returns `true` if any word is still mid-animation.
    pub fn is_animating(&self, now: Instant) -> bool {
        if matches!(self.kind, AnimationKind::None) {
            return false;
        }

        self.word_reveal_times.last().is_some_and(|last| {
            now.duration_since(*last) < self.duration
        })
    }

    /// Returns `true` once at least one word has been revealed.
    ///
    /// Useful for triggering "animation started" events in the host application
    /// (e.g., starting auto-scroll or hiding a loading indicator).
    pub fn has_started(&self) -> bool {
        !self.word_reveal_times.is_empty()
    }

    /// Returns `true` when all revealed words have finished their animation.
    ///
    /// Returns `false` if no words have been revealed yet or if any word is
    /// still mid-animation. Useful for triggering "animation ended" events
    /// (e.g., re-enabling user interaction or snapping scroll position).
    pub fn has_finished(&self, now: Instant) -> bool {
        if self.word_reveal_times.is_empty() {
            return false;
        }
        !self.is_animating(now)
    }

    /// Returns the total number of words that have been registered.
    pub fn word_count(&self) -> usize {
        self.word_reveal_times.len()
    }

    /// Resets all animation state.
    pub fn clear(&mut self) {
        self.word_reveal_times.clear();
    }
}

/// Applies the given easing to a linear `t` in 0.0..=1.0.
fn apply_easing(t: f32, easing: Easing) -> f32 {
    match easing {
        Easing::Linear => t,
        Easing::EaseInQuad => t * t,
        Easing::EaseOutQuad => t * (2.0 - t),
        Easing::EaseInOutQuad => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
        Easing::EaseOut | Easing::EaseOutCubic => {
            let t1 = t - 1.0;
            1.0 + t1 * t1 * t1
        }
        Easing::EaseIn | Easing::EaseInCubic => t * t * t,
        // For all other easings, use ease-out cubic as a reasonable default.
        _ => {
            let t1 = t - 1.0;
            1.0 + t1 * t1 * t1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn none_animation_always_opaque() {
        let state = AnimationState::new(AnimationKind::None);
        assert_eq!(state.word_opacity(0, now()), 1.0);
        assert_eq!(state.word_opacity(999, now()), 1.0);
    }

    #[test]
    fn unregistered_word_is_transparent() {
        let state = AnimationState::new(AnimationKind::FadeIn);
        assert_eq!(state.word_opacity(0, now()), 0.0);
    }

    #[test]
    fn word_fully_revealed_after_duration() {
        let mut state = AnimationState::new(AnimationKind::FadeIn)
            .duration(Duration::from_millis(100));
        let t = now();
        state.reveal_words(1, t);
        let after = t + Duration::from_millis(200);
        assert_eq!(state.word_opacity(0, after), 1.0);
    }

    #[test]
    fn word_zero_before_reveal_time() {
        let mut state = AnimationState::new(AnimationKind::FadeIn)
            .stagger(Duration::from_millis(1000));
        let t = now();
        state.reveal_words(2, t);
        // Word 1 is staggered 1s later — should be 0 at t+0
        assert_eq!(state.word_opacity(1, t), 0.0);
    }

    #[test]
    fn is_animating_while_mid_reveal() {
        let mut state = AnimationState::new(AnimationKind::FadeIn)
            .duration(Duration::from_millis(500));
        let t = now();
        state.reveal_words(1, t);
        assert!(state.is_animating(t));
        assert!(!state.is_animating(t + Duration::from_secs(1)));
    }

    #[test]
    fn has_started_and_finished() {
        let mut state = AnimationState::new(AnimationKind::FadeIn)
            .duration(Duration::from_millis(10));
        let t = now();
        assert!(!state.has_started());
        assert!(!state.has_finished(t));

        state.reveal_words(1, t);
        assert!(state.has_started());

        let later = t + Duration::from_secs(1);
        assert!(state.has_finished(later));
    }

    #[test]
    fn easing_boundaries() {
        // All easings should return 0 at t=0 and 1 at t=1
        for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut,
                       Easing::EaseInQuad, Easing::EaseOutQuad, Easing::EaseInOutQuad] {
            assert_eq!(apply_easing(0.0, easing), 0.0, "easing {:?} at 0", easing);
            assert!((apply_easing(1.0, easing) - 1.0).abs() < 1e-6,
                    "easing {:?} at 1", easing);
        }
    }

    #[test]
    fn easing_monotonic() {
        // Easings should be monotonically non-decreasing from 0 to 1
        for easing in [Easing::Linear, Easing::EaseIn, Easing::EaseOut] {
            let mut prev = 0.0f32;
            for i in 0..=100 {
                let t = i as f32 / 100.0;
                let v = apply_easing(t, easing);
                assert!(v >= prev - 1e-6, "easing {:?} not monotonic at t={}", easing, t);
                prev = v;
            }
        }
    }

    #[test]
    fn from_settings_copies_values() {
        use iced::widget::markdown;
        use iced::theme::palette::Seed;
        let style = markdown::Style::from_palette(Seed::CATPPUCCIN_MOCHA);
        let settings = StreamSettings::new(markdown::Settings::with_text_size(16.0, style))
            .animation(AnimationKind::FadeIn)
            .animation_duration(Duration::from_millis(500))
            .animation_stagger(Duration::from_millis(50));
        let state = AnimationState::from_settings(&settings);
        assert_eq!(state.duration, Duration::from_millis(500));
        assert_eq!(state.stagger, Duration::from_millis(50));
    }
}
