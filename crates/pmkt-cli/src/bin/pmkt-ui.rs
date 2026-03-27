/// pmkt-ui: Launch the interactive TUI dashboard.
///
/// Usage: pmkt-ui [--db <path>] [--artifact <path>]
use anyhow::Result;
use pmkt_app::new_shared_state;
use pmkt_domain::{
    BookLevel, BotState, Market, MarketId, MarketOutcome, MarketResult, MarketSnapshot,
    OrderBook, ParticipationMode, Side, TradingMode,
};
use pmkt_strategy::artifact::demo_artifact;
use pmkt_tui::run_tui;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Logging (to file to not interfere with TUI)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(|| {
            // In a real app, write to a log file
            std::io::stderr()
        })
        .init();

    let state = new_shared_state();

    // Seed demo data so the UI is immediately interesting
    seed_demo_data(&state);

    // Spawn a background task that periodically updates mock market data
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        demo_market_update_loop(state_clone).await;
    });

    // Run the TUI
    run_tui(state).await?;

    Ok(())
}

fn seed_demo_data(state: &pmkt_app::SharedAppState) {
    use chrono::{Duration, Utc};
    use uuid::Uuid;

    let mut s = state.write();

    // Load demo artifact
    let artifact = demo_artifact();
    s.load_artifact(artifact);
    s.risk_state.reconciliation_complete = true;
    s.set_mode(TradingMode::Dry);

    // Seed some recent market results
    let slugs = [
        ("BTC >67000 5m", MarketOutcome::Up, ParticipationMode::Dry, 3.50_f64, Some("t=late_p=0.80-0.85_v=mid")),
        ("BTC >67000 5m", MarketOutcome::Down, ParticipationMode::No, 0.0, None),
        ("BTC >67000 5m", MarketOutcome::Up, ParticipationMode::Dry, -1.20, Some("t=mid_p=0.65-0.70_v=low")),
        ("BTC >67000 5m", MarketOutcome::Down, ParticipationMode::No, 0.0, None),
        ("BTC >67000 5m", MarketOutcome::Up, ParticipationMode::Dry, 2.10, Some("t=late_p=0.70-0.75_v=mid")),
    ];

    for (i, (slug, outcome, participated, net_pnl, bucket)) in slugs.iter().enumerate() {
        let end_time = Utc::now() - Duration::minutes((i as i64 + 1) * 5);
        let result = MarketResult {
            id: Uuid::new_v4(),
            market_id: MarketId::from(format!("mkt-{}", i)),
            slug: slug.to_string(),
            start_time: end_time - Duration::minutes(5),
            end_time,
            outcome: *outcome,
            participated: *participated,
            entry_price: if *participated != ParticipationMode::No { Some(0.82) } else { None },
            exit_price: None,
            gross_pnl: *net_pnl,
            net_pnl: *net_pnl,
            fees: 0.10,
            bucket_key: bucket.map(|b| b.to_string()),
            skip_reason: if *participated == ParticipationMode::No {
                Some("bucket UNKNOWN".to_string())
            } else {
                None
            },
            mode_used: TradingMode::Dry,
        };
        s.add_market_result(result);
    }

    // Set a current market
    let current = Market {
        id: MarketId::from("mkt-current"),
        slug: "btc-usd-gt-67000-5m".to_string(),
        title: "Will BTC be above $67,000 in the next 5 minutes?".to_string(),
        start_time: Utc::now() - Duration::seconds(120),
        end_time: Utc::now() + Duration::seconds(180),
        active: true,
        closed: false,
        outcome: None,
    };
    s.set_current_market(current);

    // Set a snapshot
    let snap = MarketSnapshot::new(
        MarketId::from("mkt-current"),
        0.823,
        0.177,
        0.031,
        180,
    );
    s.update_snapshot(snap);

    // Set order book
    let ob = OrderBook {
        market_id: MarketId::from("mkt-current"),
        side: Side::Up,
        captured_at: Utc::now(),
        bids: vec![
            BookLevel { price: 0.820, size: 150.0 },
            BookLevel { price: 0.815, size: 200.0 },
            BookLevel { price: 0.810, size: 300.0 },
        ],
        asks: vec![
            BookLevel { price: 0.825, size: 120.0 },
            BookLevel { price: 0.830, size: 180.0 },
            BookLevel { price: 0.835, size: 250.0 },
        ],
    };
    s.update_orderbook(ob);

    // Set a demo signal (no trade since demo artifact has no whitelisted buckets)
    use pmkt_domain::{SignalDecision, SignalExplanation, NoTradeReason, StrategyClassification, TimeBucket, PriceZone, VolRegime};
    use pmkt_domain::{price_bin_from_price, vol_regime_from_vol, BucketKey};
    use pmkt_domain::TradeSignal;

    let bucket_key = BucketKey::new(
        TimeBucket::Late,
        price_bin_from_price(0.823),
        vol_regime_from_vol(0.031, 0.02, 0.05),
    );
    let signal = TradeSignal {
        id: Uuid::new_v4(),
        market_id: MarketId::from("mkt-current"),
        generated_at: Utc::now(),
        artifact_version: Some("demo-v1".to_string()),
        decision: SignalDecision::NoTrade {
            reason: NoTradeReason::BucketUnknown,
        },
        explanation: SignalExplanation {
            bucket_key: Some(bucket_key),
            classification: StrategyClassification::Unknown,
            classification_reason: "bucket unknown: no historical data in demo artifact — load a real artifact with pmkt-analyze".to_string(),
            price_zone: Some(PriceZone::HeavyFav),
            vol_regime: Some(VolRegime::Medium),
            conservative_prob: 0.823,
            raw_prob: 0.823,
            edge: -0.007,
            cost_buffer: 0.02,
            samples: 0,
            safety_checks: vec![
                ("time_left <= max (180s <= 240s)".to_string(), true),
                ("vol_30s present (0.0310)".to_string(), true),
            ],
        },
    };
    s.set_signal(signal);
    s.bot_state = BotState::Monitoring;
}

async fn demo_market_update_loop(state: pmkt_app::SharedAppState) {
    let mut tick = 0u64;
    loop {
        sleep(Duration::from_millis(500)).await;
        tick += 1;

        let mut s = state.write();

        // Wiggle the snapshot
        if let Some(snap) = &mut s.current_snapshot {
            let jitter = ((tick % 20) as f64 - 10.0) * 0.001;
            let new_up = (snap.up_price + jitter).clamp(0.01, 0.99);
            let new_down = 1.0 - new_up;
            *snap = pmkt_domain::MarketSnapshot::new(
                snap.market_id.clone(),
                new_up,
                new_down,
                snap.vol_30s,
                snap.time_left_sec.saturating_sub(1),
            );
        }
    }
}
