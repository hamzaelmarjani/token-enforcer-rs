use deadpool_redis::Pool;
use deadpool_redis::redis::{AsyncCommands, Script};
use crate::error::Result;

/// Lua script to atomically decrement usage with a floor of 0.
/// This prevents usage from becoming negative due to refunds or optimistic settling.
const DECREMENT_SCRIPT: &str = r#"
    local current = redis.call('GET', KEYS[1])
    if not current then
        return 0
    end
    local new_val = tonumber(current) - tonumber(ARGV[1])
    if new_val < 0 then
        new_val = 0
    end
    redis.call('SET', KEYS[1], new_val)
    return new_val
"#;

/// Internal helper to construct a budget key.
fn budget_key(prefix: &str, tenant_id: &str) -> String {
    format!("{}:budget:{}", prefix, tenant_id)
}

/// Internal helper to construct a usage key.
fn usage_key(prefix: &str, tenant_id: &str) -> String {
    format!("{}:usage:{}", prefix, tenant_id)
}

/// Set the token budget for a tenant.
///
/// # Arguments
/// * `pool` - Redis connection pool.
/// * `prefix` - Key prefix for namespacing.
/// * `tenant_id` - ID of the tenant.
/// * `limit` - Token limit to set.
pub async fn set_budget(pool: &Pool, prefix: &str, tenant_id: &str, limit: u64) -> Result<()> {
    let mut conn = pool.get().await?;
    let key = budget_key(prefix, tenant_id);
    let _: () = conn.set(key, limit).await?;
    Ok(())
}

/// Get the budget limit for a tenant.
/// Returns `None` if no budget has been set for this tenant.
pub async fn get_budget(pool: &Pool, prefix: &str, tenant_id: &str) -> Result<Option<u64>> {
    let mut conn = pool.get().await?;
    let key = budget_key(prefix, tenant_id);
    let limit: Option<u64> = conn.get(key).await?;
    Ok(limit)
}

/// Get the current usage for a tenant.
/// Returns 0 if the usage key does not exist.
pub async fn get_usage(pool: &Pool, prefix: &str, tenant_id: &str) -> Result<u64> {
    let mut conn = pool.get().await?;
    let key = usage_key(prefix, tenant_id);
    let usage: Option<u64> = conn.get(key).await?;
    Ok(usage.unwrap_or(0))
}

/// Atomically increment usage by a delta.
/// Returns the new total usage value.
pub async fn increment_usage(pool: &Pool, prefix: &str, tenant_id: &str, delta: u64) -> Result<u64> {
    let mut conn = pool.get().await?;
    let key = usage_key(prefix, tenant_id);
    let new_usage: u64 = conn.incr(key, delta).await?;
    Ok(new_usage)
}

/// Atomically decrement usage by a delta, with a floor at 0.
/// Returns the new total usage value.
pub async fn decrement_usage(pool: &Pool, prefix: &str, tenant_id: &str, delta: u64) -> Result<u64> {
    let mut conn = pool.get().await?;
    let key = usage_key(prefix, tenant_id);
    let script = Script::new(DECREMENT_SCRIPT);
    let new_usage: u64 = script.key(key).arg(delta).invoke_async(&mut *conn).await?;
    Ok(new_usage)
}

/// Delete all keys for a tenant (budget and usage).
pub async fn delete_tenant(pool: &Pool, prefix: &str, tenant_id: &str) -> Result<()> {
    let mut conn = pool.get().await?;
    let b_key = budget_key(prefix, tenant_id);
    let u_key = usage_key(prefix, tenant_id);
    let _: () = conn.del(&[b_key, u_key]).await?;
    Ok(())
}

/// Reset usage for a tenant to zero without deleting the budget.
pub async fn reset_usage(pool: &Pool, prefix: &str, tenant_id: &str) -> Result<()> {
    let mut conn = pool.get().await?;
    let key = usage_key(prefix, tenant_id);
    let _: () = conn.set(key, 0).await?;
    Ok(())
}
