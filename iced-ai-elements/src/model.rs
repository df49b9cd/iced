//! AI Model identification and pricing

use serde::{Deserialize, Serialize};

/// AI model provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    /// OpenAI models (e.g., gpt-4, gpt-4-turbo)
    OpenAi,
    /// Anthropic models (e.g., claude-3-opus, claude-3.5-sonnet)
    Anthropic,
    /// Google models (e.g., gemini-pro)
    Google,
    /// Ollama local models
    Ollama,
    /// Custom/unknown provider
    Custom,
}

impl Provider {
    /// Parses a provider prefix from a model ID string.
    ///
    /// Examples: "openai:gpt-4" -> OpenAi, "anthropic:claude-3" -> Anthropic
    pub fn parse(s: &str) -> Self {
        let prefix = s.split(':').next().unwrap_or(s).to_lowercase();
        match prefix.as_str() {
            "openai" => Provider::OpenAi,
            "anthropic" => Provider::Anthropic,
            "google" | "gemini" => Provider::Google,
            "ollama" => Provider::Ollama,
            _ => Provider::Custom,
        }
    }
}

/// A parsed model identifier.
///
/// Contains the provider and model name extracted from a string like "openai:gpt-4".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelId {
    /// The provider (e.g., OpenAI, Anthropic)
    pub provider: Provider,
    /// The model name (e.g., "gpt-4", "claude-3-opus")
    pub model: String,
}

impl ModelId {
    /// Parses a model ID string.
    ///
    /// Examples:
    /// - "openai:gpt-4" -> Provider::OpenAi, "gpt-4"
    /// - "anthropic:claude-3.5-sonnet" -> Provider::Anthropic, "claude-3.5-sonnet"
    /// - "ollama:llama2" -> Provider::Ollama, "llama2"
    pub fn parse(s: &str) -> Self {
        let (provider_str, model) = s.split_once(':').map(|(p, m)| (p, m)).unwrap_or(("custom", s));

        Self {
            provider: Provider::parse(provider_str),
            model: model.to_string(),
        }
    }

    /// Returns a display string for this model.
    pub fn display(&self) -> String {
        match self.provider {
            Provider::OpenAi => format!("OpenAI {}", self.model),
            Provider::Anthropic => format!("Anthropic {}", self.model),
            Provider::Google => format!("Gemini {}", self.model),
            Provider::Ollama => format!("Ollama {}", self.model),
            Provider::Custom => self.model.clone(),
        }
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Pricing information for a model (cost per 1 million tokens).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Cost per 1M input tokens (in USD)
    pub input: f64,
    /// Cost per 1M output tokens (in USD)
    pub output: f64,
    /// Cost per 1M reasoning tokens (in USD)
    pub reasoning: f64,
    /// Cost per 1M cached tokens (in USD)
    pub cached: f64,
}

impl ModelPricing {
    /// Creates a new [`ModelPricing`] with the given rates.
    pub fn new(input: f64, output: f64, reasoning: f64, cached: f64) -> Self {
        Self { input, output, reasoning, cached }
    }

    /// Calculates the cost for given usage.
    pub fn calculate_cost(&self, usage: &crate::Usage) -> f64 {
        let input_cost = (usage.input as f64 / 1_000_000.0) * self.input;
        let output_cost = (usage.output as f64 / 1_000_000.0) * self.output;
        let reasoning_cost = (usage.reasoning as f64 / 1_000_000.0) * self.reasoning;
        let cached_cost = (usage.cached as f64 / 1_000_000.0) * self.cached;

        input_cost + output_cost + reasoning_cost + cached_cost
    }
}

/// Returns the default pricing for a given model ID.
///
/// This provides sensible defaults for common models.
/// For production use, you may want to maintain your own pricing table.
pub fn default_pricing(model_id: &ModelId) -> ModelPricing {
    match model_id.provider {
        Provider::OpenAi => match model_id.model.as_str() {
            "gpt-4" => ModelPricing::new(30.0, 60.0, 0.0, 0.0),
            "gpt-4-turbo" | "gpt-4o" => ModelPricing::new(5.0, 15.0, 0.0, 0.0),
            "gpt-4o-mini" => ModelPricing::new(0.15, 0.6, 0.0, 0.0),
            "gpt-3.5-turbo" => ModelPricing::new(0.5, 1.5, 0.0, 0.0),
            _ => ModelPricing::new(0.0, 0.0, 0.0, 0.0),
        },
        Provider::Anthropic => match model_id.model.as_str() {
            "claude-3-opus" => ModelPricing::new(15.0, 75.0, 0.0, 0.0),
            "claude-3-sonnet" => ModelPricing::new(3.0, 15.0, 0.0, 0.0),
            "claude-3.5-sonnet" | "claude-3.5-sonnet-v2" => ModelPricing::new(3.0, 15.0, 0.0, 0.0),
            "claude-3.5-haiku" => ModelPricing::new(0.8, 4.0, 0.0, 0.0),
            "claude-3" => ModelPricing::new(0.0, 0.0, 0.0, 0.0), // Legacy, use specific model
            _ => ModelPricing::new(3.0, 15.0, 0.0, 0.0),
        },
        Provider::Google => ModelPricing::new(0.125, 0.5, 0.0, 0.0), // Gemini Pro defaults
        Provider::Ollama => ModelPricing::new(0.0, 0.0, 0.0, 0.0), // Local, no API cost
        Provider::Custom => ModelPricing::new(0.0, 0.0, 0.0, 0.0),
    }
}
