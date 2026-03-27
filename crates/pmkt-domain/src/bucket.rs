use serde::{Deserialize, Serialize};
use std::fmt;

/// Volatility regime classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VolRegime {
    Low,
    Medium,
    High,
}

impl fmt::Display for VolRegime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolRegime::Low => write!(f, "low"),
            VolRegime::Medium => write!(f, "mid"),
            VolRegime::High => write!(f, "high"),
        }
    }
}

/// A discretized price bin (e.g. 0.80–0.85).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PriceBin {
    pub lo: f64,
    pub hi: f64,
}

impl Eq for PriceBin {}

impl std::hash::Hash for PriceBin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use integer representation for hashing
        ((self.lo * 100.0) as i64).hash(state);
        ((self.hi * 100.0) as i64).hash(state);
    }
}

impl fmt::Display for PriceBin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}-{:.2}", self.lo, self.hi)
    }
}

/// Price zone classification relative to 0.50.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PriceZone {
    /// Price ≥ 0.75: heavy favourite
    HeavyFav,
    /// Price 0.60–0.74: moderate favourite
    ModerateFav,
    /// Price 0.45–0.59: near toss-up
    NearTossup,
    /// Price ≤ 0.44: underdog
    Underdog,
}

impl fmt::Display for PriceZone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PriceZone::HeavyFav => write!(f, "heavy_fav"),
            PriceZone::ModerateFav => write!(f, "moderate_fav"),
            PriceZone::NearTossup => write!(f, "near_tossup"),
            PriceZone::Underdog => write!(f, "underdog"),
        }
    }
}

impl PriceZone {
    /// Classify a market price into a zone.
    pub fn from_price(price: f64) -> Self {
        if price >= 0.75 {
            PriceZone::HeavyFav
        } else if price >= 0.60 {
            PriceZone::ModerateFav
        } else if price >= 0.45 {
            PriceZone::NearTossup
        } else {
            PriceZone::Underdog
        }
    }
}

/// Time-left bucket (seconds remaining in the market).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeBucket {
    /// > 240s remaining
    Early,
    /// 120–240s
    Mid,
    /// 60–119s
    Late,
    /// < 60s
    VeryLate,
}

impl fmt::Display for TimeBucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeBucket::Early => write!(f, "early"),
            TimeBucket::Mid => write!(f, "mid"),
            TimeBucket::Late => write!(f, "late"),
            TimeBucket::VeryLate => write!(f, "very_late"),
        }
    }
}

impl TimeBucket {
    pub fn from_seconds(secs: u64) -> Self {
        if secs > 240 {
            TimeBucket::Early
        } else if secs >= 120 {
            TimeBucket::Mid
        } else if secs >= 60 {
            TimeBucket::Late
        } else {
            TimeBucket::VeryLate
        }
    }
}

/// The composite typed bucket key used internally.
/// Never use raw strings; format to string only at display/storage boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BucketKey {
    pub time_bucket: TimeBucket,
    pub price_bin: PriceBin,
    pub vol_regime: VolRegime,
}

impl BucketKey {
    pub fn new(time_bucket: TimeBucket, price_bin: PriceBin, vol_regime: VolRegime) -> Self {
        BucketKey {
            time_bucket,
            price_bin,
            vol_regime,
        }
    }

    pub fn to_key_string(&self) -> String {
        format!(
            "t={}_p={}_v={}",
            self.time_bucket, self.price_bin, self.vol_regime
        )
    }
}

impl fmt::Display for BucketKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_key_string())
    }
}

/// Classification of a bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyClassification {
    Whitelisted,
    Blacklisted,
    Neutral,
    Unknown,
}

impl fmt::Display for StrategyClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyClassification::Whitelisted => write!(f, "WHITELISTED"),
            StrategyClassification::Blacklisted => write!(f, "BLACKLISTED"),
            StrategyClassification::Neutral => write!(f, "NEUTRAL"),
            StrategyClassification::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Aggregated statistics for a bucket from historical data.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BucketStats {
    pub key: Option<BucketKey>,
    pub total_samples: usize,
    pub win_rate: f64,
    pub avg_pnl: f64,
    pub total_pnl: f64,
    pub sharpe: f64,
    pub oos_windows: usize,
    pub oos_trades: usize,
    pub pct_profitable_windows: f64,
    pub worst_window_loss: f64,
}

/// Out-of-sample profile for a bucket (walk-forward result).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BucketOosProfile {
    pub key: Option<BucketKey>,
    pub classification: StrategyClassification,
    pub classification_reason: String,
    pub stats: BucketStats,
    pub price_zone: Option<PriceZone>,
    pub vol_regime: Option<VolRegime>,
}

impl Default for StrategyClassification {
    fn default() -> Self {
        StrategyClassification::Unknown
    }
}

/// Derive the price bin for a given price (0–1), using 0.05 buckets.
pub fn price_bin_from_price(price: f64) -> PriceBin {
    let lo = (price * 20.0).floor() / 20.0;
    let hi = lo + 0.05;
    PriceBin {
        lo: (lo * 100.0).round() / 100.0,
        hi: (hi * 100.0).round() / 100.0,
    }
}

/// Derive vol regime from 30-second rolling std (configurable thresholds).
pub fn vol_regime_from_vol(vol: f64, low_threshold: f64, high_threshold: f64) -> VolRegime {
    if vol < low_threshold {
        VolRegime::Low
    } else if vol < high_threshold {
        VolRegime::Medium
    } else {
        VolRegime::High
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_bin_derivation() {
        let bin = price_bin_from_price(0.82);
        assert_eq!(bin.lo, 0.80);
        assert_eq!(bin.hi, 0.85);

        let bin2 = price_bin_from_price(0.50);
        assert_eq!(bin2.lo, 0.50);
        assert_eq!(bin2.hi, 0.55);

        let bin3 = price_bin_from_price(0.01);
        assert_eq!(bin3.lo, 0.00);
        assert_eq!(bin3.hi, 0.05);
    }

    #[test]
    fn test_vol_regime() {
        assert_eq!(vol_regime_from_vol(0.01, 0.02, 0.05), VolRegime::Low);
        assert_eq!(vol_regime_from_vol(0.03, 0.02, 0.05), VolRegime::Medium);
        assert_eq!(vol_regime_from_vol(0.07, 0.02, 0.05), VolRegime::High);
    }

    #[test]
    fn test_price_zone() {
        assert_eq!(PriceZone::from_price(0.80), PriceZone::HeavyFav);
        assert_eq!(PriceZone::from_price(0.65), PriceZone::ModerateFav);
        assert_eq!(PriceZone::from_price(0.50), PriceZone::NearTossup);
        assert_eq!(PriceZone::from_price(0.30), PriceZone::Underdog);
    }

    #[test]
    fn test_bucket_key_display() {
        let key = BucketKey::new(
            TimeBucket::Late,
            PriceBin { lo: 0.80, hi: 0.85 },
            VolRegime::High,
        );
        let s = key.to_key_string();
        assert!(s.contains("late"));
        assert!(s.contains("0.80-0.85"));
        assert!(s.contains("high"));
    }

    #[test]
    fn test_time_bucket() {
        assert_eq!(TimeBucket::from_seconds(300), TimeBucket::Early);
        assert_eq!(TimeBucket::from_seconds(180), TimeBucket::Mid);
        assert_eq!(TimeBucket::from_seconds(90), TimeBucket::Late);
        assert_eq!(TimeBucket::from_seconds(30), TimeBucket::VeryLate);
    }
}
