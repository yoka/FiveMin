use pmkt_domain::{OrderIntent, RiskLimits, RiskState, TradingMode};

/// Pre-trade safety check result.
#[derive(Debug)]
pub struct SafetyCheckResult {
    pub passed: bool,
    pub failures: Vec<String>,
}

impl SafetyCheckResult {
    pub fn ok() -> Self {
        SafetyCheckResult {
            passed: true,
            failures: vec![],
        }
    }

    pub fn fail(reason: impl Into<String>) -> Self {
        SafetyCheckResult {
            passed: false,
            failures: vec![reason.into()],
        }
    }
}

/// Run all pre-trade safety checks before submitting a real or simulated order.
pub fn run_pre_trade_checks(
    mode: TradingMode,
    intent: &OrderIntent,
    risk_state: &RiskState,
    risk_limits: &RiskLimits,
    market_data_age_secs: u64,
    order_book_age_secs: u64,
    time_left_sec: u64,
) -> SafetyCheckResult {
    let mut failures = Vec::new();

    // Kill switch
    if risk_state.kill_switch_triggered {
        failures.push("kill switch is active".to_string());
    }

    // Mode checks
    if mode == TradingMode::Off {
        failures.push("mode is OFF".to_string());
    }

    // LIVE pre-checks
    if mode.is_live() {
        if mode == TradingMode::LiveDisarmed {
            failures.push("live mode is DISARMED".to_string());
        }
        if !risk_state.artifact_valid {
            failures.push("strategy artifact is not valid".to_string());
        }
        if !risk_state.reconciliation_complete {
            failures.push("reconciliation not complete".to_string());
        }
        if !risk_state.connectivity_healthy {
            failures.push("exchange connectivity unhealthy".to_string());
        }
    }

    // Loss limits
    if risk_state.session_loss_limit_breached {
        failures.push("session loss limit breached".to_string());
    }
    if risk_state.daily_loss_limit_breached {
        failures.push("daily loss limit breached".to_string());
    }

    // Stale data
    if market_data_age_secs > risk_limits.stale_data_cutoff_secs {
        failures.push(format!(
            "market data stale: {}s old (max {}s)",
            market_data_age_secs, risk_limits.stale_data_cutoff_secs
        ));
    }
    if order_book_age_secs > risk_limits.stale_order_book_cutoff_secs {
        failures.push(format!(
            "order book stale: {}s old (max {}s)",
            order_book_age_secs, risk_limits.stale_order_book_cutoff_secs
        ));
    }

    // Too close to close
    if time_left_sec < risk_limits.no_entry_within_secs {
        failures.push(format!(
            "too close to market close: {}s remaining (min {}s)",
            time_left_sec, risk_limits.no_entry_within_secs
        ));
    }

    // Notional limit
    let notional = intent.price * intent.size;
    if notional > risk_limits.max_notional_per_trade {
        failures.push(format!(
            "notional {:.2} exceeds limit {:.2}",
            notional, risk_limits.max_notional_per_trade
        ));
    }

    if failures.is_empty() {
        SafetyCheckResult::ok()
    } else {
        SafetyCheckResult {
            passed: false,
            failures,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pmkt_domain::{MarketId, Side};
    use uuid::Uuid;
    use chrono::Utc;

    fn make_intent() -> OrderIntent {
        OrderIntent {
            id: Uuid::new_v4(),
            market_id: MarketId::from("test"),
            side: Side::Up,
            price: 0.80,
            size: 10.0,
            signal_id: Uuid::new_v4(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_off_mode_fails() {
        let intent = make_intent();
        let risk_state = RiskState::new();
        let limits = RiskLimits::default();
        let result = run_pre_trade_checks(
            TradingMode::Off,
            &intent,
            &risk_state,
            &limits,
            0, 0, 120,
        );
        assert!(!result.passed);
        assert!(result.failures.iter().any(|f| f.contains("OFF")));
    }

    #[test]
    fn test_kill_switch_fails() {
        let intent = make_intent();
        let mut risk_state = RiskState::new();
        risk_state.kill_switch_triggered = true;
        let limits = RiskLimits::default();
        let result = run_pre_trade_checks(
            TradingMode::Dry,
            &intent,
            &risk_state,
            &limits,
            0, 0, 120,
        );
        assert!(!result.passed);
        assert!(result.failures.iter().any(|f| f.contains("kill switch")));
    }

    #[test]
    fn test_live_disarmed_fails() {
        let intent = make_intent();
        let mut risk_state = RiskState::new();
        risk_state.artifact_valid = true;
        risk_state.reconciliation_complete = true;
        let limits = RiskLimits::default();
        let result = run_pre_trade_checks(
            TradingMode::LiveDisarmed,
            &intent,
            &risk_state,
            &limits,
            0, 0, 120,
        );
        assert!(!result.passed);
    }

    #[test]
    fn test_dry_mode_passes_with_fresh_data() {
        let intent = make_intent();
        let risk_state = RiskState::new();
        let limits = RiskLimits::default();
        let result = run_pre_trade_checks(
            TradingMode::Dry,
            &intent,
            &risk_state,
            &limits,
            0, 0, 120,
        );
        assert!(result.passed, "failures: {:?}", result.failures);
    }

    #[test]
    fn test_stale_data_fails() {
        let intent = make_intent();
        let risk_state = RiskState::new();
        let limits = RiskLimits::default();
        let result = run_pre_trade_checks(
            TradingMode::Dry,
            &intent,
            &risk_state,
            &limits,
            60, 0, 120, // market_data 60s old, exceeds 30s limit
        );
        assert!(!result.passed);
    }
}
