//! AI Model identification and pricing

/// AI model provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Creates a [`ModelId`] directly from a provider and model name.
    ///
    /// Use this when the provider is already known, avoiding the string
    /// parsing overhead of [`ModelId::parse`].
    pub fn from_parts(provider: Provider, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
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
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// A pricing table entry mapping a provider prefix and model pattern to pricing.
struct PricingEntry {
    prefix: &'static str,
    model: &'static str,
    pricing: ModelPricing,
}

/// Built-in pricing table (last updated: 2026-04).
///
/// Entries are searched top-to-bottom; the first match wins. An empty `model`
/// string acts as a provider-level fallback.
///
/// IMPORTANT: more-specific patterns (e.g., "gpt-4.1-mini") MUST appear before
/// shorter prefixes ("gpt-4.1") because matching uses `starts_with`.
///
/// Pricing is in USD per 1 million tokens. Rates shown here are approximate
/// and may become outdated — for production use, override via
/// [`Context::pricing`](crate::Context::pricing).
const PRICING_TABLE: &[PricingEntry] = &[
    // OpenAI — longer model names before shorter prefixes
    PricingEntry { prefix: "openai", model: "gpt-4o-mini", pricing: ModelPricing { input: 0.15, output: 0.6, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "gpt-4o", pricing: ModelPricing { input: 2.5, output: 10.0, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "gpt-4-turbo", pricing: ModelPricing { input: 10.0, output: 30.0, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "gpt-4.1-nano", pricing: ModelPricing { input: 0.1, output: 0.4, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "gpt-4.1-mini", pricing: ModelPricing { input: 0.4, output: 1.6, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "gpt-4.1", pricing: ModelPricing { input: 2.0, output: 8.0, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "o3-mini", pricing: ModelPricing { input: 1.1, output: 4.4, reasoning: 4.4, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "o4-mini", pricing: ModelPricing { input: 1.1, output: 4.4, reasoning: 4.4, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "o3", pricing: ModelPricing { input: 2.0, output: 8.0, reasoning: 8.0, cached: 0.0 } },
    PricingEntry { prefix: "openai", model: "", pricing: ModelPricing { input: 0.0, output: 0.0, reasoning: 0.0, cached: 0.0 } },
    // Anthropic
    PricingEntry { prefix: "anthropic", model: "claude-opus-4", pricing: ModelPricing { input: 15.0, output: 75.0, reasoning: 0.0, cached: 1.875 } },
    PricingEntry { prefix: "anthropic", model: "claude-sonnet-4", pricing: ModelPricing { input: 3.0, output: 15.0, reasoning: 0.0, cached: 0.375 } },
    PricingEntry { prefix: "anthropic", model: "claude-3.5-sonnet", pricing: ModelPricing { input: 3.0, output: 15.0, reasoning: 0.0, cached: 0.375 } },
    PricingEntry { prefix: "anthropic", model: "claude-3.5-haiku", pricing: ModelPricing { input: 0.8, output: 4.0, reasoning: 0.0, cached: 0.1 } },
    PricingEntry { prefix: "anthropic", model: "claude-3-opus", pricing: ModelPricing { input: 15.0, output: 75.0, reasoning: 0.0, cached: 1.875 } },
    PricingEntry { prefix: "anthropic", model: "claude-3-sonnet", pricing: ModelPricing { input: 3.0, output: 15.0, reasoning: 0.0, cached: 0.375 } },
    PricingEntry { prefix: "anthropic", model: "", pricing: ModelPricing { input: 3.0, output: 15.0, reasoning: 0.0, cached: 0.375 } },
    // Google
    PricingEntry { prefix: "google", model: "gemini-2.5-pro", pricing: ModelPricing { input: 1.25, output: 10.0, reasoning: 0.0, cached: 0.315 } },
    PricingEntry { prefix: "google", model: "gemini-2.5-flash", pricing: ModelPricing { input: 0.15, output: 0.6, reasoning: 0.6, cached: 0.0375 } },
    PricingEntry { prefix: "google", model: "", pricing: ModelPricing { input: 0.15, output: 0.6, reasoning: 0.0, cached: 0.0 } },
    PricingEntry { prefix: "gemini", model: "", pricing: ModelPricing { input: 0.15, output: 0.6, reasoning: 0.0, cached: 0.0 } },
    // Local / free
    PricingEntry { prefix: "ollama", model: "", pricing: ModelPricing { input: 0.0, output: 0.0, reasoning: 0.0, cached: 0.0 } },
];

/// Returns the default pricing for a given model ID.
///
/// Looks up pricing from a built-in table of common models. An empty `model`
/// pattern in the table acts as a provider-level fallback. Returns zero pricing
/// for completely unknown providers.
///
/// For production use, override via [`Context::pricing`](crate::Context::pricing)
/// or [`ModelPricing::new`].
pub fn default_pricing(model_id: &ModelId) -> ModelPricing {
    let prefix = match model_id.provider {
        Provider::OpenAi => "openai",
        Provider::Anthropic => "anthropic",
        Provider::Google => "google",
        Provider::Ollama => "ollama",
        Provider::Custom => "",
    };

    for entry in PRICING_TABLE {
        if entry.prefix == prefix
            && (entry.model.is_empty() || model_id.model.starts_with(entry.model))
        {
            return entry.pricing;
        }
    }

    ModelPricing::new(0.0, 0.0, 0.0, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{format_cost, format_tokens};

    // --- format_tokens ---

    #[test]
    fn format_tokens_small() {
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn format_tokens_thousands() {
        assert_eq!(format_tokens(1_000), "1K");
        assert_eq!(format_tokens(1_500), "1.5K");
        assert_eq!(format_tokens(32_000), "32K");
    }

    #[test]
    fn format_tokens_boundary() {
        // 999_999 is < 1M so enters K branch
        assert_eq!(format_tokens(999_999), "1000.0K");
    }

    #[test]
    fn format_tokens_millions() {
        assert_eq!(format_tokens(1_000_000), "1.0M");
        assert_eq!(format_tokens(128_000_000), "128.0M");
    }

    #[test]
    fn format_tokens_billions() {
        assert_eq!(format_tokens(1_000_000_000), "1.0B");
    }

    // --- format_cost ---

    #[test]
    fn format_cost_small() {
        assert_eq!(format_cost(0.001), "$0.0010");
    }

    #[test]
    fn format_cost_normal() {
        assert_eq!(format_cost(1.50), "$1.50");
        assert_eq!(format_cost(0.04), "$0.04");
    }

    // --- ModelId::parse ---

    #[test]
    fn parse_openai() {
        let id = ModelId::parse("openai:gpt-4");
        assert_eq!(id.provider, Provider::OpenAi);
        assert_eq!(id.model, "gpt-4");
    }

    #[test]
    fn parse_anthropic() {
        let id = ModelId::parse("anthropic:claude-3.5-sonnet");
        assert_eq!(id.provider, Provider::Anthropic);
        assert_eq!(id.model, "claude-3.5-sonnet");
    }

    #[test]
    fn parse_no_colon() {
        let id = ModelId::parse("llama2");
        assert_eq!(id.provider, Provider::Custom);
        assert_eq!(id.model, "llama2");
    }

    #[test]
    fn parse_with_extra_colons() {
        let id = ModelId::parse("openai:gpt-4:latest");
        assert_eq!(id.provider, Provider::OpenAi);
        assert_eq!(id.model, "gpt-4:latest");
    }

    // --- default_pricing ---

    #[test]
    fn pricing_gpt4o_mini() {
        let id = ModelId::parse("openai:gpt-4o-mini");
        let p = default_pricing(&id);
        assert_eq!(p.input, 0.15);
    }

    #[test]
    fn from_parts_avoids_parse() {
        let id = ModelId::from_parts(Provider::OpenAi, "gpt-4");
        assert_eq!(id.provider, Provider::OpenAi);
        assert_eq!(id.model, "gpt-4");
    }

    #[test]
    fn pricing_gpt41_mini_not_gpt41() {
        let id = ModelId::parse("openai:gpt-4.1-mini");
        let p = default_pricing(&id);
        assert_eq!(p.input, 0.4); // must NOT match gpt-4.1's $2.0
    }

    #[test]
    fn pricing_gpt41_nano() {
        let id = ModelId::parse("openai:gpt-4.1-nano");
        let p = default_pricing(&id);
        assert_eq!(p.input, 0.1);
    }

    #[test]
    fn pricing_o3_mini_not_o3() {
        let id = ModelId::parse("openai:o3-mini");
        let p = default_pricing(&id);
        assert_eq!(p.input, 1.1); // must NOT match o3's $2.0
    }

    #[test]
    fn pricing_unknown_provider() {
        let id = ModelId::parse("custom:mystery-model");
        let p = default_pricing(&id);
        assert_eq!(p.input, 0.0);
    }

    // --- ModelPricing::calculate_cost ---

    #[test]
    fn calculate_cost_basic() {
        let pricing = ModelPricing::new(3.0, 15.0, 0.0, 0.375);
        let usage = crate::Usage::new(50_000, 30_000, 0, 10_000);
        let cost = pricing.calculate_cost(&usage);
        let expected = (50_000.0 / 1e6) * 3.0
            + (30_000.0 / 1e6) * 15.0
            + (10_000.0 / 1e6) * 0.375;
        assert!((cost - expected).abs() < 1e-10);
    }
}
