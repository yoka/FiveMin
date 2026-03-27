use anyhow::Result;
use rusqlite::Connection;

/// Run all database migrations.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}

const SCHEMA_SQL: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS markets (
    id          TEXT PRIMARY KEY,
    slug        TEXT NOT NULL,
    title       TEXT NOT NULL,
    start_time  TEXT NOT NULL,
    end_time    TEXT NOT NULL,
    active      INTEGER NOT NULL DEFAULT 1,
    closed      INTEGER NOT NULL DEFAULT 0,
    outcome     TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS market_snapshots (
    id          TEXT PRIMARY KEY,
    market_id   TEXT NOT NULL REFERENCES markets(id),
    captured_at TEXT NOT NULL,
    up_price    REAL NOT NULL,
    down_price  REAL NOT NULL,
    vol_30s     REAL NOT NULL,
    time_left_sec INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS orderbooks (
    id          TEXT PRIMARY KEY,
    market_id   TEXT NOT NULL REFERENCES markets(id),
    side        TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    data_json   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS signals (
    id                  TEXT PRIMARY KEY,
    market_id           TEXT NOT NULL,
    generated_at        TEXT NOT NULL,
    artifact_version    TEXT,
    decision            TEXT NOT NULL,
    bucket_key          TEXT,
    classification      TEXT NOT NULL,
    classification_reason TEXT,
    price_zone          TEXT,
    vol_regime          TEXT,
    conservative_prob   REAL,
    raw_prob            REAL,
    edge                REAL,
    cost_buffer         REAL,
    samples             INTEGER,
    explanation_json    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS orders (
    id                  TEXT PRIMARY KEY,
    market_id           TEXT NOT NULL,
    side                TEXT NOT NULL,
    price               REAL NOT NULL,
    size                REAL NOT NULL,
    filled_size         REAL NOT NULL DEFAULT 0,
    status              TEXT NOT NULL,
    is_simulated        INTEGER NOT NULL DEFAULT 1,
    created_at          TEXT NOT NULL,
    updated_at          TEXT NOT NULL,
    exchange_order_id   TEXT,
    signal_id           TEXT
);

CREATE TABLE IF NOT EXISTS positions (
    market_id           TEXT PRIMARY KEY,
    side                TEXT NOT NULL,
    size                REAL NOT NULL,
    avg_entry_price     REAL NOT NULL,
    unrealized_pnl      REAL NOT NULL DEFAULT 0,
    realized_pnl        REAL NOT NULL DEFAULT 0,
    is_simulated        INTEGER NOT NULL DEFAULT 1,
    opened_at           TEXT,
    closed_at           TEXT
);

CREATE TABLE IF NOT EXISTS market_results (
    id                  TEXT PRIMARY KEY,
    market_id           TEXT NOT NULL,
    slug                TEXT NOT NULL,
    start_time          TEXT NOT NULL,
    end_time            TEXT NOT NULL,
    outcome             TEXT NOT NULL,
    participated        TEXT NOT NULL,
    entry_price         REAL,
    exit_price          REAL,
    gross_pnl           REAL NOT NULL DEFAULT 0,
    net_pnl             REAL NOT NULL DEFAULT 0,
    fees                REAL NOT NULL DEFAULT 0,
    bucket_key          TEXT,
    skip_reason         TEXT,
    mode_used           TEXT NOT NULL,
    created_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS system_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type  TEXT NOT NULL,
    market_id   TEXT,
    data_json   TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_signals_market ON signals(market_id);
CREATE INDEX IF NOT EXISTS idx_orders_market ON orders(market_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_market ON market_snapshots(market_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON system_events(event_type);
CREATE INDEX IF NOT EXISTS idx_results_market ON market_results(market_id);
"#;
