/// pmkt-admin: Administrative tools.
use anyhow::Result;

fn main() -> Result<()> {
    println!("pmkt-admin v0.1.0");
    println!("Administrative management tools for the Polymarket BTC 5m autotrader.");
    println!("");
    println!("Commands:");
    println!("  reset-db       Drop and recreate the database");
    println!("  show-state     Display current persisted state");
    println!("  purge-signals  Remove old signals from database");
    println!("  export-results Export market results to CSV");
    println!("");
    println!("(Admin commands not yet implemented in v1)");
    Ok(())
}
