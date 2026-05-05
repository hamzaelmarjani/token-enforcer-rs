use thiserror::Error;

/// Custom error type for the Token Enforcer library.
#[derive(Debug, Error)]
pub enum EnforcerError {
    /// Error returned when a tenant has exceeded their allocated token budget.
    #[error("Budget exceeded for tenant '{tenant_id}': used {used}, limit {limit}, remaining {remaining}")]
    BudgetExceeded {
        tenant_id: String,
        used: u64,
        limit: u64,
        remaining: u64,
    },

    /// Error returned when a tenant is not found in the system.
    /// Budgets must be set with `set_budget()` before they can be checked.
    #[error("Tenant '{0}' not found — set a budget first with set_budget()")]
    TenantNotFound(String),

    /// Error originating from the Redis connection pool.
    #[error("Redis pool error: {0}")]
    Redis(#[from] deadpool_redis::PoolError),

    /// Error originating from Redis pool creation.
    #[error("Redis pool creation error: {0}")]
    RedisPool(#[from] deadpool_redis::CreatePoolError),

    /// Error originating from Redis command execution.
    #[error("Redis command error: {0}")]
    RedisCmd(#[from] redis::RedisError),

    /// Error indicating a configuration issue.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Error returned by a pluggable flush sink.
    #[error("Flush sink error: {0}")]
    Sink(String),
}

/// A specialized Result type for Token Enforcer operations.
pub type Result<T> = std::result::Result<T, EnforcerError>;
