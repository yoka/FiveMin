use chrono::{DateTime, Utc};
use pmkt_domain::{BucketKey, BucketOosProfile, StrategyClassification};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strategy configuration thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    pub min_edge: f64,
    pub cost_buffer: f64,
    pub min_prob: f64,
    pub min_samples: usize,
    pub max_time_left_sec: u64,
    pub vol_low_threshold: f64,
    pub vol_high_threshold: f64,
    /// Banned price zones (e.g. underdog, heavy_fav)
    pub banned_price_zones: Vec<String>,
    /// Allow fallback to "all" regime bucket if specific not whitelisted
    pub allow_all_regime_fallback: bool,
    /// Minimum size in USDC per trade
    pub min_trade_size: f64,
    /// Maximum size in USDC per trade
    pub max_trade_size: f64,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        StrategyConfig {
            min_edge: 0.03,
            cost_buffer: 0.02,
            min_prob: 0.55,
            min_samples: 20,
            max_time_left_sec: 240,
            vol_low_threshold: 0.02,
            vol_high_threshold: 0.05,
            banned_price_zones: vec![],
            allow_all_regime_fallback: false,
            min_trade_size: 5.0,
            max_trade_size: 50.0,
        }
    }
}

/// Whitelist classifier configuration thresholds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistConfig {
    pub min_oos_trades: usize,
    pub min_oos_windows: usize,
    pub min_pct_profitable_windows: f64,
    pub max_worst_window_loss: f64,
    pub min_oos_pnl: f64,
    /// Blacklist: bad OOS P&L threshold (negative)
    pub blacklist_pnl_threshold: f64,
    pub blacklist_min_support: usize,
    /// Blacklist: bad win rate threshold
    pub blacklist_win_rate_threshold: f64,
}

impl Default for WhitelistConfig {
    fn default() -> Self {
        WhitelistConfig {
            min_oos_trades: 30,
            min_oos_windows: 3,
            min_pct_profitable_windows: 0.60,
            max_worst_window_loss: -20.0,
            min_oos_pnl: 0.0,
            blacklist_pnl_threshold: -10.0,
            blacklist_min_support: 20,
            blacklist_win_rate_threshold: 0.35,
        }
    }
}

/// A versioned strategy artifact (lookup table + whitelist/blacklist).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyArtifact {
    pub schema_version: u32,
    pub artifact_id: String,
    pub built_at: DateTime<Utc>,
    pub build_params: StrategyConfig,
    pub whitelist_config: WhitelistConfig,
    pub vol_low_threshold: f64,
    pub vol_high_threshold: f64,
    /// BucketKey string → BucketOosProfile
    pub profiles: HashMap<String, BucketOosProfile>,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
    pub source_dataset_hash: Option<String>,
    pub git_commit: Option<String>,
}

impl StrategyArtifact {
    pub fn new(
        build_params: StrategyConfig,
        whitelist_config: WhitelistConfig,
        profiles: HashMap<String, BucketOosProfile>,
        whitelist: Vec<String>,
        blacklist: Vec<String>,
    ) -> Self {
        let vol_low = build_params.vol_low_threshold;
        let vol_high = build_params.vol_high_threshold;
        StrategyArtifact {
            schema_version: 1,
            artifact_id: uuid::Uuid::new_v4().to_string(),
            built_at: Utc::now(),
            build_params,
            whitelist_config,
            vol_low_threshold: vol_low,
            vol_high_threshold: vol_high,
            profiles,
            whitelist,
            blacklist,
            source_dataset_hash: None,
            git_commit: None,
        }
    }

    pub fn is_whitelisted(&self, key: &BucketKey) -> bool {
        let k = key.to_key_string();
        self.whitelist.contains(&k)
    }

    pub fn is_blacklisted(&self, key: &BucketKey) -> bool {
        let k = key.to_key_string();
        self.blacklist.contains(&k)
    }

    pub fn get_profile(&self, key: &BucketKey) -> Option<&BucketOosProfile> {
        self.profiles.get(&key.to_key_string())
    }

    pub fn classify_bucket(&self, key: &BucketKey) -> StrategyClassification {
        if self.is_blacklisted(key) {
            StrategyClassification::Blacklisted
        } else if self.is_whitelisted(key) {
            StrategyClassification::Whitelisted
        } else if self.profiles.contains_key(&key.to_key_string()) {
            StrategyClassification::Neutral
        } else {
            StrategyClassification::Unknown
        }
    }
}

/// Build an empty demo artifact for testing.
pub fn demo_artifact() -> StrategyArtifact {
    StrategyArtifact::new(
        StrategyConfig::default(),
        WhitelistConfig::default(),
        HashMap::new(),
        vec![],
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pmkt_domain::{price_bin_from_price, vol_regime_from_vol, TimeBucket};

    #[test]
    fn test_artifact_classification_empty() {
        let artifact = demo_artifact();
        let key = pmkt_domain::BucketKey::new(
            TimeBucket::Late,
            price_bin_from_price(0.82),
            vol_regime_from_vol(0.03, 0.02, 0.05),
        );
        assert_eq!(artifact.classify_bucket(&key), StrategyClassification::Unknown);
    }

    #[test]
    fn test_artifact_serialization_roundtrip() {
        let artifact = demo_artifact();
        let json = serde_json::to_string(&artifact).unwrap();
        let restored: StrategyArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(artifact.schema_version, restored.schema_version);
        assert_eq!(artifact.whitelist.len(), restored.whitelist.len());
    }
}
