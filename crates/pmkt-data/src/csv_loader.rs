use pmkt_strategy::lookup_table::TradeRecord;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// CSV row format for historical BTC 5m trade data.
#[derive(Debug, Deserialize)]
pub struct CsvTradeRow {
    pub time_left_sec: u64,
    pub up_price: f64,
    pub vol_30s: f64,
    pub outcome_correct: bool,
    pub pnl: f64,
}

/// Load trade records from a CSV file.
/// Expected columns: time_left_sec, up_price, vol_30s, outcome_correct, pnl
pub fn load_csv_records(path: &Path) -> Result<Vec<TradeRecord>> {
    let mut rdr = csv::Reader::from_path(path)
        .with_context(|| format!("opening CSV: {}", path.display()))?;

    let mut records = Vec::new();
    for result in rdr.deserialize::<CsvTradeRow>() {
        let row = result.with_context(|| "parsing CSV row")?;
        records.push(TradeRecord {
            time_left_sec: row.time_left_sec,
            up_price: row.up_price,
            vol_30s: row.vol_30s,
            outcome_correct: row.outcome_correct,
            pnl: row.pnl,
        });
    }

    Ok(records)
}

/// Load trade records from CSV string content (for testing).
pub fn load_csv_records_from_str(content: &str) -> Result<Vec<TradeRecord>> {
    let mut rdr = csv::Reader::from_reader(content.as_bytes());
    let mut records = Vec::new();
    for result in rdr.deserialize::<CsvTradeRow>() {
        let row = result.with_context(|| "parsing CSV row")?;
        records.push(TradeRecord {
            time_left_sec: row.time_left_sec,
            up_price: row.up_price,
            vol_30s: row.vol_30s,
            outcome_correct: row.outcome_correct,
            pnl: row.pnl,
        });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_csv_from_str() {
        let csv = "time_left_sec,up_price,vol_30s,outcome_correct,pnl\n\
                   90,0.82,0.03,true,2.0\n\
                   120,0.65,0.04,false,-1.0\n";
        let records = load_csv_records_from_str(csv).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].time_left_sec, 90);
        assert!((records[0].up_price - 0.82).abs() < 1e-9);
        assert!(records[0].outcome_correct);
        assert!(!records[1].outcome_correct);
    }
}
