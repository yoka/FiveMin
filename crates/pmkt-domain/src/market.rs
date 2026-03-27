use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a Polymarket market.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarketId(pub String);

impl std::fmt::Display for MarketId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for MarketId {
    fn from(s: String) -> Self {
        MarketId(s)
    }
}

impl From<&str> for MarketId {
    fn from(s: &str) -> Self {
        MarketId(s.to_string())
    }
}

/// The final resolved outcome of a BTC 5m market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketOutcome {
    Up,
    Down,
    Unknown,
}

impl std::fmt::Display for MarketOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MarketOutcome::Up => write!(f, "UP"),
            MarketOutcome::Down => write!(f, "DOWN"),
            MarketOutcome::Unknown => write!(f, "?"),
        }
    }
}

/// The favored side according to strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Up,
    Down,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Up => write!(f, "UP"),
            Side::Down => write!(f, "DOWN"),
        }
    }
}

/// Full market metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub id: MarketId,
    pub slug: String,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub active: bool,
    pub closed: bool,
    pub outcome: Option<MarketOutcome>,
}

impl Market {
    pub fn seconds_remaining(&self) -> i64 {
        let now = Utc::now();
        let remaining = (self.end_time - now).num_seconds();
        remaining.max(0)
    }
}

/// A point-in-time snapshot of market prices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub id: Uuid,
    pub market_id: MarketId,
    pub captured_at: DateTime<Utc>,
    pub up_price: f64,
    pub down_price: f64,
    pub vol_30s: f64,
    pub time_left_sec: u64,
}

impl MarketSnapshot {
    pub fn new(
        market_id: MarketId,
        up_price: f64,
        down_price: f64,
        vol_30s: f64,
        time_left_sec: u64,
    ) -> Self {
        MarketSnapshot {
            id: Uuid::new_v4(),
            market_id,
            captured_at: Utc::now(),
            up_price,
            down_price,
            vol_30s,
            time_left_sec,
        }
    }

    /// The price of the favored side.
    pub fn favored_price(&self, side: Side) -> f64 {
        match side {
            Side::Up => self.up_price,
            Side::Down => self.down_price,
        }
    }
}

/// A single order book level (price/size pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookLevel {
    pub price: f64,
    pub size: f64,
}

/// Full order book for one outcome token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub market_id: MarketId,
    pub side: Side,
    pub captured_at: DateTime<Utc>,
    /// Sorted descending by price (best bid first).
    pub bids: Vec<BookLevel>,
    /// Sorted ascending by price (best ask first).
    pub asks: Vec<BookLevel>,
}

impl OrderBook {
    pub fn best_bid(&self) -> Option<f64> {
        self.bids.first().map(|l| l.price)
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks.first().map(|l| l.price)
    }

    pub fn spread(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn mid(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(bid), Some(ask)) => Some((bid + ask) / 2.0),
            _ => None,
        }
    }

    /// Bid/ask size imbalance: (bid_depth - ask_depth) / (bid_depth + ask_depth)
    pub fn imbalance(&self, levels: usize) -> Option<f64> {
        let bid_depth: f64 = self.bids.iter().take(levels).map(|l| l.size).sum();
        let ask_depth: f64 = self.asks.iter().take(levels).map(|l| l.size).sum();
        let total = bid_depth + ask_depth;
        if total == 0.0 {
            None
        } else {
            Some((bid_depth - ask_depth) / total)
        }
    }
}

/// A historical market result with participation info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketResult {
    pub id: Uuid,
    pub market_id: MarketId,
    pub slug: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub outcome: MarketOutcome,
    pub participated: ParticipationMode,
    pub entry_price: Option<f64>,
    pub exit_price: Option<f64>,
    pub gross_pnl: f64,
    pub net_pnl: f64,
    pub fees: f64,
    pub bucket_key: Option<String>,
    pub skip_reason: Option<String>,
    pub mode_used: TradingMode,
}

/// How the bot participated in a market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipationMode {
    No,
    Dry,
    Live,
}

impl std::fmt::Display for ParticipationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParticipationMode::No => write!(f, "NO"),
            ParticipationMode::Dry => write!(f, "DRY"),
            ParticipationMode::Live => write!(f, "LIVE"),
        }
    }
}

/// The current trading mode of the bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TradingMode {
    #[default]
    Off,
    Dry,
    LiveDisarmed,
    LiveArmed,
    LiveTrading,
}

impl std::fmt::Display for TradingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradingMode::Off => write!(f, "OFF"),
            TradingMode::Dry => write!(f, "DRY"),
            TradingMode::LiveDisarmed => write!(f, "LIVE_DISARMED"),
            TradingMode::LiveArmed => write!(f, "LIVE_ARMED"),
            TradingMode::LiveTrading => write!(f, "LIVE_TRADING"),
        }
    }
}

impl TradingMode {
    pub fn is_live(&self) -> bool {
        matches!(
            self,
            TradingMode::LiveDisarmed | TradingMode::LiveArmed | TradingMode::LiveTrading
        )
    }

    pub fn allows_real_orders(&self) -> bool {
        matches!(self, TradingMode::LiveTrading)
    }

    pub fn allows_sim_orders(&self) -> bool {
        matches!(self, TradingMode::Dry)
    }
}

/// High-level state of the bot state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BotState {
    #[default]
    Starting,
    Idle,
    Monitoring,
    SignalReady,
    EntryPending,
    Entered,
    ExitPending,
    Settled,
    Halted,
    Error,
}

impl std::fmt::Display for BotState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotState::Starting => write!(f, "STARTING"),
            BotState::Idle => write!(f, "IDLE"),
            BotState::Monitoring => write!(f, "MONITORING"),
            BotState::SignalReady => write!(f, "SIGNAL_READY"),
            BotState::EntryPending => write!(f, "ENTRY_PENDING"),
            BotState::Entered => write!(f, "ENTERED"),
            BotState::ExitPending => write!(f, "EXIT_PENDING"),
            BotState::Settled => write!(f, "SETTLED"),
            BotState::Halted => write!(f, "HALTED"),
            BotState::Error => write!(f, "ERROR"),
        }
    }
}
