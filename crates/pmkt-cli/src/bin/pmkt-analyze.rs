/// pmkt-analyze: Offline strategy analysis tool.
///
/// Commands:
///   build-lookup    — Build lookup table from CSV data
///   build-whitelist — Classify buckets and build whitelist artifact  
///   backtest-whitelist — Run whitelist backtest over historical data
///   score-state     — Score a single live state against artifact
///   inspect-bucket  — Inspect a specific bucket's stats
use anyhow::Result;
use pmkt_data::csv_loader::load_csv_records_from_str;
use pmkt_strategy::{
    artifact::{demo_artifact, StrategyArtifact, StrategyConfig, WhitelistConfig},
    backtest::backtest_whitelist,
    classify::build_whitelist,
    lookup_table::build_lookup_table,
    score::score_state,
};
use pmkt_domain::{LiveState, MarketId};
use chrono::Utc;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "build-lookup" => cmd_build_lookup(&args[2..]),
        "build-whitelist" => cmd_build_whitelist(&args[2..]),
        "backtest-whitelist" => cmd_backtest_whitelist(&args[2..]),
        "score-state" => cmd_score_state(&args[2..]),
        "inspect-bucket" => cmd_inspect_bucket(&args[2..]),
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!("pmkt-analyze — Polymarket BTC 5m strategy analysis tool\n");
    println!("USAGE:");
    println!("  pmkt-analyze <COMMAND> [OPTIONS]\n");
    println!("COMMANDS:");
    println!("  build-lookup      Build lookup table from CSV data");
    println!("  build-whitelist   Classify buckets, build whitelist artifact");
    println!("  backtest-whitelist Run backtest over historical data");
    println!("  score-state       Score a live state");
    println!("    --time-left <secs>  Time remaining in market");
    println!("    --up-price <0..1>   Current UP outcome price");
    println!("    --vol <float>       30s rolling volatility");
    println!("  inspect-bucket    Inspect a bucket from artifact");
    println!("    --time-left <secs>");
    println!("    --price-bin <0.80-0.85>");
    println!("    --vol-regime <low|mid|high>");
    println!("\nExample (no data needed — uses demo artifact):");
    println!("  pmkt-analyze score-state --time-left 90 --up-price 0.82 --vol 0.03");
}

fn cmd_build_lookup(_args: &[String]) -> Result<()> {
    println!("build-lookup: No CSV path provided — using embedded demo data.\n");
    println!("In production: cargo run --bin pmkt-analyze -- build-lookup --input data/trades.csv\n");

    let demo_csv = "time_left_sec,up_price,vol_30s,outcome_correct,pnl\n\
        90,0.82,0.03,true,2.0\n\
        90,0.82,0.03,false,-1.0\n\
        90,0.82,0.03,true,2.0\n\
        120,0.65,0.04,true,1.5\n\
        120,0.65,0.04,false,-1.0\n";

    let records = load_csv_records_from_str(demo_csv)?;
    println!("Loaded {} records", records.len());

    let table = build_lookup_table(&records, 0.02, 0.05);
    println!("Built {} buckets:", table.len());
    for (k, v) in &table {
        println!(
            "  {} → trades={} win_rate={:.2} pnl={:.2}",
            k, v.oos_trades, v.win_rate, v.total_pnl
        );
    }
    Ok(())
}

fn cmd_build_whitelist(_args: &[String]) -> Result<()> {
    println!("build-whitelist: Using demo data.\n");

    let demo_csv = "time_left_sec,up_price,vol_30s,outcome_correct,pnl\n".to_string()
        + &(0..50)
            .map(|i| format!("{},0.82,0.03,{},2.0\n", 90, if i % 3 != 0 { "true" } else { "false" }))
            .collect::<String>();

    let records = load_csv_records_from_str(&demo_csv)?;
    let mut table = build_lookup_table(&records, 0.02, 0.05);

    // Set OOS stats for whitelist classification
    for stats in table.values_mut() {
        stats.oos_trades = stats.total_samples;
        stats.oos_windows = 5;
        stats.pct_profitable_windows = 0.70;
        stats.worst_window_loss = -3.0;
    }

    let config = WhitelistConfig::default();
    let (whitelist, blacklist, profiles) = build_whitelist(&table, &config);

    println!("Whitelist: {} buckets", whitelist.len());
    for k in &whitelist {
        println!("  ✓ {}", k);
    }
    println!("Blacklist: {} buckets", blacklist.len());
    for k in &blacklist {
        println!("  ✗ {}", k);
    }

    let artifact = StrategyArtifact::new(
        StrategyConfig::default(),
        config,
        profiles,
        whitelist,
        blacklist,
    );

    let json = serde_json::to_string_pretty(&artifact)?;
    println!("\nArtifact JSON (first 500 chars):\n{}", &json[..json.len().min(500)]);
    Ok(())
}

fn cmd_backtest_whitelist(_args: &[String]) -> Result<()> {
    println!("backtest-whitelist: Using demo artifact.\n");
    let artifact = demo_artifact();
    let demo_csv = "time_left_sec,up_price,vol_30s,outcome_correct,pnl\n\
        90,0.82,0.03,true,2.0\n\
        90,0.82,0.03,false,-1.0\n\
        90,0.75,0.03,true,1.5\n";
    let records = load_csv_records_from_str(demo_csv)?;
    let results = backtest_whitelist(&records, &artifact);
    println!("Total signals : {}", results.total_signals);
    println!("Trades taken  : {}", results.trades_taken);
    println!("Win rate      : {:.1}%", results.win_rate * 100.0);
    println!("Total P&L     : {:.2}", results.total_pnl);
    println!("\n(Demo artifact has empty whitelist, so 0 trades — load a real artifact)");
    Ok(())
}

fn cmd_score_state(args: &[String]) -> Result<()> {
    let time_left = parse_flag(args, "--time-left")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(90);
    let up_price = parse_flag(args, "--up-price")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.82);
    let vol = parse_flag(args, "--vol")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.03);

    let artifact = demo_artifact();
    let config = StrategyConfig::default();

    let state = LiveState {
        market_id: MarketId::from("cli-score"),
        time_left_sec: time_left,
        up_price,
        vol_30s: vol,
        captured_at: Utc::now(),
    };

    let signal = score_state(&state, &artifact, &config);

    println!("=== Score State ===");
    println!("Input: time_left={}s up_price={:.3} vol={:.4}", time_left, up_price, vol);
    println!("");
    println!("Decision     : {}", signal.decision);
    println!("Bucket Key   : {}", signal.explanation.bucket_key.as_ref().map(|k| k.to_key_string()).unwrap_or_else(|| "N/A".to_string()));
    println!("Classification: {}", signal.explanation.classification);
    println!("Reason       : {}", signal.explanation.classification_reason);
    println!("Price Zone   : {}", signal.explanation.price_zone.as_ref().map(|z| z.to_string()).unwrap_or_else(|| "N/A".to_string()));
    println!("Vol Regime   : {}", signal.explanation.vol_regime.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()));
    println!("Raw Prob     : {:.4}", signal.explanation.raw_prob);
    println!("Cons. Prob   : {:.4}", signal.explanation.conservative_prob);
    println!("Edge         : {:+.4}", signal.explanation.edge);
    println!("Samples      : {}", signal.explanation.samples);

    Ok(())
}

fn cmd_inspect_bucket(args: &[String]) -> Result<()> {
    let time_left_s = parse_flag(args, "--time-left").unwrap_or("90".to_string());
    let price_bin_s = parse_flag(args, "--price-bin").unwrap_or("0.80-0.85".to_string());
    let vol_regime_s = parse_flag(args, "--vol-regime").unwrap_or("mid".to_string());

    println!("=== Inspect Bucket ===");
    println!("time_left={} price_bin={} vol_regime={}", time_left_s, price_bin_s, vol_regime_s);
    println!("");
    println!("(Demo artifact has no profiles — load a real artifact built from historical data)");
    Ok(())
}

fn parse_flag(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}
