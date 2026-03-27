use crate::artifact::StrategyArtifact;
use crate::lookup_table::TradeRecord;
use crate::score::score_state;
use pmkt_domain::{LiveState, MarketId};
use chrono::Utc;

/// Result of a single backtest trade.
#[derive(Debug, Clone)]
pub struct BacktestTrade {
    pub record_index: usize,
    pub decision: String,
    pub pnl: f64,
    pub correct: bool,
}

/// Aggregate backtest results.
#[derive(Debug, Clone, Default)]
pub struct BacktestResults {
    pub total_signals: usize,
    pub trades_taken: usize,
    pub wins: usize,
    pub losses: usize,
    pub total_pnl: f64,
    pub win_rate: f64,
}

impl BacktestResults {
    pub fn compute_win_rate(&mut self) {
        self.win_rate = if self.trades_taken > 0 {
            self.wins as f64 / self.trades_taken as f64
        } else {
            0.0
        };
    }
}

/// Run whitelist backtest over historical records.
pub fn backtest_whitelist(
    records: &[TradeRecord],
    artifact: &StrategyArtifact,
) -> BacktestResults {
    let config = &artifact.build_params;
    let mut results = BacktestResults::default();

    for (_i, record) in records.iter().enumerate() {
        results.total_signals += 1;

        let state = LiveState {
            market_id: MarketId::from("backtest"),
            time_left_sec: record.time_left_sec,
            up_price: record.up_price,
            vol_30s: record.vol_30s,
            captured_at: Utc::now(),
        };

        let signal = score_state(&state, artifact, config);

        if signal.decision.is_trade() {
            results.trades_taken += 1;
            if record.outcome_correct {
                results.wins += 1;
            } else {
                results.losses += 1;
            }
            results.total_pnl += record.pnl;
        }
    }

    results.compute_win_rate();
    results
}
