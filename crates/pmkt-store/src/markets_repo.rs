use anyhow::Result;
use pmkt_domain::{Market, MarketId, MarketOutcome, MarketResult, ParticipationMode, TradingMode};
use rusqlite::{params, Connection};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Insert or replace a market record.
pub fn upsert_market(conn: &Connection, market: &Market) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO markets (id, slug, title, start_time, end_time, active, closed, outcome)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            market.id.0,
            market.slug,
            market.title,
            market.start_time.to_rfc3339(),
            market.end_time.to_rfc3339(),
            market.active as i64,
            market.closed as i64,
            market.outcome.map(|o| o.to_string()),
        ],
    )?;
    Ok(())
}

/// Insert a market result record.
pub fn insert_market_result(conn: &Connection, result: &MarketResult) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO market_results
         (id, market_id, slug, start_time, end_time, outcome, participated, entry_price,
          exit_price, gross_pnl, net_pnl, fees, bucket_key, skip_reason, mode_used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            result.id.to_string(),
            result.market_id.0,
            result.slug,
            result.start_time.to_rfc3339(),
            result.end_time.to_rfc3339(),
            result.outcome.to_string(),
            result.participated.to_string(),
            result.entry_price,
            result.exit_price,
            result.gross_pnl,
            result.net_pnl,
            result.fees,
            result.bucket_key,
            result.skip_reason,
            result.mode_used.to_string(),
        ],
    )?;
    Ok(())
}

/// Load recent market results ordered by end_time DESC.
pub fn load_recent_market_results(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<MarketResult>> {
    let mut stmt = conn.prepare(
        "SELECT id, market_id, slug, start_time, end_time, outcome, participated,
                entry_price, exit_price, gross_pnl, net_pnl, fees, bucket_key, skip_reason, mode_used
         FROM market_results
         ORDER BY end_time DESC
         LIMIT ?1",
    )?;

    let results = stmt.query_map(params![limit as i64], |row| {
        let outcome_str: String = row.get(5)?;
        let participated_str: String = row.get(6)?;
        let mode_str: String = row.get(14)?;

        Ok(MarketResult {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::new_v4()),
            market_id: MarketId(row.get(1)?),
            slug: row.get(2)?,
            start_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                .unwrap_or_default()
                .with_timezone(&Utc),
            end_time: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                .unwrap_or_default()
                .with_timezone(&Utc),
            outcome: parse_outcome(&outcome_str),
            participated: parse_participation(&participated_str),
            entry_price: row.get(7)?,
            exit_price: row.get(8)?,
            gross_pnl: row.get(9)?,
            net_pnl: row.get(10)?,
            fees: row.get(11)?,
            bucket_key: row.get(12)?,
            skip_reason: row.get(13)?,
            mode_used: parse_mode(&mode_str),
        })
    })?
    .filter_map(|r| r.ok())
    .collect();

    Ok(results)
}

fn parse_outcome(s: &str) -> MarketOutcome {
    match s {
        "UP" => MarketOutcome::Up,
        "DOWN" => MarketOutcome::Down,
        _ => MarketOutcome::Unknown,
    }
}

fn parse_participation(s: &str) -> ParticipationMode {
    match s {
        "DRY" => ParticipationMode::Dry,
        "LIVE" => ParticipationMode::Live,
        _ => ParticipationMode::No,
    }
}

fn parse_mode(s: &str) -> TradingMode {
    match s {
        "DRY" => TradingMode::Dry,
        "LIVE_TRADING" => TradingMode::LiveTrading,
        "LIVE_ARMED" => TradingMode::LiveArmed,
        "LIVE_DISARMED" => TradingMode::LiveDisarmed,
        _ => TradingMode::Off,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use chrono::Utc;

    fn make_result() -> MarketResult {
        MarketResult {
            id: Uuid::new_v4(),
            market_id: MarketId::from("mkt-001"),
            slug: "btc-5m-test".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            outcome: MarketOutcome::Up,
            participated: ParticipationMode::Dry,
            entry_price: Some(0.82),
            exit_price: None,
            gross_pnl: 5.0,
            net_pnl: 4.5,
            fees: 0.5,
            bucket_key: Some("t=late_p=0.80-0.85_v=mid".to_string()),
            skip_reason: None,
            mode_used: TradingMode::Dry,
        }
    }

    #[test]
    fn test_insert_and_load_market_results() {
        let conn = open_in_memory().unwrap();
        let result = make_result();
        insert_market_result(&conn, &result).unwrap();
        let loaded = load_recent_market_results(&conn, 10).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].slug, "btc-5m-test");
        assert_eq!(loaded[0].outcome, MarketOutcome::Up);
    }
}
