//! Demo of the Context widget showing AI token usage
//!
//! Run with: cargo run -p iced_ai_elements --example context

use iced::widget::{center, column, text};
use iced::{Element, Task};
use iced_ai_elements::context::Context;
use iced_ai_elements::model::ModelId;
use iced_ai_elements::usage::Usage;

/// Entry point for the context widget demo.
pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Context Widget Demo")
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    Hovered(bool),
}

struct App {
    hovered: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (App { hovered: false }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Hovered(v) => self.hovered = v,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let usage = Usage::new(50_000, 30_000, 5_000, 2_000);
        let model_id = ModelId::parse("anthropic:claude-3.5-sonnet");

        let ctx: Context<'_, Message, iced::Theme> = Context::new()
            .max_tokens(200_000)
            .used_tokens(87_000)
            .usage(usage)
            .model_id(&model_id)
            .on_hover(Message::Hovered);

        let status = if self.hovered {
            "Showing token details"
        } else {
            "Hover over the pill to see token details"
        };

        center(
            column![Element::from(ctx), text(status).size(14),]
                .spacing(20)
                .align_x(iced::Alignment::Center),
        )
        .into()
    }
}
