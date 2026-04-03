use iced::time::{self, Instant, milliseconds};
use iced::widget::{center_x, column, container, markdown, scrollable, text};
use iced::window;
use iced::{Element, Fill, Subscription, Task, Theme};

use iced_ai_streamdown::{
    AnimationKind, AnimationState, RemendOptions, StreamContent, StreamSettings, stream_view,
};

const SAMPLE_RESPONSE: &str = r#"# Hello from Streamdown!

This is a **streaming markdown** renderer built on iced. Watch as the text appears word by word with a smooth fade-in animation.

## Features

- Word-level **fade-in** animation
- Blinking caret at the insertion point
- Block-level change tracking for efficient rendering
- Built on iced's existing markdown widget

## Code Example

```rust
fn main() {
    println!("Hello, streamdown!");
}
```

Here is some `inline code` and a [link](https://iced.rs) to the iced website.

> This is a blockquote that demonstrates how quoted text
> is rendered during streaming.

And finally, a table:

| Feature | Status |
|---------|--------|
| FadeIn | Done |
| BlurIn | Fallback to FadeIn |
| SlideUp | Fallback to FadeIn |
| Caret | Done |
"#;

pub fn main() -> iced::Result {
    iced::application::timed(Chat::new, Chat::update, Chat::subscription, Chat::view)
        .theme(Chat::theme)
        .title("iced-ai-streamdown — Chat Example")
        .run()
}

struct Chat {
    content: StreamContent,
    animation: AnimationState,
    pending: String,
    is_streaming: bool,
    now: Instant,
}

#[derive(Debug, Clone)]
enum Message {
    NextToken,
    Tick(Instant),
    LinkClicked(markdown::Uri),
}

impl Chat {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: StreamContent::with_remend(RemendOptions::default()),
                animation: AnimationState::new(AnimationKind::FadeIn)
                    .duration(std::time::Duration::from_millis(300))
                    .stagger(std::time::Duration::from_millis(30)),
                pending: SAMPLE_RESPONSE.to_owned(),
                is_streaming: true,
                now: Instant::now(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message, now: Instant) -> Task<Message> {
        self.now = now;

        match message {
            Message::NextToken => {
                if self.pending.is_empty() {
                    self.is_streaming = false;
                    self.content.finish();
                } else {
                    let prev_words = self.content.total_word_count();

                    // Emit a few words at a time to simulate token streaming.
                    let mut chars_to_take = 0;
                    let mut words_emitted = 0;
                    for (i, c) in self.pending.char_indices() {
                        chars_to_take = i + c.len_utf8();
                        if c.is_whitespace() {
                            words_emitted += 1;
                            if words_emitted >= 2 {
                                break;
                            }
                        }
                    }
                    if words_emitted == 0 {
                        chars_to_take = self.pending.len();
                    }

                    let chunk = self.pending[..chars_to_take].to_owned();
                    self.pending = self.pending[chars_to_take..].to_owned();
                    self.content.push_str(&chunk);

                    let new_words = self.content.total_word_count() - prev_words;
                    self.animation.reveal_words(new_words, now);
                }

                Task::none()
            }
            Message::Tick(_now) => Task::none(),
            Message::LinkClicked(url) => {
                println!("Link clicked: {url}");
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let stream_tokens = if self.is_streaming {
            time::every(milliseconds(50)).map(|_| Message::NextToken)
        } else {
            Subscription::none()
        };

        let animate = if self.is_streaming || self.animation.is_animating(self.now) {
            window::frames().map(|_| Message::Tick(Instant::now()))
        } else {
            Subscription::none()
        };

        Subscription::batch([stream_tokens, animate])
    }

    fn theme(&self) -> Theme {
        Theme::TokyoNight
    }

    fn view(&self) -> Element<'_, Message> {
        let settings = StreamSettings::new(&Theme::TokyoNight);

        let content = stream_view(&self.content, &self.animation, &settings, self.now)
            .map(Message::LinkClicked);

        let header = text("AI Assistant").size(14);

        scrollable(
            center_x(
                container(
                    column![header, content].spacing(12),
                )
                .padding(24)
                .max_width(800),
            )
            .width(Fill),
        )
        .height(Fill)
        .into()
    }
}
