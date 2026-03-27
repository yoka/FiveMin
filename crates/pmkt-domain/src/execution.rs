use crate::market::{MarketId, Side};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of an exchange order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Open,
    Filled,
    PartiallyFilled,
    Cancelled,
    Rejected,
}

impl std::fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderStatus::Pending => write!(f, "PENDING"),
            OrderStatus::Open => write!(f, "OPEN"),
            OrderStatus::Filled => write!(f, "FILLED"),
            OrderStatus::PartiallyFilled => write!(f, "PARTIAL"),
            OrderStatus::Cancelled => write!(f, "CANCELLED"),
            OrderStatus::Rejected => write!(f, "REJECTED"),
        }
    }
}

/// An order placed (or simulated) on the exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOrder {
    pub id: Uuid,
    pub market_id: MarketId,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub filled_size: f64,
    pub status: OrderStatus,
    pub is_simulated: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub exchange_order_id: Option<String>,
    pub signal_id: Option<Uuid>,
}

impl ExecutionOrder {
    pub fn new_simulated(market_id: MarketId, side: Side, price: f64, size: f64, signal_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        ExecutionOrder {
            id: Uuid::new_v4(),
            market_id,
            side,
            price,
            size,
            filled_size: 0.0,
            status: OrderStatus::Pending,
            is_simulated: true,
            created_at: now,
            updated_at: now,
            exchange_order_id: None,
            signal_id,
        }
    }

    pub fn avg_fill_price(&self) -> Option<f64> {
        if self.filled_size > 0.0 {
            Some(self.price)
        } else {
            None
        }
    }
}

/// Current position in a market.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub market_id: MarketId,
    pub side: Side,
    pub size: f64,
    pub avg_entry_price: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub is_simulated: bool,
    pub opened_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl Default for Side {
    fn default() -> Self {
        Side::Up
    }
}

impl Default for MarketId {
    fn default() -> Self {
        MarketId(String::new())
    }
}

impl Position {
    pub fn net_pnl(&self) -> f64 {
        self.unrealized_pnl + self.realized_pnl
    }
}

/// P&L summary across various dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PnlSummary {
    pub session_pnl: f64,
    pub daily_pnl: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub gross_pnl: f64,
    pub net_pnl: f64,
    pub total_fees: f64,
}

impl PnlSummary {
    pub fn win_rate(&self) -> f64 {
        if self.total_trades == 0 {
            0.0
        } else {
            self.winning_trades as f64 / self.total_trades as f64
        }
    }
}
