use serde::{Deserialize, Serialize};

/// Supported LLM providers.
///
/// Serialised to/from lowercase strings (e.g. `"openai"`, `"gemini"`, `"anthropic"`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Providers {
    /// Anthropic (Claude models via `api.anthropic.com`).
    Anthropic,
    /// OpenAI-compatible API (works with local servers like llama.cpp).
    OpenAI,
    /// Google Gemini API.
    Gemini,
}

impl Providers {
    /// Returns the lowercase string representation of this provider.
    ///
    /// Useful for serialising the provider name for the frontend or for
    /// dynamic dispatch in the agent builder.
    ///
    /// # Returns
    ///
    /// A static string: `"openai"`, `"gemini"`, or `"anthropic"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Providers::OpenAI => "openai",
            Providers::Gemini => "gemini",
            Providers::Anthropic => "anthropic",
        }
    }

    /// Returns all supported provider variants in a fixed order.
    ///
    /// # Returns
    ///
    /// A vector containing `OpenAI`, `Gemini`, and `Anthropic`.
    pub fn all() -> Vec<Self> {
        vec![Providers::OpenAI, Providers::Gemini, Providers::Anthropic]
    }
}

impl std::fmt::Display for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Providers {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(Providers::OpenAI),
            "gemini" => Ok(Providers::Gemini),
            "anthropic" => Ok(Providers::Anthropic),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}
