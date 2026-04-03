use iced::animation::Easing;
use iced::time::{Duration, Instant};

use crate::settings::AnimationKind;

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
