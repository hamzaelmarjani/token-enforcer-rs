use token_enforcer::{TokenEnforcer, Config, TokenUsage};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the enforcer with default config (pointing to localhost Redis)
    let enforcer = TokenEnforcer::new(Config::default()
        .with_flush_interval(Duration::from_secs(5))
    ).await?;

    let tenant_id = "tenant_abc";

    // Set budget for a tenant
    println!("Setting budget for {}...", tenant_id);
    enforcer.set_budget(tenant_id, 100_000).await?;

    // Before LLM call: Check if budget allows 500 tokens
    println!("Checking budget...");
    enforcer.check(tenant_id, 500).await?;

    // Simulate LLM call — after it returns, record actual usage:
    println!("Recording usage...");
    enforcer.record(tenant_id, TokenUsage {
        input_tokens: 312,
        output_tokens: 189,
    }).await?;

    // Check current status
    let status = enforcer.status(tenant_id).await?;
    println!("Budget Status for {}:", tenant_id);
    println!("  Used: {} / {}", status.used, status.limit);
    println!("  Remaining: {}", status.remaining);
    println!("  Usage: {:.1}%", status.percentage_used * 100.0);

    // Clean up (optional)
    // enforcer.remove_tenant(tenant_id).await?;

    Ok(())
}
