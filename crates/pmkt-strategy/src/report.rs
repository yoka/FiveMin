use crate::artifact::StrategyArtifact;
use crate::backtest::BacktestResults;

/// Generate a text strategy report.
pub fn generate_report(artifact: &StrategyArtifact, backtest: Option<&BacktestResults>) -> String {
    let mut lines = Vec::new();

    lines.push("=== PMKT Strategy Report ===".to_string());
    lines.push(format!("Artifact ID : {}", artifact.artifact_id));
    lines.push(format!("Built at    : {}", artifact.built_at));
    lines.push(format!("Schema v    : {}", artifact.schema_version));
    lines.push(String::new());
    lines.push(format!("Whitelisted buckets : {}", artifact.whitelist.len()));
    lines.push(format!("Blacklisted buckets : {}", artifact.blacklist.len()));
    lines.push(format!("Total profiles      : {}", artifact.profiles.len()));
    lines.push(String::new());

    lines.push("--- Whitelist ---".to_string());
    for k in &artifact.whitelist {
        if let Some(profile) = artifact.profiles.get(k) {
            lines.push(format!(
                "  {} | trades={} win_rate={:.2} pnl={:.2} | {}",
                k,
                profile.stats.oos_trades,
                profile.stats.win_rate,
                profile.stats.total_pnl,
                profile.classification_reason
            ));
        } else {
            lines.push(format!("  {}", k));
        }
    }
    lines.push(String::new());

    lines.push("--- Blacklist ---".to_string());
    for k in &artifact.blacklist {
        if let Some(profile) = artifact.profiles.get(k) {
            lines.push(format!(
                "  {} | {}",
                k, profile.classification_reason
            ));
        } else {
            lines.push(format!("  {}", k));
        }
    }
    lines.push(String::new());

    if let Some(bt) = backtest {
        lines.push("--- Backtest Results ---".to_string());
        lines.push(format!("Total signals  : {}", bt.total_signals));
        lines.push(format!("Trades taken   : {}", bt.trades_taken));
        lines.push(format!("Win rate       : {:.1}%", bt.win_rate * 100.0));
        lines.push(format!("Total P&L      : {:.2}", bt.total_pnl));
    }

    lines.join("\n")
}
