use axum::{
    routing::post,
    extract::State,
    response::IntoResponse,
    http::StatusCode,
    Router,
};
use token_enforcer::{TokenEnforcer, Config, TokenUsage, EnforcerError};
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Initialize the enforcer
    let enforcer = TokenEnforcer::new(Config::default()).await.unwrap();
    
    // Set a demo budget
    enforcer.set_budget("demo_tenant", 50_000).await.unwrap();

    // Wrap in Arc to share with Axum state
    let shared_enforcer = Arc::new(enforcer);

    let app = Router::new()
        .route("/generate", post(generate_handler))
        .with_state(shared_enforcer);

    println!("Axum server running on http://127.0.0.1:3000");
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn generate_handler(
    State(enforcer): State<Arc<TokenEnforcer>>,
) -> impl IntoResponse {
    let tenant_id = "demo_tenant";

    // 1. Pre-check: Does the tenant have budget?
    if let Err(e) = enforcer.check(tenant_id, 1000).await {
        return match e {
            EnforcerError::BudgetExceeded { remaining, .. } => {
                (StatusCode::TOO_MANY_REQUESTS, format!("Budget exceeded. Remaining: {}", remaining))
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
    }

    // 2. Simulate LLM API call
    // ... llm call ...
    let actual_usage = TokenUsage { input_tokens: 200, output_tokens: 150 };

    // 3. Record actual usage
    if let Err(e) = enforcer.record(tenant_id, actual_usage).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }

    (StatusCode::OK, "Tokens consumed successfully".to_string())
}
