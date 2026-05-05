use serde::{Deserialize, Serialize};

/// Actual token usage returned from an LLM API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of tokens used in the input/prompt.
    pub input_tokens: u64,
    /// Number of tokens used in the output/completion.
    pub output_tokens: u64,
}

impl TokenUsage {
    /// Returns the total number of tokens (input + output).
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Current budget state for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// The unique identifier for the tenant.
    pub tenant_id: String,
    /// Number of tokens already used.
    pub used: u64,
    /// The maximum allowed token limit.
    pub limit: u64,
    /// Remaining tokens available (limit - used).
    pub remaining: u64,
    /// Usage as a fraction between 0.0 and 1.0.
    pub percentage_used: f64,
}

/// A usage event recorded after an LLM call.
/// These events are passed to the `FlushSink` for persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    /// The unique identifier for the tenant.
    pub tenant_id: String,
    /// Number of tokens used in the input.
    pub input_tokens: u64,
    /// Number of tokens used in the output.
    pub output_tokens: u64,
    /// Total tokens used in this event.
    pub total_tokens: u64,
    /// UTC Unix timestamp in seconds when the event was recorded.
    pub timestamp: u64,
    /// Optional model name used for the request (e.g., "gpt-4o").
    pub model: Option<String>,
}
