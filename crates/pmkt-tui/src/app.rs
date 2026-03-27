use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use pmkt_app::SharedAppState;
use pmkt_domain::TradingMode;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::time::Duration;

use crate::ui::draw_ui;

/// Currently active tab/panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Main,
    BucketDiagnostics,
    SignalExplanation,
    OrderLog,
    SystemHealth,
    RiskAlerts,
}

impl ActiveTab {
    pub fn next(self) -> Self {
        match self {
            ActiveTab::Main => ActiveTab::BucketDiagnostics,
            ActiveTab::BucketDiagnostics => ActiveTab::SignalExplanation,
            ActiveTab::SignalExplanation => ActiveTab::OrderLog,
            ActiveTab::OrderLog => ActiveTab::SystemHealth,
            ActiveTab::SystemHealth => ActiveTab::RiskAlerts,
            ActiveTab::RiskAlerts => ActiveTab::Main,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ActiveTab::Main => "Main",
            ActiveTab::BucketDiagnostics => "Bucket",
            ActiveTab::SignalExplanation => "Signal",
            ActiveTab::OrderLog => "Orders",
            ActiveTab::SystemHealth => "Health",
            ActiveTab::RiskAlerts => "Risk",
        }
    }
}

/// TUI application loop.
pub async fn run_tui(state: SharedAppState) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, state).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: SharedAppState,
) -> Result<()> {
    let mut active_tab = ActiveTab::Main;
    let tick_rate = Duration::from_millis(150); // ~6 Hz

    loop {
        terminal.draw(|f| {
            draw_ui(f, &state, active_tab);
        })?;

        if crossterm::event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    // Quit
                    (KeyCode::Char('q'), _) | (KeyCode::Char('Q'), _) => break,
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                    // Tab navigation
                    (KeyCode::Tab, _) => active_tab = active_tab.next(),
                    (KeyCode::Char('1'), _) => active_tab = ActiveTab::Main,
                    (KeyCode::Char('2'), _) => active_tab = ActiveTab::BucketDiagnostics,
                    (KeyCode::Char('3'), _) => active_tab = ActiveTab::SignalExplanation,
                    (KeyCode::Char('4'), _) => active_tab = ActiveTab::OrderLog,
                    (KeyCode::Char('5'), _) => active_tab = ActiveTab::SystemHealth,

                    // Mode control
                    (KeyCode::Char('m'), _) => {
                        let mut s = state.write();
                        let new_mode = match s.mode {
                            TradingMode::Off => TradingMode::Dry,
                            TradingMode::Dry => TradingMode::LiveDisarmed,
                            TradingMode::LiveDisarmed => TradingMode::Off,
                            _ => TradingMode::Off,
                        };
                        s.set_mode(new_mode);
                    }

                    // Arm/disarm live
                    (KeyCode::Char('a'), _) => {
                        let mut s = state.write();
                        match s.mode {
                            TradingMode::LiveDisarmed => {
                                if s.risk_state.is_safe_for_live() {
                                    s.set_mode(TradingMode::LiveArmed);
                                } else {
                                    s.set_error("Cannot arm: safety checks failed".to_string());
                                }
                            }
                            TradingMode::LiveArmed => {
                                s.set_mode(TradingMode::LiveDisarmed);
                            }
                            _ => {}
                        }
                    }

                    // Kill switch
                    (KeyCode::Char('k'), _) => {
                        let mut s = state.write();
                        s.trigger_kill_switch();
                    }

                    // Reload artifacts (stub)
                    (KeyCode::Char('r'), _) => {
                        let mut s = state.write();
                        s.log_event("Reload artifacts (stub — use pmkt-analyze to build)".to_string());
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}
