use anyhow::Result;
use pmkt_domain::TradeSignal;
use rusqlite::{params, Connection};
use serde_json;

/// Persist a trade signal with full explanation.
pub fn insert_signal(conn: &Connection, signal: &TradeSignal) -> Result<()> {
    let decision_str = signal.decision.to_string();
    let bucket_key = signal
        .explanation
        .bucket_key
        .as_ref()
        .map(|k| k.to_key_string());
    let classification = signal.explanation.classification.to_string();
    let price_zone = signal
        .explanation
        .price_zone
        .as_ref()
        .map(|z| z.to_string());
    let vol_regime = signal
        .explanation
        .vol_regime
        .as_ref()
        .map(|v| v.to_string());
    let explanation_json = serde_json::to_string(&signal.explanation)?;

    conn.execute(
        "INSERT OR IGNORE INTO signals
         (id, market_id, generated_at, artifact_version, decision, bucket_key, classification,
          classification_reason, price_zone, vol_regime, conservative_prob, raw_prob, edge,
          cost_buffer, samples, explanation_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            signal.id.to_string(),
            signal.market_id.0,
            signal.generated_at.to_rfc3339(),
            signal.artifact_version,
            decision_str,
            bucket_key,
            classification,
            signal.explanation.classification_reason,
            price_zone,
            vol_regime,
            signal.explanation.conservative_prob,
            signal.explanation.raw_prob,
            signal.explanation.edge,
            signal.explanation.cost_buffer,
            signal.explanation.samples as i64,
            explanation_json,
        ],
    )?;
    Ok(())
}

/// Record a system event.
pub fn insert_system_event(
    conn: &Connection,
    event_type: &str,
    market_id: Option<&str>,
    data: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO system_events (event_type, market_id, data_json) VALUES (?1, ?2, ?3)",
        params![event_type, market_id, data],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use pmkt_domain::{LiveState, MarketId, SignalDecision, SignalExplanation, StrategyClassification, TradeSignal};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_insert_signal() {
        let conn = open_in_memory().unwrap();
        let signal = TradeSignal {
            id: Uuid::new_v4(),
            market_id: MarketId::from("mkt-001"),
            generated_at: Utc::now(),
            artifact_version: Some("v1".to_string()),
            decision: pmkt_domain::SignalDecision::NoTrade {
                reason: pmkt_domain::NoTradeReason::BucketUnknown,
            },
            explanation: SignalExplanation::default(),
        };
        insert_signal(&conn, &signal).unwrap();
    }
}
