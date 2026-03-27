use pmkt_domain::{BucketKey, BucketStats, TimeBucket};
use pmkt_domain::{price_bin_from_price, vol_regime_from_vol};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single historical trade record for building the lookup table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub time_left_sec: u64,
    pub up_price: f64,
    pub vol_30s: f64,
    pub outcome_correct: bool,
    pub pnl: f64,
}

/// Build bucket statistics from historical trade records.
pub fn build_lookup_table(
    records: &[TradeRecord],
    vol_low: f64,
    vol_high: f64,
) -> HashMap<String, BucketStats> {
    let mut map: HashMap<BucketKey, Vec<&TradeRecord>> = HashMap::new();

    for record in records {
        let time_bucket = TimeBucket::from_seconds(record.time_left_sec);
        let price_bin = price_bin_from_price(record.up_price);
        let vol_regime = vol_regime_from_vol(record.vol_30s, vol_low, vol_high);
        let key = BucketKey::new(time_bucket, price_bin, vol_regime);
        map.entry(key).or_default().push(record);
    }

    let mut result = HashMap::new();

    for (key, trades) in map {
        let total = trades.len();
        if total == 0 {
            continue;
        }
        let wins = trades.iter().filter(|t| t.outcome_correct).count();
        let win_rate = wins as f64 / total as f64;
        let total_pnl: f64 = trades.iter().map(|t| t.pnl).sum();
        let avg_pnl = total_pnl / total as f64;

        // Simplified Sharpe (mean / std of P&L)
        let mean = avg_pnl;
        let variance = if total > 1 {
            trades.iter().map(|t| (t.pnl - mean).powi(2)).sum::<f64>() / (total - 1) as f64
        } else {
            0.0
        };
        let sharpe = if variance > 0.0 {
            mean / variance.sqrt()
        } else {
            0.0
        };

        let stats = BucketStats {
            key: Some(key.clone()),
            total_samples: total,
            win_rate,
            avg_pnl,
            total_pnl,
            sharpe,
            // Walk-forward stats would be filled in by the WF engine
            oos_windows: 0,
            oos_trades: total,
            pct_profitable_windows: 0.0,
            worst_window_loss: 0.0,
        };

        result.insert(key.to_key_string(), stats);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_records(
        time_left: u64,
        up_price: f64,
        vol: f64,
        n_wins: usize,
        n_losses: usize,
    ) -> Vec<TradeRecord> {
        let mut records = vec![];
        for _ in 0..n_wins {
            records.push(TradeRecord {
                time_left_sec: time_left,
                up_price,
                vol_30s: vol,
                outcome_correct: true,
                pnl: 2.0,
            });
        }
        for _ in 0..n_losses {
            records.push(TradeRecord {
                time_left_sec: time_left,
                up_price,
                vol_30s: vol,
                outcome_correct: false,
                pnl: -1.0,
            });
        }
        records
    }

    #[test]
    fn test_build_lookup_basic() {
        let records = make_records(90, 0.82, 0.03, 6, 4);
        let table = build_lookup_table(&records, 0.02, 0.05);
        assert!(!table.is_empty());
        let stats = table.values().next().unwrap();
        assert_eq!(stats.total_samples, 10);
        assert!((stats.win_rate - 0.6).abs() < 1e-9);
    }
}
