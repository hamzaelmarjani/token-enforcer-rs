// These tests require a running Redis instance at redis://127.0.0.1:6379
// Run with: cargo test --test integration

use token_enforcer::{TokenEnforcer, Config, TokenUsage, EnforcerError};
use uuid::Uuid;
use std::time::Duration;

async fn setup_enforcer() -> TokenEnforcer {
    TokenEnforcer::new(Config::default()
        .with_key_prefix("test_te")
        .with_flush_interval(Duration::from_millis(100))
    ).await.expect("Failed to create enforcer")
}

fn next_id() -> String {
    Uuid::new_v4().to_string()
}

#[tokio::test]
async fn test_set_and_check_budget() {
    let te = setup_enforcer().await;
    let id = next_id();

    te.set_budget(&id, 1000).await.unwrap();
    
    // Should be OK
    te.check(&id, 500).await.expect("Budget check failed");
}

#[tokio::test]
async fn test_budget_exceeded() {
    let te = setup_enforcer().await;
    let id = next_id();

    te.set_budget(&id, 100).await.unwrap();
    
    // Record 90
    te.record(&id, TokenUsage { input_tokens: 50, output_tokens: 40 }).await.unwrap();
    
    // Check 20 -> should fail (90 + 20 = 110 > 100)
    let result = te.check(&id, 20).await;
    assert!(matches!(result, Err(EnforcerError::BudgetExceeded { .. })));
}

#[tokio::test]
async fn test_record_increments_usage() {
    let te = setup_enforcer().await;
    let id = next_id();

    te.set_budget(&id, 1000).await.unwrap();
    
    te.record(&id, TokenUsage { input_tokens: 100, output_tokens: 50 }).await.unwrap();
    te.record(&id, TokenUsage { input_tokens: 200, output_tokens: 50 }).await.unwrap();
    
    let status = te.status(&id).await.unwrap();
    assert_eq!(status.used, 400);
}

#[tokio::test]
async fn test_tenant_not_found() {
    let te = setup_enforcer().await;
    let id = next_id();

    let result = te.check(&id, 100).await;
    assert!(matches!(result, Err(EnforcerError::TenantNotFound(_))));
}

#[tokio::test]
async fn test_reset_usage() {
    let te = setup_enforcer().await;
    let id = next_id();

    te.set_budget(&id, 1000).await.unwrap();
    te.record(&id, TokenUsage { input_tokens: 100, output_tokens: 100 }).await.unwrap();
    
    let status = te.status(&id).await.unwrap();
    assert_eq!(status.used, 200);

    te.reset_usage(&id).await.unwrap();
    
    let status = te.status(&id).await.unwrap();
    assert_eq!(status.used, 0);
}

#[tokio::test]
async fn test_optimistic_reservation_overspend() {
    let te = setup_enforcer().await;
    let id = next_id();

    te.set_budget(&id, 1000).await.unwrap();
    
    let token = te.check_and_reserve(&id, 500).await.unwrap();
    
    // Actual is 600
    te.settle_reservation(token, TokenUsage { input_tokens: 300, output_tokens: 300 }).await.unwrap();
    
    let status = te.status(&id).await.unwrap();
    assert_eq!(status.used, 600);
}

#[tokio::test]
async fn test_optimistic_reservation_refund() {
    let te = setup_enforcer().await;
    let id = next_id();

    te.set_budget(&id, 1000).await.unwrap();
    
    let token = te.check_and_reserve(&id, 500).await.unwrap();
    
    // Actual is 300
    te.settle_reservation(token, TokenUsage { input_tokens: 150, output_tokens: 150 }).await.unwrap();
    
    let status = te.status(&id).await.unwrap();
    assert_eq!(status.used, 300);
}
