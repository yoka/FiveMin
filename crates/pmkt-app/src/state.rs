use pmkt_domain::{
    BotState, Market, MarketResult, MarketSnapshot, OrderBook, Position, PnlSummary,
    RiskLimits, RiskState, Side, TradeSignal, TradingMode,
};
use pmkt_strategy::artifact::StrategyArtifact;
use parking_lot::RwLock;
use std::sync::Arc;

/// The central shared application state.
/// Wrapped in Arc<RwLock<>> for shared async access.
#[derive(Debug, Default)]
pub struct AppState {
    pub mode: TradingMode,
    pub bot_state: BotState,
    pub risk_state: RiskState,
    pub risk_limits: RiskLimits,

    /// Currently active market (if any).
    pub current_market: Option<Market>,
    /// Latest snapshot of the current market.
    pub current_snapshot: Option<MarketSnapshot>,
    /// Latest order book for the UP outcome.
    pub current_orderbook_up: Option<OrderBook>,
    /// Latest order book for the DOWN outcome.
    pub current_orderbook_down: Option<OrderBook>,
    /// Latest generated signal.
    pub current_signal: Option<TradeSignal>,

    /// Current open position (if any).
    pub current_position: Option<Position>,

    /// Recent market results (last N markets).
    pub recent_results: Vec<MarketResult>,

    /// Strategy artifact in use.
    pub artifact: Option<StrategyArtifact>,

    /// Current P&L summary.
    pub pnl: PnlSummary,

    /// Last error message.
    pub last_error: Option<String>,
    /// Last action taken.
    pub last_action: Option<String>,

    /// Event log lines (most recent last).
    pub event_log: Vec<String>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            risk_state: RiskState::new(),
            risk_limits: RiskLimits::default(),
            ..Default::default()
        }
    }

    pub fn set_mode(&mut self, mode: TradingMode) {
        self.mode = mode;
        self.log_event(format!("Mode changed to {}", mode));
    }

    pub fn trigger_kill_switch(&mut self) {
        self.risk_state.trigger_kill_switch();
        self.mode = TradingMode::Off;
        self.bot_state = BotState::Halted;
        self.log_event("⚠️  KILL SWITCH TRIGGERED".to_string());
    }

    pub fn load_artifact(&mut self, artifact: StrategyArtifact) {
        self.risk_state.artifact_valid = true;
        self.log_event(format!("Artifact loaded: {}", artifact.artifact_id));
        self.artifact = Some(artifact);
    }

    pub fn set_current_market(&mut self, market: Market) {
        self.log_event(format!("Market activated: {}", market.slug));
        self.current_market = Some(market);
        self.bot_state = BotState::Monitoring;
    }

    pub fn update_snapshot(&mut self, snap: MarketSnapshot) {
        self.current_snapshot = Some(snap);
    }

    pub fn update_orderbook(&mut self, ob: OrderBook) {
        match ob.side {
            Side::Up => self.current_orderbook_up = Some(ob),
            Side::Down => self.current_orderbook_down = Some(ob),
        }
    }

    pub fn set_signal(&mut self, signal: TradeSignal) {
        self.log_event(format!(
            "Signal: {} | bucket={} cls={}",
            signal.decision,
            signal.explanation.bucket_key.as_ref().map(|k| k.to_key_string()).unwrap_or_default(),
            signal.explanation.classification,
        ));
        if signal.decision.is_trade() {
            self.bot_state = BotState::SignalReady;
        }
        self.current_signal = Some(signal);
    }

    pub fn add_market_result(&mut self, result: MarketResult) {
        self.pnl.net_pnl += result.net_pnl;
        self.pnl.gross_pnl += result.gross_pnl;
        self.pnl.total_fees += result.fees;
        self.recent_results.insert(0, result);
        if self.recent_results.len() > 50 {
            self.recent_results.truncate(50);
        }
    }

    pub fn log_event(&mut self, msg: String) {
        let ts = chrono::Utc::now().format("%H:%M:%S").to_string();
        self.event_log.push(format!("[{}] {}", ts, msg));
        if self.event_log.len() > 200 {
            self.event_log.remove(0);
        }
        self.last_action = Some(msg);
    }

    pub fn set_error(&mut self, err: String) {
        self.last_error = Some(err.clone());
        self.log_event(format!("ERROR: {}", err));
    }
}

/// Shared app state handle.
pub type SharedAppState = Arc<RwLock<AppState>>;

/// Create a new shared app state.
pub fn new_shared_state() -> SharedAppState {
    Arc::new(RwLock::new(AppState::new()))
}
