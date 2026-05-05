use actix_web::{web, App, HttpServer, HttpResponse, post};
use token_enforcer::{TokenEnforcer, Config, TokenUsage, EnforcerError};

#[post("/generate")]
async fn generate(enforcer: web::Data<TokenEnforcer>) -> HttpResponse {
    let tenant_id = "tenant_xyz";

    // 1. Pre-check
    match enforcer.check(tenant_id, 800).await {
        Err(EnforcerError::BudgetExceeded { remaining, .. }) => {
            HttpResponse::TooManyRequests().body(format!("Budget exceeded. Remaining: {}", remaining))
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
        Ok(()) => {
            // 2. Simulate LLM usage
            let usage = TokenUsage { input_tokens: 300, output_tokens: 100 };
            
            // 3. Record
            if let Err(e) = enforcer.record(tenant_id, usage).await {
                return HttpResponse::InternalServerError().body(e.to_string());
            }
            
            HttpResponse::Ok().body("Generated")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize the enforcer
    let enforcer = TokenEnforcer::new(Config::default()).await.unwrap();
    
    // Set budget
    enforcer.set_budget("tenant_xyz", 200_000).await.unwrap();
    
    // Wrap in web::Data to share with Actix
    let enforcer_data = web::Data::new(enforcer);

    println!("Actix-Web server running on http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .app_data(enforcer_data.clone())
            .service(generate)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
