use serde::{Deserialize, Serialize};

/// Internal system events for the event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    MarketDiscovered { market_id: String },
    MarketActivated { market_id: String },
    OrderBookUpdated { market_id: String },
    SnapshotComputed { market_id: String },
    BucketDerived { market_id: String, bucket_key: String },
    SignalGenerated { market_id: String, is_trade: bool },
    SignalRejected { market_id: String, reason: String },
    OrderIntentCreated { market_id: String },
    OrderPlaced { market_id: String, order_id: String },
    OrderCancelled { order_id: String },
    OrderFilled { order_id: String },
    PositionOpened { market_id: String },
    PositionClosed { market_id: String },
    MarketResolved { market_id: String, outcome: String },
    PnlUpdated { net_pnl: f64 },
    RiskLimitTriggered { reason: String },
    ModeChanged { new_mode: String },
    KillSwitchTriggered,
    ErrorRaised { message: String },
}
