# token-enforcer-rs

Per-tenant LLM token budget enforcement library for Rust apps.

`token-enforcer-rs` is a lightweight library that helps you manage and enforce token budgets for LLM API calls (like OpenAI, Anthropic, etc.) across different tenants or users. It uses Redis as a high-performance counter store and provides a pluggable sink for long-term persistence.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
token-enforcer = { git = "https://github.com/hamzaelmarjani/token-enforcer-rs" }
tokio = { version = "1", features = ["full"] }
```

## Quick Start

```rust
use token_enforcer::{TokenEnforcer, Config, TokenUsage};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Initialize
    let enforcer = TokenEnforcer::new(Config::default()).await?;

    // 2. Set a budget for a tenant
    enforcer.set_budget("tenant_123", 100_000).await?;

    // 3. Check before an LLM call
    enforcer.check("tenant_123", 500).await?;

    // 4. Record after the call returns
    enforcer.record("tenant_123", TokenUsage {
        input_tokens: 312,
        output_tokens: 189,
    }).await?;

    // 5. Check status
    let status = enforcer.status("tenant_123").await?;
    println!("Remaining: {}", status.remaining);

    Ok(())
}
```

## Core API

| Method | Description |
|--------|-------------|
| `check(tenant_id, estimated)` | Validates if a tenant has enough budget remaining. |
| `record(tenant_id, usage)` | Increments usage in Redis and queues an event for flushing. |
| `check_and_reserve(tenant_id, estimated)` | Optimistically reserve tokens before a call. |
| `settle_reservation(token, actual)` | Adjusts the reservation based on actual usage. |
| `set_budget(tenant_id, limit)` | Sets or updates the token limit for a tenant. |
| `status(tenant_id)` | Returns the current budget usage and limit. |
| `reset_usage(tenant_id)` | Resets the current usage counter to zero. |

## Framework Integration

The library is designed to be easily integrated into any Rust framework.

### Axum Example

```rust
async fn handler(State(enforcer): State<Arc<TokenEnforcer>>) -> impl IntoResponse {
    enforcer.check("user_1", 1000).await?;
    // ... call LLM ...
    enforcer.record("user_1", usage).await?;
    Ok("Success")
}
```

### Actix-Web Example

```rust
async fn handler(enforcer: web::Data<TokenEnforcer>) -> HttpResponse {
    enforcer.check("user_1", 1000).await.unwrap();
    // ... call LLM ...
    enforcer.record("user_1", usage).await.unwrap();
    HttpResponse::Ok().finish()
}
```

## Persistence with `FlushSink`

Implement the `FlushSink` trait to save usage events to your primary database (e.g., PostgreSQL).

```rust
use async_trait::async_trait;
use token_enforcer::{FlushSink, UsageEvent, Result};

pub struct MyPostgresSink { 
    pool: sqlx::PgPool 
}

#[async_trait]
impl FlushSink for MyPostgresSink {
    async fn flush(&self, events: Vec<UsageEvent>) -> Result<()> {
        // Insert events into your database here
        Ok(())
    }
}

// Usage:
// let config = Config::default().with_sink(Arc::new(MyPostgresSink { ... }));
```

## Redis Key Schema

The library uses the following key structure in Redis:

- `{prefix}:budget:{tenant_id}`: Store the `u64` budget limit.
- `{prefix}:usage:{tenant_id}`: Store the `u64` current usage counter.

## Note
This library manages token counting and enforcement logic. It does **not** manage your users/tenants database or provide an HTTP server.

## License
MIT
