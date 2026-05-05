pub mod enforcer;
pub mod config;
pub mod models;
pub mod error;
pub mod flush;
mod redis;

pub use enforcer::{TokenEnforcer, ReservationToken};
pub use config::Config;
pub use models::{TokenUsage, BudgetStatus, UsageEvent};
pub use error::{EnforcerError, Result};
pub use flush::FlushSink;

/// Compile-time assertion to ensure TokenEnforcer is Send and Sync.
#[allow(dead_code)]
fn _assert_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<TokenEnforcer>();
}
