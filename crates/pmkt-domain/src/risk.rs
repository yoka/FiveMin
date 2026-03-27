use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Live risk state of the bot.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskState {
    pub kill_switch_triggered: bool,
    pub session_loss_limit_breached: bool,
    pub daily_loss_limit_breached: bool,
    pub max_positions_reached: bool,
    pub max_orders_reached: bool,
    pub stale_market_data: bool,
    pub stale_order_book: bool,
    pub connectivity_healthy: bool,
    pub artifact_valid: bool,
    pub reconciliation_complete: bool,
    pub risk_flags: Vec<String>,
}

impl RiskState {
    pub fn new() -> Self {
        RiskState {
            connectivity_healthy: true,
            artifact_valid: false,
            reconciliation_complete: false,
            ..Default::default()
        }
    }

    pub fn is_safe_for_live(&self) -> bool {
        !self.kill_switch_triggered
            && !self.session_loss_limit_breached
            && !self.daily_loss_limit_breached
            && !self.max_positions_reached
            && !self.stale_market_data
            && !self.stale_order_book
            && self.connectivity_healthy
            && self.artifact_valid
            && self.reconciliation_complete
    }

    pub fn trigger_kill_switch(&mut self) {
        self.kill_switch_triggered = true;
        self.risk_flags.push("KILL SWITCH TRIGGERED".to_string());
    }
}

/// Hard risk limits configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    pub max_session_loss: f64,
    pub max_daily_loss: f64,
    pub max_notional_per_trade: f64,
    pub max_open_orders: usize,
    pub max_active_positions: usize,
    pub stale_data_cutoff_secs: u64,
    pub stale_order_book_cutoff_secs: u64,
    pub no_entry_within_secs: u64,
}

impl Default for RiskLimits {
    fn default() -> Self {
        RiskLimits {
            max_session_loss: 100.0,
            max_daily_loss: 200.0,
            max_notional_per_trade: 50.0,
            max_open_orders: 3,
            max_active_positions: 1,
            stale_data_cutoff_secs: 30,
            stale_order_book_cutoff_secs: 30,
            no_entry_within_secs: 30,
        }
    }
}

/// Risk-related error types.
#[derive(Debug, Error)]
pub enum RiskError {
    #[error("kill switch is active")]
    KillSwitchActive,
    #[error("session loss limit breached: {0:.2}")]
    SessionLossLimit(f64),
    #[error("daily loss limit breached: {0:.2}")]
    DailyLossLimit(f64),
    #[error("notional {0:.2} exceeds per-trade limit {1:.2}")]
    NotionalLimit(f64, f64),
    #[error("stale market data: {0}s old")]
    StaleMarketData(u64),
    #[error("stale order book: {0}s old")]
    StaleOrderBook(u64),
    #[error("too close to market close: {0}s remaining")]
    TooCloseToClose(u64),
    #[error("reconciliation not complete")]
    ReconciliationIncomplete,
    #[error("artifact invalid")]
    ArtifactInvalid,
    #[error("mode not armed for live trading")]
    NotArmed,
}
