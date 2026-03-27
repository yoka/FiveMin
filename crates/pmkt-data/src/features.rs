use pmkt_domain::{price_bin_from_price, vol_regime_from_vol, BucketKey, PriceZone, TimeBucket, VolRegime};

/// Derive all bucket features for a given market state.
pub struct DerivedFeatures {
    pub time_bucket: TimeBucket,
    pub price_bin: pmkt_domain::PriceBin,
    pub vol_regime: VolRegime,
    pub price_zone: PriceZone,
    pub bucket_key: BucketKey,
    pub favored_price: f64,
}

pub fn derive_features(
    time_left_sec: u64,
    up_price: f64,
    vol_30s: f64,
    vol_low: f64,
    vol_high: f64,
) -> DerivedFeatures {
    let time_bucket = TimeBucket::from_seconds(time_left_sec);
    let price_bin = price_bin_from_price(up_price);
    let vol_regime = vol_regime_from_vol(vol_30s, vol_low, vol_high);
    let favored_price = if up_price >= 0.5 { up_price } else { 1.0 - up_price };
    let price_zone = PriceZone::from_price(favored_price);
    let bucket_key = BucketKey::new(time_bucket, price_bin, vol_regime);

    DerivedFeatures {
        time_bucket,
        price_bin,
        vol_regime,
        price_zone,
        bucket_key,
        favored_price,
    }
}
