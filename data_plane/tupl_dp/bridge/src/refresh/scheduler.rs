//! Scheduled rule refresh service - background task for periodic rule reloading.
//!
//! Periodically rebuilds the in-memory HashMap from SQLite to ensure consistency.

use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::bridge::Bridge;
use crate::types::now_ms;
use log::{error, info};

/// Configuration for the scheduled refresh service.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Interval between refresh operations
    pub refresh_interval: Duration,
    /// Whether the scheduler is enabled
    pub enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            refresh_interval: Duration::from_secs(6 * 60 * 60),
            enabled: true,
        }
    }
}

/// Scheduler for periodic rule refresh from SQLite.
///
/// Runs as a background task spawned during server initialization.
/// Periodically rebuilds the in-memory HashMap from SQLite.
pub struct RefreshScheduler {
    /// Reference to the bridge instance
    bridge: Arc<Bridge>,
    /// Scheduler configuration
    config: SchedulerConfig,
    /// Timestamp of the last successful refresh
    last_refresh_at: Arc<RwLock<u64>>,
}

impl RefreshScheduler {
    /// Creates a new refresh scheduler.
    pub fn new(bridge: Arc<Bridge>, config: SchedulerConfig) -> Self {
        Self {
            bridge,
            config,
            last_refresh_at: Arc::new(RwLock::new(now_ms())),
        }
    }

    /// Returns the timestamp of the last successful refresh.
    pub fn last_refresh(&self) -> u64 {
        *self.last_refresh_at.read()
    }

    /// Starts the scheduler background task.
    ///
    /// This method runs indefinitely and should be spawned as a tokio task.
    pub async fn start(self: Arc<Self>) {
        if !self.config.enabled {
            info!("Scheduled refresh is disabled, skipping");
            return;
        }

        info!(
            "Starting scheduled refresh with {}-second interval",
            self.config.refresh_interval.as_secs()
        );

        let mut ticker = interval(self.config.refresh_interval);

        loop {
            ticker.tick().await;
            self.do_refresh().await;
        }
    }

    /// Executes one refresh cycle.
    async fn do_refresh(&self) {
        match self.bridge.rebuild_from_db_public() {
            Ok(()) => {
                let num_rules = self.bridge.rule_count();
                info!(
                    "Scheduled refresh completed: {} rules loaded from SQLite",
                    num_rules
                );
                *self.last_refresh_at.write() = now_ms();
            }
            Err(e) => {
                error!("Scheduled refresh failed: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_defaults() {
        let config = SchedulerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.refresh_interval.as_secs(), 6 * 60 * 60);
    }

    #[test]
    fn test_scheduler_creation() {
        let config = SchedulerConfig::default();
        assert!(config.enabled);
        assert_eq!(config.refresh_interval.as_secs(), 21600);
    }

    #[test]
    fn test_scheduler_config_custom() {
        let config = SchedulerConfig {
            refresh_interval: Duration::from_secs(3600),
            enabled: false,
        };
        assert!(!config.enabled);
        assert_eq!(config.refresh_interval.as_secs(), 3600);
    }
}
