use std::sync::Arc;
use tokio::sync::{watch, Mutex};
use deadpool_redis::{Config as RedisConfig, Pool, Runtime};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;
use crate::error::{EnforcerError, Result};
use crate::models::{BudgetStatus, TokenUsage, UsageEvent};
use crate::redis;
use crate::flush;

/// A token for a pre-reserved token amount.
#[derive(Debug, Clone)]
pub struct ReservationToken {
    /// The tenant for which the reservation was made.
    pub tenant_id: String,
    /// The number of tokens that were reserved.
    pub reserved: u64,
}

/// The core enforcer that manages token budgets and usage.
///
/// This struct is `Clone`, `Send`, and `Sync`. It uses an internal `Arc` to share state.
#[derive(Clone)]
pub struct TokenEnforcer {
    pool: Arc<Pool>,
    config: Arc<Config>,
    event_queue: Arc<Mutex<Vec<UsageEvent>>>,
    shutdown_tx: watch::Sender<bool>,
}

impl TokenEnforcer {
    /// Creates a new `TokenEnforcer` with the given configuration.
    ///
    /// This will initialize the Redis connection pool and start the background flush task.
    ///
    /// # Errors
    /// Returns an error if the Redis pool cannot be created.
    pub async fn new(config: Config) -> Result<Self> {
        let redis_cfg = RedisConfig::from_url(&config.redis_url);
        let pool = redis_cfg.create_pool(Some(Runtime::Tokio1))?;
        
        let pool = Arc::new(pool);
        let config = Arc::new(config);
        let event_queue = Arc::new(Mutex::new(Vec::new()));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        // Start background flush task
        tokio::spawn(flush::start_flush_task(
            event_queue.clone(),
            config.flush_interval,
            config.sink.clone(),
            shutdown_rx,
        ));

        Ok(Self {
            pool,
            config,
            event_queue,
            shutdown_tx,
        })
    }

    /// Checks if a tenant has enough budget for an estimated number of tokens.
    ///
    /// This should be called BEFORE an LLM request.
    ///
    /// # Errors
    /// * `EnforcerError::TenantNotFound` if the tenant has no budget set.
    /// * `EnforcerError::BudgetExceeded` if the current usage plus `estimated_tokens` exceeds the limit.
    pub async fn check(&self, tenant_id: &str, estimated_tokens: u64) -> Result<()> {
        let limit = redis::get_budget(&self.pool, &self.config.key_prefix, tenant_id).await?
            .ok_or_else(|| EnforcerError::TenantNotFound(tenant_id.to_string()))?;

        let used = redis::get_usage(&self.pool, &self.config.key_prefix, tenant_id).await?;

        if used + estimated_tokens > limit {
            return Err(EnforcerError::BudgetExceeded {
                tenant_id: tenant_id.to_string(),
                used,
                limit,
                remaining: limit.saturating_sub(used),
            });
        }

        Ok(())
    }

    /// Records actual token usage after an LLM request.
    ///
    /// This will increment the usage in Redis and queue a `UsageEvent` for the background flush.
    pub async fn record(&self, tenant_id: &str, usage: TokenUsage) -> Result<()> {
        self.record_internal(tenant_id, usage, None).await
    }

    /// Records actual token usage with a specific model name.
    pub async fn record_with_model(&self, tenant_id: &str, usage: TokenUsage, model: &str) -> Result<()> {
        self.record_internal(tenant_id, usage, Some(model.to_string())).await
    }

    async fn record_internal(&self, tenant_id: &str, usage: TokenUsage, model: Option<String>) -> Result<()> {
        let total = usage.total();
        redis::increment_usage(&self.pool, &self.config.key_prefix, tenant_id, total).await?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = UsageEvent {
            tenant_id: tenant_id.to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: total,
            timestamp,
            model,
        };

        let mut queue = self.event_queue.lock().await;
        queue.push(event);

        Ok(())
    }

    /// Performs an optimistic reservation of tokens.
    ///
    /// This increments usage in Redis immediately. Use `settle_reservation` to adjust 
    /// based on actual usage later.
    pub async fn check_and_reserve(&self, tenant_id: &str, estimated_tokens: u64) -> Result<ReservationToken> {
        self.check(tenant_id, estimated_tokens).await?;
        
        redis::increment_usage(&self.pool, &self.config.key_prefix, tenant_id, estimated_tokens).await?;

        Ok(ReservationToken {
            tenant_id: tenant_id.to_string(),
            reserved: estimated_tokens,
        })
    }

    /// Settles an optimistic reservation with actual usage data.
    ///
    /// This adjusts the Redis usage counter (refunding or adding extra) and records the event.
    pub async fn settle_reservation(&self, reservation: ReservationToken, actual: TokenUsage) -> Result<()> {
        let actual_total = actual.total();
        
        if actual_total >= reservation.reserved {
            let extra = actual_total - reservation.reserved;
            if extra > 0 {
                redis::increment_usage(&self.pool, &self.config.key_prefix, &reservation.tenant_id, extra).await?;
            }
        } else {
            let refund = reservation.reserved - actual_total;
            redis::decrement_usage(&self.pool, &self.config.key_prefix, &reservation.tenant_id, refund).await?;
        }

        // Record the event
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let event = UsageEvent {
            tenant_id: reservation.tenant_id,
            input_tokens: actual.input_tokens,
            output_tokens: actual.output_tokens,
            total_tokens: actual_total,
            timestamp,
            model: None,
        };

        let mut queue = self.event_queue.lock().await;
        queue.push(event);

        Ok(())
    }

    /// Sets or updates the token budget for a tenant.
    pub async fn set_budget(&self, tenant_id: &str, limit: u64) -> Result<()> {
        redis::set_budget(&self.pool, &self.config.key_prefix, tenant_id, limit).await
    }

    /// Gets the current budget status for a tenant.
    pub async fn status(&self, tenant_id: &str) -> Result<BudgetStatus> {
        let limit = redis::get_budget(&self.pool, &self.config.key_prefix, tenant_id).await?
            .ok_or_else(|| EnforcerError::TenantNotFound(tenant_id.to_string()))?;

        let used = redis::get_usage(&self.pool, &self.config.key_prefix, tenant_id).await?;
        let remaining = limit.saturating_sub(used);
        let percentage_used = if limit > 0 {
            used as f64 / limit as f64
        } else {
            1.0
        };

        Ok(BudgetStatus {
            tenant_id: tenant_id.to_string(),
            used,
            limit,
            remaining,
            percentage_used,
        })
    }

    /// Resets usage for a tenant to zero.
    pub async fn reset_usage(&self, tenant_id: &str) -> Result<()> {
        redis::reset_usage(&self.pool, &self.config.key_prefix, tenant_id).await
    }

    /// Removes a tenant and all their data from the enforcer.
    pub async fn remove_tenant(&self, tenant_id: &str) -> Result<()> {
        redis::delete_tenant(&self.pool, &self.config.key_prefix, tenant_id).await
    }

    /// Signals the background task to shutdown.
    /// 
    /// The task will perform one final flush before exiting.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

impl Drop for TokenEnforcer {
    fn drop(&mut self) {
        // We try to signal shutdown, but since it's an async task it might not 
        // complete if the runtime is also shutting down. 
        // In most cases, the explicit `shutdown()` should be called if graceful 
        // exit is critical.
        let _ = self.shutdown_tx.send(true);
    }
}
