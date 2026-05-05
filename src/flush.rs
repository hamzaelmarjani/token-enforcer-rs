use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex};
use async_trait::async_trait;
use tracing::{error, info};

use crate::models::UsageEvent;
use crate::error::Result;

/// A trait for persisting token usage events to an external store.
/// Developers should implement this trait to save usage data to their primary database.
#[async_trait]
pub trait FlushSink: Send + Sync {
    /// Flushes a batch of usage events to the underlying storage.
    ///
    /// # Arguments
    /// * `events` - A vector of `UsageEvent` objects to be persisted.
    async fn flush(&self, events: Vec<UsageEvent>) -> Result<()>;
}

/// Internal function to start the background task that periodically flushes events.
///
/// # Arguments
/// * `queue` - Shared thread-safe queue of pending events.
/// * `interval` - How often to attempt a flush.
/// * `sink` - Optional implementation of `FlushSink` to receive events.
/// * `shutdown_rx` - Watch channel to signal task shutdown.
pub async fn start_flush_task(
    queue: Arc<Mutex<Vec<UsageEvent>>>,
    interval: Duration,
    sink: Option<Arc<dyn FlushSink>>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if let Err(e) = flush_events(&queue, &sink).await {
                    error!(error = %e, "Background flush failed");
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    info!("Shutdown signal received, performing final flush");
                    if let Err(e) = flush_events(&queue, &sink).await {
                        error!(error = %e, "Final background flush failed");
                    }
                    break;
                }
            }
        }
    }
}

/// Internal helper to drain the queue and send events to the sink.
///
/// # Arguments
/// * `queue` - Shared thread-safe queue of pending events.
/// * `sink` - Optional implementation of `FlushSink` to receive events.
async fn flush_events(
    queue: &Arc<Mutex<Vec<UsageEvent>>>,
    sink: &Option<Arc<dyn FlushSink>>,
) -> Result<()> {
    let events = {
        let mut q = queue.lock().await;
        if q.is_empty() {
            return Ok(());
        }
        std::mem::take(&mut *q)
    };

    if let Some(sink) = sink {
        sink.flush(events).await?;
    }

    Ok(())
}
