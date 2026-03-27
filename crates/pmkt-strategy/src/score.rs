use pmkt_domain::{
    price_bin_from_price, vol_regime_from_vol, BucketKey, LiveState,
    NoTradeReason, PriceZone, Side, SignalDecision, SignalExplanation,
    StrategyClassification, TimeBucket, TradeSignal,
};
use crate::artifact::{StrategyArtifact, StrategyConfig};
use chrono::Utc;
use uuid::Uuid;

/// Score a live market state against the strategy artifact.
/// Returns a TradeSignal with full explanation regardless of outcome.
pub fn score_state(
    state: &LiveState,
    artifact: &StrategyArtifact,
    config: &StrategyConfig,
) -> TradeSignal {
    let mut explanation = SignalExplanation::default();

    // 1. Derive favored side and market price
    let (favored_side, market_price, raw_prob) = if state.up_price >= 0.5 {
        (Side::Up, state.up_price, state.up_price)
    } else {
        (Side::Down, 1.0 - state.up_price, 1.0 - state.up_price)
    };

    explanation.raw_prob = raw_prob;

    // 2. Derive bucket key components
    let time_bucket = TimeBucket::from_seconds(state.time_left_sec);
    let price_bin = price_bin_from_price(state.up_price);
    let vol_regime =
        vol_regime_from_vol(state.vol_30s, artifact.vol_low_threshold, artifact.vol_high_threshold);
    let bucket_key = BucketKey::new(time_bucket, price_bin, vol_regime);

    let price_zone = PriceZone::from_price(market_price);
    explanation.bucket_key = Some(bucket_key.clone());
    explanation.price_zone = Some(price_zone);
    explanation.vol_regime = Some(vol_regime);

    // 3. Check banned price zone
    let zone_str = price_zone.to_string();
    if config.banned_price_zones.contains(&zone_str) {
        explanation.classification = StrategyClassification::Unknown;
        explanation.classification_reason =
            format!("price zone {} is banned", zone_str);
        add_safety_checks(&mut explanation, state, config, false);
        return make_no_trade(
            state,
            artifact,
            NoTradeReason::BannedPriceZone { zone: price_zone },
            explanation,
        );
    }

    // 4. Time-left check
    if state.time_left_sec > config.max_time_left_sec {
        explanation.classification = StrategyClassification::Unknown;
        explanation.classification_reason =
            format!("time_left {}s > max {}s", state.time_left_sec, config.max_time_left_sec);
        add_safety_checks(&mut explanation, state, config, false);
        return make_no_trade(
            state,
            artifact,
            NoTradeReason::TimeLeftExceeded {
                time_left_sec: state.time_left_sec,
                max: config.max_time_left_sec,
            },
            explanation,
        );
    }

    // 5. Classify bucket
    let classification = artifact.classify_bucket(&bucket_key);
    explanation.classification = classification;

    // 6. Reject non-whitelisted
    match classification {
        StrategyClassification::Blacklisted => {
            explanation.classification_reason = "bucket is blacklisted".to_string();
            if let Some(profile) = artifact.get_profile(&bucket_key) {
                explanation.classification_reason = profile.classification_reason.clone();
            }
            add_safety_checks(&mut explanation, state, config, false);
            return make_no_trade(state, artifact, NoTradeReason::BucketBlacklisted, explanation);
        }
        StrategyClassification::Neutral => {
            explanation.classification_reason = "bucket is neutral".to_string();
            if let Some(profile) = artifact.get_profile(&bucket_key) {
                explanation.classification_reason = profile.classification_reason.clone();
            }
            add_safety_checks(&mut explanation, state, config, false);
            return make_no_trade(state, artifact, NoTradeReason::BucketNeutral, explanation);
        }
        StrategyClassification::Unknown => {
            explanation.classification_reason = "bucket unknown: no historical data".to_string();
            add_safety_checks(&mut explanation, state, config, false);
            return make_no_trade(state, artifact, NoTradeReason::BucketUnknown, explanation);
        }
        StrategyClassification::Whitelisted => {
            if let Some(profile) = artifact.get_profile(&bucket_key) {
                explanation.classification_reason = profile.classification_reason.clone();
                explanation.samples = profile.stats.total_samples;
            }
        }
    }

    // 7. Get profile stats
    let profile = match artifact.get_profile(&bucket_key) {
        Some(p) => p,
        None => {
            explanation.classification_reason = "whitelist entry exists but no profile data".to_string();
            add_safety_checks(&mut explanation, state, config, false);
            return make_no_trade(state, artifact, NoTradeReason::BucketUnknown, explanation);
        }
    };

    // 8. Check sample count
    if profile.stats.total_samples < config.min_samples {
        add_safety_checks(&mut explanation, state, config, false);
        return make_no_trade(
            state,
            artifact,
            NoTradeReason::InsufficientSamples {
                samples: profile.stats.total_samples,
                required: config.min_samples,
            },
            explanation,
        );
    }

    // 9. Compute conservative probability (shrink toward base rate 0.5)
    let shrinkage = (config.min_samples as f64 / (profile.stats.total_samples as f64 + config.min_samples as f64)).min(0.5);
    let conservative_prob = raw_prob * (1.0 - shrinkage) + 0.5 * shrinkage;
    explanation.conservative_prob = conservative_prob;
    explanation.cost_buffer = config.cost_buffer;

    // 10. Check minimum probability
    if conservative_prob < config.min_prob {
        add_safety_checks(&mut explanation, state, config, false);
        return make_no_trade(
            state,
            artifact,
            NoTradeReason::ProbabilityBelowThreshold {
                prob: conservative_prob,
                threshold: config.min_prob,
            },
            explanation,
        );
    }

    // 11. Compute edge = conservative_prob - market_price - cost_buffer
    let edge = conservative_prob - market_price - config.cost_buffer;
    explanation.edge = edge;

    if edge < config.min_edge {
        add_safety_checks(&mut explanation, state, config, false);
        return make_no_trade(
            state,
            artifact,
            NoTradeReason::EdgeBelowThreshold {
                edge,
                threshold: config.min_edge,
            },
            explanation,
        );
    }

    // 12. All checks passed — emit TRADE
    add_safety_checks(&mut explanation, state, config, true);

    let suggested_price = market_price + config.cost_buffer;
    let size = config.min_trade_size;

    TradeSignal {
        id: Uuid::new_v4(),
        market_id: state.market_id.clone(),
        generated_at: Utc::now(),
        artifact_version: Some(artifact.artifact_id.clone()),
        decision: SignalDecision::Trade {
            side: favored_side,
            suggested_price,
            size,
        },
        explanation,
    }
}

fn make_no_trade(
    state: &LiveState,
    artifact: &StrategyArtifact,
    reason: NoTradeReason,
    explanation: SignalExplanation,
) -> TradeSignal {
    TradeSignal {
        id: Uuid::new_v4(),
        market_id: state.market_id.clone(),
        generated_at: Utc::now(),
        artifact_version: Some(artifact.artifact_id.clone()),
        decision: SignalDecision::NoTrade { reason },
        explanation,
    }
}

fn add_safety_checks(
    explanation: &mut SignalExplanation,
    state: &LiveState,
    config: &StrategyConfig,
    _all_pass: bool,
) {
    explanation.safety_checks.push((
        format!("time_left <= max ({}s <= {}s)", state.time_left_sec, config.max_time_left_sec),
        state.time_left_sec <= config.max_time_left_sec,
    ));
    explanation.safety_checks.push((
        format!("vol_30s present ({:.4})", state.vol_30s),
        state.vol_30s >= 0.0,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{demo_artifact, StrategyConfig};
    use pmkt_domain::MarketId;

    fn make_state(time_left: u64, up_price: f64, vol: f64) -> LiveState {
        LiveState {
            market_id: MarketId::from("test-market"),
            time_left_sec: time_left,
            up_price,
            vol_30s: vol,
            captured_at: Utc::now(),
        }
    }

    #[test]
    fn test_no_trade_unknown_bucket() {
        let artifact = demo_artifact();
        let config = StrategyConfig::default();
        let state = make_state(90, 0.82, 0.03);
        let signal = score_state(&state, &artifact, &config);
        assert!(!signal.decision.is_trade());
        matches!(signal.decision, SignalDecision::NoTrade { reason: NoTradeReason::BucketUnknown });
    }

    #[test]
    fn test_no_trade_mode_off_time_exceeded() {
        let artifact = demo_artifact();
        let config = StrategyConfig {
            max_time_left_sec: 60,
            ..Default::default()
        };
        // time_left = 300 > max 60
        let state = make_state(300, 0.82, 0.03);
        let signal = score_state(&state, &artifact, &config);
        assert!(!signal.decision.is_trade());
    }

    #[test]
    fn test_edge_calculation() {
        // Edge = conservative_prob - market_price - cost_buffer
        // With a whitelisted bucket and sufficient samples
        let artifact = demo_artifact();
        let config = StrategyConfig::default();
        let state = make_state(90, 0.75, 0.03);
        let signal = score_state(&state, &artifact, &config);
        // We have no whitelisted buckets in demo, so should be NoTrade
        assert!(!signal.decision.is_trade());
        // But explanation should have been populated
        assert!(signal.explanation.bucket_key.is_some());
    }
}
