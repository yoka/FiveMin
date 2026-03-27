use crate::bucket::{BucketKey, PriceZone, StrategyClassification, VolRegime};
use crate::market::{MarketId, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A generated trade signal (TRADE or NO TRADE) with full explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub id: Uuid,
    pub market_id: MarketId,
    pub generated_at: DateTime<Utc>,
    pub artifact_version: Option<String>,
    pub decision: SignalDecision,
    pub explanation: SignalExplanation,
}

/// Whether the signal recommends a trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalDecision {
    Trade {
        side: Side,
        suggested_price: f64,
        size: f64,
    },
    NoTrade {
        reason: NoTradeReason,
    },
}

impl SignalDecision {
    pub fn is_trade(&self) -> bool {
        matches!(self, SignalDecision::Trade { .. })
    }
}

impl std::fmt::Display for SignalDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignalDecision::Trade { side, suggested_price, size } =>
                write!(f, "TRADE {} @ {:.3} size={:.2}", side, suggested_price, size),
            SignalDecision::NoTrade { reason } =>
                write!(f, "NO TRADE: {}", reason),
        }
    }
}

/// Reason why no trade was taken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoTradeReason {
    BucketBlacklisted,
    BucketNeutral,
    BucketUnknown,
    EdgeBelowThreshold { edge: f64, threshold: f64 },
    ProbabilityBelowThreshold { prob: f64, threshold: f64 },
    InsufficientSamples { samples: usize, required: usize },
    BannedPriceZone { zone: PriceZone },
    TimeLeftExceeded { time_left_sec: u64, max: u64 },
    ModeOff,
    ArtifactNotLoaded,
    SafetyCheckFailed { check: String },
}

impl std::fmt::Display for NoTradeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoTradeReason::BucketBlacklisted => write!(f, "bucket is BLACKLISTED"),
            NoTradeReason::BucketNeutral => write!(f, "bucket is NEUTRAL"),
            NoTradeReason::BucketUnknown => write!(f, "bucket is UNKNOWN (no data)"),
            NoTradeReason::EdgeBelowThreshold { edge, threshold } =>
                write!(f, "edge {:.4} < threshold {:.4}", edge, threshold),
            NoTradeReason::ProbabilityBelowThreshold { prob, threshold } =>
                write!(f, "prob {:.3} < min_prob {:.3}", prob, threshold),
            NoTradeReason::InsufficientSamples { samples, required } =>
                write!(f, "samples {} < required {}", samples, required),
            NoTradeReason::BannedPriceZone { zone } =>
                write!(f, "price zone {} is banned", zone),
            NoTradeReason::TimeLeftExceeded { time_left_sec, max } =>
                write!(f, "time_left {}s > max {}s", time_left_sec, max),
            NoTradeReason::ModeOff => write!(f, "mode is OFF"),
            NoTradeReason::ArtifactNotLoaded => write!(f, "strategy artifact not loaded"),
            NoTradeReason::SafetyCheckFailed { check } =>
                write!(f, "safety check failed: {}", check),
        }
    }
}

/// Full explainability record attached to every signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalExplanation {
    pub bucket_key: Option<BucketKey>,
    pub classification: StrategyClassification,
    pub classification_reason: String,
    pub price_zone: Option<PriceZone>,
    pub vol_regime: Option<VolRegime>,
    pub conservative_prob: f64,
    pub raw_prob: f64,
    pub edge: f64,
    pub cost_buffer: f64,
    pub samples: usize,
    pub safety_checks: Vec<(String, bool)>,
}

impl Default for SignalExplanation {
    fn default() -> Self {
        SignalExplanation {
            bucket_key: None,
            classification: StrategyClassification::Unknown,
            classification_reason: String::new(),
            price_zone: None,
            vol_regime: None,
            conservative_prob: 0.0,
            raw_prob: 0.0,
            edge: 0.0,
            cost_buffer: 0.0,
            samples: 0,
            safety_checks: Vec::new(),
        }
    }
}

/// A live market state used as input to the strategy scorer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveState {
    pub market_id: MarketId,
    pub time_left_sec: u64,
    pub up_price: f64,
    pub vol_30s: f64,
    pub captured_at: DateTime<Utc>,
}

/// An intent to place an order (not yet submitted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub id: Uuid,
    pub market_id: MarketId,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub signal_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl OrderIntent {
    pub fn new(market_id: MarketId, side: Side, price: f64, size: f64, signal_id: Uuid) -> Self {
        OrderIntent {
            id: Uuid::new_v4(),
            market_id,
            side,
            price,
            size,
            signal_id,
            created_at: Utc::now(),
        }
    }
}
