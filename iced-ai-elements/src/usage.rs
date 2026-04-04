//! Token usage types

/// Token usage breakdown for an AI model request.
///
/// Represents the number of tokens used in different categories:
/// - **Input**: Tokens sent to the model
/// - **Output**: Tokens received from the model
/// - **Reasoning**: Tokens used for chain-of-thought reasoning (if supported)
/// - **Cached**: Tokens from context caching (if supported)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Usage {
    /// Number of input tokens
    pub input: u64,
    /// Number of output tokens
    pub output: u64,
    /// Number of reasoning tokens (if applicable)
    pub reasoning: u64,
    /// Number of cached tokens (if applicable)
    pub cached: u64,
}

impl Usage {
    /// Creates a new [`Usage`] with the given token counts.
    pub fn new(input: u64, output: u64, reasoning: u64, cached: u64) -> Self {
        Self { input, output, reasoning, cached }
    }

    /// Returns the total number of tokens used.
    pub fn total(&self) -> u64 {
        self.input + self.output + self.reasoning + self.cached
    }
}

impl std::ops::Add for Usage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            input: self.input + other.input,
            output: self.output + other.output,
            reasoning: self.reasoning + other.reasoning,
            cached: self.cached + other.cached,
        }
    }
}
