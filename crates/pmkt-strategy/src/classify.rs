use pmkt_domain::{
    BucketKey, BucketOosProfile, BucketStats, StrategyClassification,
};
use crate::artifact::WhitelistConfig;
use std::collections::HashMap;

/// Classify a single bucket given its stats and config.
pub fn classify_bucket(
    _key: &BucketKey,
    stats: &BucketStats,
    config: &WhitelistConfig,
) -> (StrategyClassification, String) {
    // Blacklist checks first (poison)
    if stats.total_samples >= config.blacklist_min_support {
        if stats.total_pnl < config.blacklist_pnl_threshold {
            return (
                StrategyClassification::Blacklisted,
                format!(
                    "bad OOS P&L {:.2} < threshold {:.2} with {} samples",
                    stats.total_pnl, config.blacklist_pnl_threshold, stats.total_samples
                ),
            );
        }
        if stats.win_rate < config.blacklist_win_rate_threshold {
            return (
                StrategyClassification::Blacklisted,
                format!(
                    "bad win rate {:.2} < threshold {:.2} with {} samples",
                    stats.win_rate, config.blacklist_win_rate_threshold, stats.total_samples
                ),
            );
        }
    }

    // Whitelist checks (must pass ALL)
    let mut reasons: Vec<String> = Vec::new();

    if stats.oos_trades < config.min_oos_trades {
        reasons.push(format!(
            "oos_trades {} < min {}",
            stats.oos_trades, config.min_oos_trades
        ));
    }
    if stats.oos_windows < config.min_oos_windows {
        reasons.push(format!(
            "oos_windows {} < min {}",
            stats.oos_windows, config.min_oos_windows
        ));
    }
    if stats.pct_profitable_windows < config.min_pct_profitable_windows {
        reasons.push(format!(
            "pct_profitable_windows {:.2} < min {:.2}",
            stats.pct_profitable_windows, config.min_pct_profitable_windows
        ));
    }
    // worst_window_loss is negative (e.g. -15.0); max_worst_window_loss is the floor
    // (e.g. -20.0). Reject if actual loss is worse (more negative) than the floor.
    if stats.worst_window_loss < config.max_worst_window_loss {
        reasons.push(format!(
            "worst_window_loss {:.2} < limit {:.2}",
            stats.worst_window_loss, config.max_worst_window_loss
        ));
    }
    if stats.total_pnl < config.min_oos_pnl {
        reasons.push(format!(
            "total_pnl {:.2} < min_oos_pnl {:.2}",
            stats.total_pnl, config.min_oos_pnl
        ));
    }

    if reasons.is_empty() {
        (
            StrategyClassification::Whitelisted,
            format!(
                "passes all checks: trades={}, win_rate={:.2}, pnl={:.2}",
                stats.oos_trades, stats.win_rate, stats.total_pnl
            ),
        )
    } else {
        (
            StrategyClassification::Neutral,
            format!("neutral: {}", reasons.join("; ")),
        )
    }
}

/// Build whitelist/blacklist from a map of bucket stats.
pub fn build_whitelist(
    stats_map: &HashMap<String, BucketStats>,
    config: &WhitelistConfig,
) -> (Vec<String>, Vec<String>, HashMap<String, BucketOosProfile>) {
    let mut whitelist = Vec::new();
    let mut blacklist = Vec::new();
    let mut profiles = HashMap::new();

    for (key_str, stats) in stats_map {
        let (classification, reason) = if let Some(bucket_key) = &stats.key {
            classify_bucket(bucket_key, stats, config)
        } else {
            (StrategyClassification::Unknown, "no bucket key".to_string())
        };

        match classification {
            StrategyClassification::Whitelisted => whitelist.push(key_str.clone()),
            StrategyClassification::Blacklisted => blacklist.push(key_str.clone()),
            _ => {}
        }

        profiles.insert(
            key_str.clone(),
            BucketOosProfile {
                key: stats.key.clone(),
                classification,
                classification_reason: reason,
                stats: stats.clone(),
                price_zone: None,
                vol_regime: None,
            },
        );
    }

    (whitelist, blacklist, profiles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pmkt_domain::{price_bin_from_price, vol_regime_from_vol, TimeBucket};

    fn make_stats(
        oos_trades: usize,
        oos_windows: usize,
        pct_profitable: f64,
        worst_loss: f64,
        total_pnl: f64,
        win_rate: f64,
        total_samples: usize,
    ) -> BucketStats {
        BucketStats {
            key: None,
            total_samples,
            win_rate,
            avg_pnl: total_pnl / total_samples.max(1) as f64,
            total_pnl,
            sharpe: 1.0,
            oos_windows,
            oos_trades,
            pct_profitable_windows: pct_profitable,
            worst_window_loss: worst_loss,
        }
    }

    #[test]
    fn test_whitelist_passes() {
        let config = WhitelistConfig::default();
        let key = BucketKey::new(
            TimeBucket::Late,
            price_bin_from_price(0.82),
            vol_regime_from_vol(0.03, 0.02, 0.05),
        );
        let stats = make_stats(50, 5, 0.70, -5.0, 15.0, 0.60, 50);
        let (class, _reason) = classify_bucket(&key, &stats, &config);
        assert_eq!(class, StrategyClassification::Whitelisted);
    }

    #[test]
    fn test_blacklist_bad_pnl() {
        let config = WhitelistConfig::default();
        let key = BucketKey::new(
            TimeBucket::Late,
            price_bin_from_price(0.82),
            vol_regime_from_vol(0.03, 0.02, 0.05),
        );
        let stats = make_stats(50, 5, 0.70, -5.0, -50.0, 0.60, 50);
        let (class, _reason) = classify_bucket(&key, &stats, &config);
        assert_eq!(class, StrategyClassification::Blacklisted);
    }

    #[test]
    fn test_neutral_insufficient_samples() {
        let config = WhitelistConfig::default();
        let key = BucketKey::new(
            TimeBucket::Late,
            price_bin_from_price(0.82),
            vol_regime_from_vol(0.03, 0.02, 0.05),
        );
        let stats = make_stats(5, 1, 0.70, -5.0, 10.0, 0.60, 5);
        let (class, _reason) = classify_bucket(&key, &stats, &config);
        assert_eq!(class, StrategyClassification::Neutral);
    }

    #[test]
    fn test_no_trade_on_blacklisted_unknown() {
        // Property: blacklisted and unknown buckets must never generate trade signals
        let configs = [
            make_stats(50, 5, 0.20, -50.0, -100.0, 0.25, 50), // blacklist
            make_stats(1, 1, 0.50, 0.0, 0.0, 0.50, 1),          // neutral
        ];
        let config = WhitelistConfig::default();
        let key = BucketKey::new(
            TimeBucket::Late,
            price_bin_from_price(0.82),
            vol_regime_from_vol(0.03, 0.02, 0.05),
        );
        for stats in &configs {
            let (class, _) = classify_bucket(&key, stats, &config);
            assert_ne!(class, StrategyClassification::Whitelisted);
        }
    }
}
