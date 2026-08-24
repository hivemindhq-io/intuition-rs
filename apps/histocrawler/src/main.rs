use app::HistoCrawler;
use error::HistoCrawlerError;
use log::{error, info};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;

mod app;
mod error;

#[derive(Debug, Deserialize)]
pub struct Env {
    pub histocrawler_database_url: String,
    pub indexer_schema: String,
    /// Cap adaptive block-window size (default 2000). Lower = gentler on public RPCs.
    #[serde(default = "default_max_batch_size")]
    pub histocrawler_max_batch_size: i64,
    /// Cap logs pulled per eth_getLogs (default 10000).
    #[serde(default = "default_max_logs_per_request")]
    pub histocrawler_max_logs_per_request: usize,
    /// Sleep after each successful batch (default 0ms).
    #[serde(default = "default_inter_batch_delay_ms")]
    pub histocrawler_inter_batch_delay_ms: u64,
    /// Sleep between eth_getBlockByNumber timestamp lookups (default 0ms).
    #[serde(default = "default_timestamp_delay_ms")]
    pub histocrawler_timestamp_delay_ms: u64,
    /// How long to wait for new blocks once caught up to the chain head
    /// (default 1500ms). Governs eth_blockNumber polling rate at the tail.
    #[serde(default = "default_tail_poll_delay_ms")]
    pub histocrawler_tail_poll_delay_ms: u64,
}

fn default_max_batch_size() -> i64 {
    2000
}
fn default_max_logs_per_request() -> usize {
    10_000
}
fn default_inter_batch_delay_ms() -> u64 {
    0
}
fn default_timestamp_delay_ms() -> u64 {
    0
}
fn default_tail_poll_delay_ms() -> u64 {
    1500
}

#[tokio::main]
async fn main() -> Result<(), HistoCrawlerError> {
    // Init logger once — HistoCrawler::new used to call env_logger::init on every
    // restart, which panics after the first outer-loop iteration.
    env_logger::init();
    dotenvy::dotenv().ok();

    // Outer loop so a transient RPC outage cannot crash-loop via Docker restart.
    // Progress is always resumed from histocrawler.app_config.last_processed_block.
    let mut backoff = Duration::from_secs(5);
    let max_backoff = Duration::from_secs(120);
    loop {
        match HistoCrawler::new().await {
            Ok(mut app) => match app.start_indexing().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    error!("Indexing stopped with error: {e}. Retrying in {:?}...", backoff);
                }
            },
            Err(e) => {
                error!("Failed to start histocrawler: {e}. Retrying in {:?}...", backoff);
            }
        }
        sleep(backoff).await;
        backoff = std::cmp::min(backoff * 2, max_backoff);
        info!("Restarting histocrawler after backoff");
    }
}

