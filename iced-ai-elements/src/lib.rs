//! # iced-ai-elements
//!
//! AI-themed components for the iced GUI library.
//!
//! This crate provides components like [`Context`] for displaying
//! AI model context window usage, token consumption, and cost estimation.
//!
//! # Example
//!
//! ```ignore
//! use iced_ai_elements::{Context, Usage, ModelId};
//!
//! Context::new()
//!     .max_tokens(200_000)
//!     .used_tokens(85_000)
//!     .usage(Usage::new(50_000, 30_000, 5_000, 0))
//!     .model_id(&ModelId::parse("openai:gpt-4"))
//! ```

pub mod context;
pub mod model;
pub mod usage;

pub use context::{format_cost, format_tokens, Catalog, Context, Style, StyleFn};
pub use model::{default_pricing, ModelId, ModelPricing, Provider};
pub use usage::Usage;
