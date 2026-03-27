use pmkt_domain::{ExecutionOrder, OrderIntent, OrderStatus};
use chrono::Utc;

/// DRY mode fill simulator.
/// Uses a conservative touch-based fill model.
pub struct DrySimulator {
    /// Fraction of spread as slippage cost
    pub slippage_fraction: f64,
    /// Flat fee per trade in USDC
    pub flat_fee: f64,
}

impl DrySimulator {
    pub fn new() -> Self {
        DrySimulator {
            slippage_fraction: 0.5,
            flat_fee: 0.0,
        }
    }

    /// Simulate placing an order from an intent.
    /// Returns an immediately "filled" simulated order.
    pub fn simulate_fill(&self, intent: &OrderIntent) -> ExecutionOrder {
        let fill_price = intent.price + self.slippage_fraction * 0.01;
        let now = Utc::now();
        ExecutionOrder {
            id: uuid::Uuid::new_v4(),
            market_id: intent.market_id.clone(),
            side: intent.side,
            price: fill_price,
            size: intent.size,
            filled_size: intent.size,
            status: OrderStatus::Filled,
            is_simulated: true,
            created_at: now,
            updated_at: now,
            exchange_order_id: None,
            signal_id: Some(intent.signal_id),
        }
    }

    /// Calculate fee for a trade.
    pub fn fee(&self, size: f64, price: f64) -> f64 {
        self.flat_fee + size * price * 0.002 // 0.2% maker fee assumption
    }
}

impl Default for DrySimulator {
    fn default() -> Self {
        DrySimulator::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pmkt_domain::{MarketId, Side};
    use uuid::Uuid;

    #[test]
    fn test_dry_fill() {
        let sim = DrySimulator::new();
        let intent = OrderIntent::new(
            MarketId::from("test"),
            Side::Up,
            0.80,
            10.0,
            Uuid::new_v4(),
        );
        let order = sim.simulate_fill(&intent);
        assert_eq!(order.status, OrderStatus::Filled);
        assert!(order.is_simulated);
        assert_eq!(order.filled_size, 10.0);
        assert!(order.avg_fill_price().is_some());
    }
}
