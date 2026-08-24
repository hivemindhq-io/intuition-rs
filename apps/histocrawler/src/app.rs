use crate::{Env, error::HistoCrawlerError};
use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::{Block, Filter, Log},
};
use log::{debug, info, warn};
use models::{histocrawler::AppConfig, raw_logs::RawLog};
use shared_utils::postgres::connect_to_db;
use sqlx::PgPool;
use std::{
    collections::HashMap,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use url::Url;

/// Added to `inter_batch_delay` for each unresolved throttling incident, so the
/// crawl settles at a rate the provider tolerates rather than repeatedly probing
/// the ceiling.
const RATE_LIMIT_PACE_STEP: Duration = Duration::from_secs(2);
const RATE_LIMIT_PACE_MAX: Duration = Duration::from_secs(30);
/// One-off pause after a throttling incident. Bandwidth quotas reset on a window
/// measured in minutes, so a short sleep just spends another request on a door
/// that is still closed.
///
/// The ceiling stays well under the `Retry-After` the provider advertises (900s
/// on Intuition testnet). That header is a worst case rather than a countdown —
/// it returns the same 900 on every probe — and in practice crawling resumed
/// after 60-240s cool-offs. Sleeping the full window idles through quota we
/// already have, and a refused probe costs ~85 bytes, so looking early is close
/// to free.
const RATE_LIMIT_COOLOFF_BASE: Duration = Duration::from_secs(60);
const RATE_LIMIT_COOLOFF_MAX: Duration = Duration::from_secs(240);
/// Quiet time required to walk one step of pacing back off.
///
/// This was a count of clean batches, which a throttled crawl cannot reach: the
/// count reset on every incident and incidents arrived long before the threshold,
/// so pacing only ratcheted upward. Elapsed time is the honest measure of "the
/// provider has tolerated us for a while", since it does not depend on batch
/// size — which throttling itself halves.
const RATE_LIMIT_DECAY_INTERVAL: Duration = Duration::from_secs(120);
/// How long a chain-head reading stays usable while crawling near the tip.
const HEAD_CACHE_TTL: Duration = Duration::from_secs(60);

/// This is the main struct for the HistoCrawler application
pub struct HistoCrawler {
    pub contract_address: Address,
    pub pg_pool: PgPool,
    pub provider: Box<dyn Provider>,
    pub backoff_delay: Duration,
    pub app_config: AppConfig,
    // Adaptive batching fields
    pub current_batch_size: i64,
    pub avg_logs_per_block: f64,
    pub max_logs_per_request: usize,
    pub min_batch_size: i64,
    pub max_batch_size: i64,
    pub inter_batch_delay: Duration,
    pub timestamp_delay: Duration,
    // Self-throttling state
    /// Extra per-batch delay earned by tripping the provider's limits.
    pub rate_limit_delay: Duration,
    /// Unresolved throttling incidents; drives both pacing and cool-off length.
    rate_limit_hits: u32,
    /// When pacing last moved, in either direction. Drives time-based recovery.
    pace_changed_at: Option<Instant>,
    /// Last chain head we read, and when. The head is only re-read when the crawl
    /// is close enough for its exact value to matter.
    cached_head: Option<(i64, Instant)>,
}

impl HistoCrawler {
    pub async fn new() -> Result<Self, HistoCrawlerError> {
        let env = Self::init().await?;
        let pg_pool = connect_to_db(&env.histocrawler_database_url).await?;
        let backoff_delay = Duration::from_millis(env.histocrawler_tail_poll_delay_ms);
        let app_config = AppConfig::find_by_indexer_schema(&env.indexer_schema, &pg_pool).await?;
        if let Some(app_config) = app_config {
            let contract_address = Address::from_str(&app_config.contract_address.to_lowercase())?;
            let provider = Self::get_provider(app_config.rpc_url.clone()).await?;
            let max_batch_size = env.histocrawler_max_batch_size.max(10);
            let max_logs_per_request = env.histocrawler_max_logs_per_request.max(50);
            info!(
                "Throttle config: max_batch_size={}, max_logs_per_request={}, inter_batch_delay_ms={}, timestamp_delay_ms={}, tail_poll_delay_ms={}",
                max_batch_size,
                max_logs_per_request,
                env.histocrawler_inter_batch_delay_ms,
                env.histocrawler_timestamp_delay_ms,
                env.histocrawler_tail_poll_delay_ms
            );
            Ok(Self {
                contract_address,
                pg_pool,
                provider: Box::new(provider),
                backoff_delay,
                app_config,
                // Initialize adaptive batching with conservative defaults for high-activity contracts
                current_batch_size: 50.min(max_batch_size), // Start small for safety
                avg_logs_per_block: 100.0,  // Conservative estimate
                max_logs_per_request,
                min_batch_size: 10,
                max_batch_size,
                inter_batch_delay: Duration::from_millis(env.histocrawler_inter_batch_delay_ms),
                timestamp_delay: Duration::from_millis(env.histocrawler_timestamp_delay_ms),
                rate_limit_delay: Duration::ZERO,
                rate_limit_hits: 0,
                pace_changed_at: None,
                cached_head: None,
            })
        } else {
            Err(HistoCrawlerError::AppConfigNotFound)
        }
    }

    /// Create a filter for the given start and end block
    pub async fn create_filter(
        &self,
        start_block: i64,
        end_block: i64,
    ) -> Result<Filter, HistoCrawlerError> {
        // Validate block range
        if start_block > end_block {
            return Err(HistoCrawlerError::InvalidBlockRange {
                start: start_block,
                end: end_block,
            });
        }

        let filter = Filter::new()
            .address(self.contract_address)
            .from_block(start_block as u64)
            .to_block(end_block as u64);
        Ok(filter)
    }

    /// Decode the raw log and insert it into the database (with timestamp cache)
    pub async fn decode_raw_log_and_insert_with_cache(
        &self,
        log: Log,
        block_timestamps: &HashMap<u64, u64>,
    ) -> Result<(), HistoCrawlerError> {
        let block_number = log
            .block_number
            .ok_or(HistoCrawlerError::BlockNumberNotFound)?;

        // Get timestamp from cache
        let block_timestamp = block_timestamps
            .get(&block_number)
            .ok_or(HistoCrawlerError::BlockNotFound(block_number))?;

        let mut raw_log = RawLog::from(log);
        raw_log
            .update_block_timestamp(*block_timestamp)
            .insert(&self.pg_pool, &self.app_config.indexer_schema)
            .await?;

        debug!("Inserted log: {:#?}", raw_log);
        Ok(())
    }

    /// Decode the raw log and insert it into the database
    /// Note: This is the legacy method that fetches timestamps individually.
    /// Use decode_raw_log_and_insert_with_cache for better performance.
    #[allow(dead_code)]
    pub async fn decode_raw_log_and_insert(&self, log: Log) -> Result<(), HistoCrawlerError> {
        let block_number = log
            .block_number
            .ok_or(HistoCrawlerError::BlockNumberNotFound)?;

        // Fetch the block timestamp from the provider
        let block = self
            .fetch_block_timestamp(block_number as i64)
            .await?
            .ok_or(HistoCrawlerError::BlockNotFound(block_number))?;

        let block_timestamp = block.header.timestamp;
        let mut raw_log = RawLog::from(log);
        raw_log
            .update_block_timestamp(block_timestamp)
            .insert(&self.pg_pool, &self.app_config.indexer_schema)
            .await?;

        debug!("Inserted log: {:#?}", raw_log);
        Ok(())
    }

    /// This method is used to fetch the timestamp of a block from the provider
    pub async fn fetch_block_timestamp(
        &self,
        block_number: i64,
    ) -> Result<Option<Block>, HistoCrawlerError> {
        let block = self
            .provider
            .get_block_by_number(BlockNumberOrTag::Number(block_number as u64))
            .await?;
        Ok(block)
    }

    /// Batch fetch block timestamps for all unique blocks in the logs.
    ///
    /// Most nodes (Reth, Erigon, Geth >=1.13) already return `blockTimestamp` on
    /// each log, so those are harvested for free. Only blocks whose timestamp is
    /// absent fall back to an `eth_getBlockByNumber` round trip — on a busy
    /// contract that fallback is one request per block, which is what exhausts
    /// bandwidth quotas on public RPCs.
    pub async fn fetch_block_timestamps_batch(
        &self,
        logs: &[Log],
    ) -> Result<HashMap<u64, u64>, HistoCrawlerError> {
        use std::collections::HashSet;

        let mut block_timestamps = HashMap::new();
        let mut missing: HashSet<u64> = HashSet::new();

        for log in logs {
            let Some(block_number) = log.block_number else {
                continue;
            };
            // A zero timestamp means the node omitted the field (alloy defaults it).
            match log.block_timestamp {
                Some(timestamp) if timestamp > 0 => {
                    block_timestamps.insert(block_number, timestamp);
                }
                _ => {
                    missing.insert(block_number);
                }
            }
        }

        // A block can appear on both paths if only some of its logs carry the field.
        missing.retain(|block_number| !block_timestamps.contains_key(block_number));

        info!(
            "Timestamps for {} logs: {} from logs, {} need a block fetch",
            logs.len(),
            block_timestamps.len(),
            missing.len()
        );

        // Fetch the remainder (optionally paced for public RPCs)
        for block_number in missing {
            match self.fetch_block_timestamp(block_number as i64).await? {
                Some(block) => {
                    block_timestamps.insert(block_number, block.header.timestamp);
                }
                None => {
                    warn!("Block {} not found, skipping", block_number);
                }
            }
            if !self.timestamp_delay.is_zero() {
                sleep(self.timestamp_delay).await;
            }
        }

        Ok(block_timestamps)
    }

    /// Get the last block number from the provider (with retry — startup used to
    /// fatal-exit here on a single flaky RPC, then Docker restart-looped).
    pub async fn get_last_block(&self) -> Result<i64, HistoCrawlerError> {
        let mut delay = Duration::from_secs(2);
        let max_delay = Duration::from_secs(60);
        let max_attempts = 8;
        let mut attempts = 0;

        loop {
            match self.provider.get_block_number().await {
                Ok(block_number) => return Ok(block_number as i64),
                Err(e) => {
                    let error_msg = e.to_string();
                    attempts += 1;
                    if attempts > max_attempts {
                        return Err(HistoCrawlerError::TransientError(error_msg));
                    }
                    // Hammering through a quota window only deepens the hole.
                    let wait = if Self::is_throttled(&error_msg) {
                        std::cmp::max(delay, RATE_LIMIT_COOLOFF_BASE)
                    } else {
                        delay
                    };
                    warn!(
                        "eth_blockNumber failed, attempt {}/{}: {}. Retrying in {:?}...",
                        attempts, max_attempts, e, wait
                    );
                    sleep(wait).await;
                    delay = std::cmp::min(delay * 2, max_delay);
                }
            }
        }
    }

    /// Read the head and refresh the cache.
    async fn refresh_last_block(&mut self) -> Result<i64, HistoCrawlerError> {
        let head = self.get_last_block().await?;
        self.cached_head = Some((head, Instant::now()));
        Ok(head)
    }

    /// Chain head, cached.
    ///
    /// A backfill spends most of its life millions of blocks behind the tip, where
    /// re-reading the head every batch doubles the request count and changes no
    /// decision. A stale head is safe while far behind because it only ever moves
    /// forward, so acting on it just means crawling slightly less than available.
    async fn cached_last_block(&mut self, next_block: i64) -> Result<i64, HistoCrawlerError> {
        if let Some((head, read_at)) = self.cached_head {
            let far_behind = next_block.saturating_add(self.max_batch_size * 4) < head;
            if far_behind || read_at.elapsed() < HEAD_CACHE_TTL {
                return Ok(head);
            }
        }
        self.refresh_last_block().await
    }

    /// Get the provider
    pub async fn get_provider(rpc_url: String) -> Result<impl Provider, HistoCrawlerError> {
        let rpc_url = Url::parse(&rpc_url)?;
        Ok(ProviderBuilder::new().connect_http(rpc_url))
    }

    /// Initialize the environment variables
    pub async fn init() -> Result<Env, HistoCrawlerError> {
        // Logger + dotenv are initialized once in main (safe across restart loop).
        // Parse the environment
        let env = envy::from_env::<Env>()?;
        Ok(env)
    }

    /// Get the block number ceiling using adaptive batch sizing
    /// This calculates the batch size based on average logs per block to avoid RPC response size limits
    pub async fn get_block_number_ceiling(
        &self,
        start_block: i64,
        last_block: i64,
    ) -> Result<i64, HistoCrawlerError> {
        // If the end block is provided, we will use it as the ceiling
        if self.app_config.end_block.is_some() {
            let config_end = self.app_config.end_block.unwrap();
            return Ok(std::cmp::min(config_end, last_block));
        }

        // Calculate the target end block using current_batch_size (adaptive)
        let target_end_block = start_block + self.current_batch_size;

        // If the target end block is greater than the last block available, use the last block available
        if target_end_block > last_block {
            Ok(last_block)
        } else {
            Ok(target_end_block)
        }
    }

    /// Calculate and update the adaptive batch size based on recent metrics
    pub fn update_batch_size(&mut self, logs_in_batch: usize, blocks_in_batch: i64) {
        // Calculate logs per block for this batch
        let logs_per_block = if blocks_in_batch > 0 {
            logs_in_batch as f64 / blocks_in_batch as f64
        } else {
            self.avg_logs_per_block
        };

        // Update rolling average (70% old, 30% new for smooth adaptation)
        self.avg_logs_per_block = (self.avg_logs_per_block * 0.7) + (logs_per_block * 0.3);

        // Calculate new batch size to target max_logs_per_request
        let calculated_batch_size = if self.avg_logs_per_block > 0.0 {
            (self.max_logs_per_request as f64 / self.avg_logs_per_block) as i64
        } else {
            self.max_batch_size
        };

        // Clamp to min/max bounds
        self.current_batch_size = calculated_batch_size
            .max(self.min_batch_size)
            .min(self.max_batch_size);

        info!(
            "Batch size updated: {} blocks (avg logs/block: {:.1}, logs in batch: {})",
            self.current_batch_size, self.avg_logs_per_block, logs_in_batch
        );
    }

    /// Reduce batch size when encountering RPC errors (emergency reduction)
    pub fn reduce_batch_size(&mut self, reduction_factor: f64) {
        let new_size = (self.current_batch_size as f64 * reduction_factor) as i64;
        self.current_batch_size = new_size.max(self.min_batch_size);
        warn!(
            "Emergency batch size reduction: {} blocks (factor: {})",
            self.current_batch_size, reduction_factor
        );
    }

    /// Update the start and end block for the next iteration
    pub async fn update_start_end_blocks(
        &mut self,
        start_block: &mut i64,
        end_block: &mut i64,
    ) -> Result<(), HistoCrawlerError> {
        // Update the start and end block for the next iteration
        *start_block = *end_block + 1; // Add 1 to avoid skipping blocks

        let mut current_last_block = self.cached_last_block(*start_block).await?;

        // If start_block is now beyond the last available block, we need to wait
        if *start_block > current_last_block {
            info!(
                "Start block {} is beyond last available block {}, backing off for {:?}",
                *start_block, current_last_block, self.backoff_delay
            );
            sleep(self.backoff_delay).await;

            // At the tip a cached head answers nothing; read through for new blocks.
            current_last_block = self.refresh_last_block().await?;
            if *start_block > current_last_block {
                // No new blocks, keep current values and continue waiting
                return Ok(());
            }
        }

        *end_block = self
            .get_block_number_ceiling(*start_block, current_last_block)
            .await?;

        Ok(())
    }

    fn is_response_too_large(error_msg: &str) -> bool {
        error_msg.contains("Response is too big")
            || error_msg.contains("Exceeded max limit")
            || error_msg.contains("response size")
            || error_msg.contains("too large")
    }

    fn is_transient_error(error_msg: &str) -> bool {
        error_msg.contains("timed out")
            || error_msg.contains("timeout")
            || error_msg.contains("IncompleteBody")
            || error_msg.contains("incomplete body")
            || error_msg.contains("decoding response body")
            || error_msg.contains("error sending request")
            || error_msg.contains("External service error")
            || error_msg.contains("connection reset")
            || error_msg.contains("connection closed")
            || error_msg.contains("broken pipe")
            || error_msg.contains("EOF")
            || error_msg.contains("status: 500")
            || error_msg.contains("status: 502")
            || error_msg.contains("status: 503")
            || error_msg.contains("HTTP error 500")
            || error_msg.contains("ECONNRESET")
    }

    fn is_rate_limited(error_msg: &str) -> bool {
        error_msg.contains("429")
            || error_msg.contains("rate limit")
            || error_msg.contains("Bandwidth limit")
            || error_msg.contains("Too Many Requests")
    }

    /// The shared rpc-proxy collapses every upstream failure into a generic 500
    /// ("External service error: Request to ... failed after 3 retries"), so a
    /// bandwidth ban is indistinguishable from a blip by status alone. Treat the
    /// proxy's own exhaustion signature as throttling: a provider that has failed
    /// the proxy's full retry budget will not be rescued by us retrying faster.
    fn is_upstream_exhausted(error_msg: &str) -> bool {
        error_msg.contains("External service error")
            || (error_msg.contains("failed after") && error_msg.contains("retries"))
    }

    fn is_throttled(error_msg: &str) -> bool {
        Self::is_rate_limited(error_msg) || Self::is_upstream_exhausted(error_msg)
    }

    /// Record a throttling incident and return how long to stand down.
    ///
    /// Two effects: a one-off cool-off long enough to outlast a quota window, and
    /// a lasting per-batch delay so the resumed crawl is slower than the one that
    /// got us throttled. Without the second part the crawler just walks straight
    /// back into the limit.
    fn penalize_rate_limit(&mut self) -> Duration {
        self.rate_limit_hits = self.rate_limit_hits.saturating_add(1);
        self.pace_changed_at = Some(Instant::now());
        self.rate_limit_delay = std::cmp::min(
            RATE_LIMIT_PACE_STEP * self.rate_limit_hits,
            RATE_LIMIT_PACE_MAX,
        );

        let exponent = self.rate_limit_hits.min(5) - 1;
        std::cmp::min(
            RATE_LIMIT_COOLOFF_BASE * 2u32.saturating_pow(exponent),
            RATE_LIMIT_COOLOFF_MAX,
        )
    }

    /// Walk pacing back down after the provider has tolerated us for a while.
    fn maybe_relax_pacing(&mut self) {
        if self.rate_limit_hits == 0 {
            return;
        }
        let Some(changed_at) = self.pace_changed_at else {
            self.pace_changed_at = Some(Instant::now());
            return;
        };
        if changed_at.elapsed() < RATE_LIMIT_DECAY_INTERVAL {
            return;
        }
        self.pace_changed_at = Some(Instant::now());
        self.rate_limit_hits -= 1;
        self.rate_limit_delay = std::cmp::min(
            RATE_LIMIT_PACE_STEP * self.rate_limit_hits,
            RATE_LIMIT_PACE_MAX,
        );
        info!(
            "No throttling for {:?}; relaxing added pacing to {:?}",
            RATE_LIMIT_DECAY_INTERVAL, self.rate_limit_delay
        );
    }

    async fn get_logs_with_retry(&self, filter: &Filter) -> Result<Vec<Log>, HistoCrawlerError> {
        let mut delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(30);
        let mut attempts = 0;
        let max_attempts = 5;

        loop {
            match self.provider.get_logs(filter).await {
                Ok(logs) => return Ok(logs),
                Err(e) => {
                    let error_msg = e.to_string();

                    if Self::is_response_too_large(&error_msg) {
                        warn!(
                            "RPC response size limit exceeded: {}. Batch size needs reduction.",
                            error_msg
                        );
                        return Err(HistoCrawlerError::ResponseSizeLimitExceeded);
                    }

                    // An explicit throttling response is not worth retrying at all;
                    // the caller owns the cool-off.
                    if Self::is_rate_limited(&error_msg) {
                        return Err(HistoCrawlerError::RateLimited(error_msg));
                    }

                    if Self::is_transient_error(&error_msg) {
                        attempts += 1;
                        if attempts > max_attempts {
                            warn!(
                                "Transient error persisted after {} attempts: {}",
                                max_attempts, error_msg
                            );
                            // Surviving the whole retry budget means the upstream is
                            // down or refusing us, not momentarily busy.
                            if Self::is_upstream_exhausted(&error_msg) {
                                return Err(HistoCrawlerError::RateLimited(error_msg));
                            }
                            return Err(HistoCrawlerError::TransientError(error_msg));
                        }
                        warn!(
                            "Transient RPC error, attempt {}/{}: {}. Retrying in {:?}...",
                            attempts, max_attempts, e, delay
                        );
                        sleep(delay).await;
                        delay = std::cmp::min(delay * 2, max_delay);
                        continue;
                    }

                    attempts += 1;
                    if attempts > max_attempts {
                        return Err(e.into());
                    }
                    warn!(
                        "RPC call failed, attempt {}/{}. Error: {}. Retrying in {:?}...",
                        attempts, max_attempts, e, delay
                    );
                    sleep(delay).await;
                    delay = std::cmp::min(delay * 2, max_delay);
                }
            }
        }
    }

    pub async fn start_indexing(&mut self) -> Result<(), HistoCrawlerError> {
        info!(
            "Starting indexing from block {}",
            self.app_config.start_block
        );
        let last_block = self.get_last_block().await?;

        info!("Current last available block: {}", last_block);

        let mut start_block = self.app_config.start_block;

        // Use last_processed_block from app_config to determine where to resume
        if let Some(last_processed_block) = self.app_config.last_processed_block {
            info!(
                "Found last processed block: {}, resuming from block {}",
                last_processed_block,
                last_processed_block + 1
            );
            start_block = last_processed_block + 1;
        } else {
            info!(
                "No last processed block found, starting from configured start block: {}",
                start_block
            );
        }

        let mut end_block = self
            .get_block_number_ceiling(start_block, last_block)
            .await?;

        loop {
            // Validate block range before creating filter
            if start_block > end_block {
                info!(
                    "Invalid block range detected: start_block ({}) > end_block ({}), updating...",
                    start_block, end_block
                );
                self.update_start_end_blocks(&mut start_block, &mut end_block)
                    .await?;
                continue;
            }

            let filter = self.create_filter(start_block, end_block).await?;
            info!(
                "Fetching logs for blocks {} to {} (batch size: {})",
                start_block, end_block, self.current_batch_size
            );

            let logs = match self.get_logs_with_retry(&filter).await {
                Ok(logs) => logs,
                Err(HistoCrawlerError::ResponseSizeLimitExceeded) => {
                    self.reduce_batch_size(0.5);
                    end_block = self
                        .get_block_number_ceiling(start_block, end_block)
                        .await?;
                    info!("Response too large, reduced batch to end_block: {}", end_block);
                    continue;
                }
                Err(HistoCrawlerError::RateLimited(msg)) => {
                    let cooloff = self.penalize_rate_limit();
                    warn!(
                        "Throttled by RPC provider: {}. Cooling off {:?}, then crawling with \
                         {:?} extra pacing per batch.",
                        msg, cooloff, self.rate_limit_delay
                    );
                    self.reduce_batch_size(0.5);
                    end_block = self
                        .get_block_number_ceiling(start_block, end_block)
                        .await?;
                    sleep(cooloff).await;
                    continue;
                }
                Err(HistoCrawlerError::TransientError(msg)) => {
                    warn!("Transient error: {}. Reducing batch and backing off...", msg);
                    self.reduce_batch_size(0.7);
                    end_block = self
                        .get_block_number_ceiling(start_block, end_block)
                        .await?;
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if Self::is_throttled(&error_msg) {
                        let cooloff = self.penalize_rate_limit();
                        warn!("Throttling escaped retry loop: {}. Cooling off {:?}...", e, cooloff);
                        self.reduce_batch_size(0.5);
                        end_block = self
                            .get_block_number_ceiling(start_block, end_block)
                            .await?;
                        sleep(cooloff).await;
                        continue;
                    }
                    if Self::is_transient_error(&error_msg) {
                        warn!("Recoverable error escaped retry loop: {}. Backing off...", e);
                        self.reduce_batch_size(0.5);
                        end_block = self
                            .get_block_number_ceiling(start_block, end_block)
                            .await?;
                        sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                    return Err(e);
                }
            };

            info!("Retrieved {} logs for batch", logs.len());

            // Batch fetch block timestamps for all logs (massive RPC call reduction!)
            let block_timestamps = match self.fetch_block_timestamps_batch(&logs).await {
                Ok(timestamps) => timestamps,
                Err(e) if Self::is_throttled(&e.to_string()) => {
                    let cooloff = self.penalize_rate_limit();
                    warn!(
                        "Throttled while backfilling block timestamps: {}. Cooling off {:?}...",
                        e, cooloff
                    );
                    sleep(cooloff).await;
                    continue;
                }
                Err(e) => return Err(e),
            };

            // Process logs in the current batch with cached timestamps
            let mut encountered_error = false;
            let mut processed_log_count = 0;
            for log in &logs {
                if let Err(e) = self
                    .decode_raw_log_and_insert_with_cache(log.clone(), &block_timestamps)
                    .await
                {
                    info!(
                        "Error processing log in block {}: {}. Reducing batch size.",
                        log.block_number.unwrap_or(start_block as u64),
                        e
                    );
                    if end_block - start_block > 100 {
                        end_block = start_block + (end_block - start_block) / 2;
                        self.reduce_batch_size(0.5); // Emergency 50% reduction
                        info!("New end_block: {}", end_block);
                        encountered_error = true;
                        break; // break out to refetch logs for the smaller range
                    } else {
                        return Err(e);
                    }
                }
                processed_log_count += 1;
            }

            if encountered_error {
                // Instead of updating start/end blocks immediately, retry the current range.
                continue;
            }

            info!(
                "Successfully scanned blocks {} to {}, processed {} logs",
                start_block, end_block, processed_log_count
            );

            // Update adaptive batch size based on this batch's metrics
            let blocks_in_batch = end_block - start_block + 1;
            self.update_batch_size(logs.len(), blocks_in_batch);

            // Update the last processed block in the database after successful processing
            self.app_config
                .update_last_processed_block(end_block, &self.pg_pool)
                .await?;

            self.maybe_relax_pacing();
            let pause = self.inter_batch_delay + self.rate_limit_delay;
            if !pause.is_zero() {
                sleep(pause).await;
            }

            self.update_start_end_blocks(&mut start_block, &mut end_block)
                .await?;
        }
    }
}
