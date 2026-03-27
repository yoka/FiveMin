/// pmkt-replay: Replay recorded historical sessions to validate signals.
use anyhow::Result;

fn main() -> Result<()> {
    println!("pmkt-replay v0.1.0");
    println!("Replay recorded historical sessions to validate expected signals and P&L.");
    println!("");
    println!("Usage: pmkt-replay --input <session.json> [--artifact <whitelist.json>]");
    println!("");
    println!("(Not yet implemented in v1 — replay engine coming in M3)");
    Ok(())
}
