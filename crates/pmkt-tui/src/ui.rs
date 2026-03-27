use pmkt_app::SharedAppState;
use pmkt_domain::TradingMode;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs, Wrap},
    Frame,
};

use crate::app::ActiveTab;

/// Main draw function dispatching to layout panels.
pub fn draw_ui(f: &mut Frame, state: &SharedAppState, tab: ActiveTab) {
    let s = state.read();

    // Outer layout: tabs at bottom, main content above
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    // Draw tab bar
    draw_tabs(f, outer[1], tab);

    match tab {
        ActiveTab::Main => draw_main(f, outer[0], &s),
        ActiveTab::BucketDiagnostics => draw_bucket_diagnostics(f, outer[0], &s),
        ActiveTab::SignalExplanation => draw_signal_explanation(f, outer[0], &s),
        ActiveTab::OrderLog => draw_order_log(f, outer[0], &s),
        ActiveTab::SystemHealth => draw_system_health(f, outer[0], &s),
        ActiveTab::RiskAlerts => draw_risk_alerts(f, outer[0], &s),
    }
}

fn draw_tabs(f: &mut Frame, area: Rect, active: ActiveTab) {
    let tabs = [
        ActiveTab::Main,
        ActiveTab::BucketDiagnostics,
        ActiveTab::SignalExplanation,
        ActiveTab::OrderLog,
        ActiveTab::SystemHealth,
        ActiveTab::RiskAlerts,
    ];
    let tab_titles: Vec<Line> = tabs
        .iter()
        .map(|t| Line::from(t.label()))
        .collect();
    let selected = tabs.iter().position(|&t| t == active).unwrap_or(0);

    let hint = Span::styled(
        " [q]Quit [Tab]Switch [m]Mode [a]Arm [k]Kill [r]Reload",
        Style::default().fg(Color::DarkGray),
    );

    let tab_widget = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title(hint))
        .select(selected)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_widget(tab_widget, area);
}

fn draw_main(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    // Three columns: Recent Markets | Current Market | Order Book + Controls
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    draw_recent_markets(f, cols[0], s);
    draw_current_market(f, cols[1], s);
    draw_orderbook_and_controls(f, cols[2], s);
}

fn draw_recent_markets(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let header = Row::new(vec![
        Cell::from("Time").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Slug"),
        Cell::from("Out"),
        Cell::from("Mode"),
        Cell::from("P&L"),
        Cell::from("Bucket"),
    ])
    .style(Style::default().fg(Color::Cyan));

    let rows: Vec<Row> = s
        .recent_results
        .iter()
        .take(20)
        .map(|r| {
            let pnl_color = if r.net_pnl >= 0.0 { Color::Green } else { Color::Red };
            let mode_color = mode_color(r.mode_used);
            Row::new(vec![
                Cell::from(r.end_time.format("%H:%M").to_string()),
                Cell::from(truncate(&r.slug, 12)),
                Cell::from(r.outcome.to_string()),
                Cell::from(r.participated.to_string()).style(Style::default().fg(mode_color)),
                Cell::from(format!("{:+.2}", r.net_pnl))
                    .style(Style::default().fg(pnl_color)),
                Cell::from(
                    r.bucket_key
                        .as_deref()
                        .unwrap_or(r.skip_reason.as_deref().unwrap_or("-"))
                        .chars()
                        .take(14)
                        .collect::<String>(),
                ),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(13),
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(7),
        Constraint::Min(1),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recent Markets"),
        )
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(table, area);
}

fn draw_current_market(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Market info panel
    let mut lines: Vec<Line> = Vec::new();

    if let Some(mkt) = &s.current_market {
        lines.push(Line::from(vec![
            Span::styled("Market : ", Style::default().fg(Color::Gray)),
            Span::styled(&mkt.slug, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Title  : ", Style::default().fg(Color::Gray)),
            Span::raw(truncate(&mkt.title, 40)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Ends   : ", Style::default().fg(Color::Gray)),
            Span::raw(mkt.end_time.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
        ]));
        let remaining = mkt.seconds_remaining();
        let time_color = if remaining < 60 { Color::Red } else if remaining < 120 { Color::Yellow } else { Color::Green };
        lines.push(Line::from(vec![
            Span::styled("Remain : ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{}s", remaining), Style::default().fg(time_color).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "No active market",
            Style::default().fg(Color::DarkGray),
        )));
    }

    if let Some(snap) = &s.current_snapshot {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("UP    : ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.3}", snap.up_price),
                Style::default().fg(Color::Green),
            ),
            Span::raw("  "),
            Span::styled("DOWN  : ", Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:.3}", snap.down_price),
                Style::default().fg(Color::Red),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Vol30s: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{:.4}", snap.vol_30s)),
        ]));
    }

    // Signal info
    if let Some(sig) = &s.current_signal {
        lines.push(Line::from(""));
        let (signal_text, signal_color) = if sig.decision.is_trade() {
            (sig.decision.to_string(), Color::Green)
        } else {
            (sig.decision.to_string(), Color::Yellow)
        };
        lines.push(Line::from(vec![
            Span::styled("Signal : ", Style::default().fg(Color::Gray)),
            Span::styled(signal_text, Style::default().fg(signal_color).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Bucket : ", Style::default().fg(Color::Gray)),
            Span::raw(
                sig.explanation
                    .bucket_key
                    .as_ref()
                    .map(|k| k.to_key_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Class  : ", Style::default().fg(Color::Gray)),
            Span::styled(
                sig.explanation.classification.to_string(),
                classification_color(sig.explanation.classification),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Edge   : ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{:.4}", sig.explanation.edge)),
            Span::raw("  "),
            Span::styled("Prob   : ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{:.3}", sig.explanation.conservative_prob)),
        ]));
    }

    let market_block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Current Market"))
        .wrap(Wrap { trim: true });
    f.render_widget(market_block, rows[0]);

    // Bot state + P&L panel
    let mut state_lines: Vec<Line> = Vec::new();
    state_lines.push(Line::from(vec![
        Span::styled("Bot State: ", Style::default().fg(Color::Gray)),
        Span::styled(
            s.bot_state.to_string(),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ]));
    state_lines.push(Line::from(vec![
        Span::styled("Session P&L: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:+.2}", s.pnl.net_pnl),
            if s.pnl.net_pnl >= 0.0 { Style::default().fg(Color::Green) } else { Style::default().fg(Color::Red) },
        ),
    ]));
    if let Some(pos) = &s.current_position {
        state_lines.push(Line::from(vec![
            Span::styled("Position : ", Style::default().fg(Color::Gray)),
            Span::raw(format!(
                "{} {:.2}@{:.3} uPnL={:+.2}",
                pos.side, pos.size, pos.avg_entry_price, pos.unrealized_pnl
            )),
        ]));
    }

    let state_block = Paragraph::new(state_lines)
        .block(Block::default().borders(Borders::ALL).title("Bot State / P&L"))
        .wrap(Wrap { trim: true });
    f.render_widget(state_block, rows[1]);
}

fn draw_orderbook_and_controls(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Order book
    let mut ob_lines: Vec<Line> = Vec::new();
    if let Some(ob) = &s.current_orderbook_up {
        ob_lines.push(Line::from(Span::styled(
            "=== UP Token ===",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )));
        if let (Some(bid), Some(ask)) = (ob.best_bid(), ob.best_ask()) {
            ob_lines.push(Line::from(format!("Best Bid: {:.3}  Best Ask: {:.3}", bid, ask)));
            if let Some(spread) = ob.spread() {
                ob_lines.push(Line::from(format!("Spread  : {:.4}", spread)));
            }
        }
        ob_lines.push(Line::from(""));
        ob_lines.push(Line::from(Span::styled(
            " Asks (sell) ",
            Style::default().fg(Color::Red),
        )));
        for level in ob.asks.iter().rev().take(5) {
            ob_lines.push(Line::from(format!("  {:.3}  {:.1}", level.price, level.size)));
        }
        ob_lines.push(Line::from(Span::styled(
            "──── spread ────",
            Style::default().fg(Color::DarkGray),
        )));
        for level in ob.bids.iter().take(5) {
            ob_lines.push(Line::from(format!("  {:.3}  {:.1}", level.price, level.size)));
        }
        ob_lines.push(Line::from(Span::styled(
            " Bids (buy) ",
            Style::default().fg(Color::Green),
        )));
    } else {
        ob_lines.push(Line::from(Span::styled(
            "No order book data",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let ob_block = Paragraph::new(ob_lines)
        .block(Block::default().borders(Borders::ALL).title("Order Book"))
        .wrap(Wrap { trim: false });
    f.render_widget(ob_block, rows[0]);

    // Controls panel
    let mode_color = trading_mode_color(s.mode);
    let mode_style = Style::default()
        .fg(mode_color)
        .add_modifier(Modifier::BOLD);

    let mut ctrl_lines: Vec<Line> = Vec::new();
    ctrl_lines.push(Line::from(vec![
        Span::styled("Mode    : ", Style::default().fg(Color::Gray)),
        Span::styled(s.mode.to_string(), mode_style),
    ]));

    if s.risk_state.kill_switch_triggered {
        ctrl_lines.push(Line::from(Span::styled(
            "⚠️  KILL SWITCH ACTIVE",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    // Risk flags
    if !s.risk_state.risk_flags.is_empty() {
        ctrl_lines.push(Line::from(""));
        ctrl_lines.push(Line::from(Span::styled("Risk Flags:", Style::default().fg(Color::Yellow))));
        for flag in &s.risk_state.risk_flags {
            ctrl_lines.push(Line::from(format!("  ⚠ {}", flag)));
        }
    }

    ctrl_lines.push(Line::from(""));
    if let Some(action) = &s.last_action {
        ctrl_lines.push(Line::from(vec![
            Span::styled("Last: ", Style::default().fg(Color::Gray)),
            Span::raw(truncate(action, 30)),
        ]));
    }
    if let Some(err) = &s.last_error {
        ctrl_lines.push(Line::from(vec![
            Span::styled("Err : ", Style::default().fg(Color::Red)),
            Span::styled(truncate(err, 30), Style::default().fg(Color::Red)),
        ]));
    }

    let ctrl_block = Paragraph::new(ctrl_lines)
        .block(Block::default().borders(Borders::ALL).title("Controls"))
        .wrap(Wrap { trim: true });
    f.render_widget(ctrl_block, rows[1]);
}

fn draw_bucket_diagnostics(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(sig) = &s.current_signal {
        let exp = &sig.explanation;

        lines.push(Line::from(Span::styled(
            "=== Bucket Diagnostics ===",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Bucket Key   : ", Style::default().fg(Color::Gray)),
            Span::styled(
                exp.bucket_key.as_ref().map(|k| k.to_key_string()).unwrap_or_else(|| "N/A".to_string()),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Classification: ", Style::default().fg(Color::Gray)),
            Span::styled(
                exp.classification.to_string(),
                classification_color(exp.classification),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Reason       : ", Style::default().fg(Color::Gray)),
            Span::raw(&exp.classification_reason),
        ]));

        lines.push(Line::from(""));

        lines.push(Line::from(vec![
            Span::styled("Price Zone   : ", Style::default().fg(Color::Gray)),
            Span::raw(
                exp.price_zone.as_ref().map(|z| z.to_string()).unwrap_or_else(|| "N/A".to_string()),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Vol Regime   : ", Style::default().fg(Color::Gray)),
            Span::raw(
                exp.vol_regime.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
            ),
        ]));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Probabilities:", Style::default().fg(Color::Yellow))));
        lines.push(Line::from(format!("  Raw         : {:.4}", exp.raw_prob)));
        lines.push(Line::from(format!("  Conservative: {:.4}", exp.conservative_prob)));
        lines.push(Line::from(format!("  Market Price: (see snapshot)")));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Signal Economics:", Style::default().fg(Color::Yellow))));
        let edge_color = if exp.edge > 0.0 { Color::Green } else { Color::Red };
        lines.push(Line::from(vec![
            Span::styled("  Edge        : ", Style::default().fg(Color::Gray)),
            Span::styled(format!("{:+.4}", exp.edge), Style::default().fg(edge_color)),
        ]));
        lines.push(Line::from(format!("  Cost Buffer : {:.4}", exp.cost_buffer)));
        lines.push(Line::from(format!("  Samples     : {}", exp.samples)));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Safety Checks:", Style::default().fg(Color::Yellow))));
        for (check, passed) in &exp.safety_checks {
            let (icon, color) = if *passed { ("✓", Color::Green) } else { ("✗", Color::Red) };
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                Span::raw(check),
            ]));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No signal generated yet",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Bucket Diagnostics"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn draw_signal_explanation(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(sig) = &s.current_signal {
        lines.push(Line::from(Span::styled(
            "=== Signal Explanation ===",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        let (decision_text, decision_color) = if sig.decision.is_trade() {
            (format!("🟢 {}", sig.decision), Color::Green)
        } else {
            (format!("🔴 {}", sig.decision), Color::Red)
        };

        lines.push(Line::from(Span::styled(
            decision_text,
            Style::default().fg(decision_color).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(format!("Generated    : {}", sig.generated_at.format("%Y-%m-%d %H:%M:%S UTC"))));
        lines.push(Line::from(format!("Artifact     : {}", sig.artifact_version.as_deref().unwrap_or("N/A"))));
        lines.push(Line::from(format!("Market       : {}", sig.market_id)));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Why this decision?",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(format!("  Classification: {}", sig.explanation.classification)));
        lines.push(Line::from(format!("  Reason: {}", sig.explanation.classification_reason)));
    } else {
        lines.push(Line::from(Span::styled(
            "No signal available",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Signal Explanation"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn draw_order_log(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let lines: Vec<Line> = s
        .event_log
        .iter()
        .rev()
        .take(50)
        .map(|l| Line::from(l.as_str()))
        .collect();

    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Event / Order Log"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn draw_system_health(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let check = |ok: bool| -> Span {
        if ok {
            Span::styled("✓ OK", Style::default().fg(Color::Green))
        } else {
            Span::styled("✗ FAIL", Style::default().fg(Color::Red))
        }
    };

    let lines = vec![
        Line::from(Span::styled("=== System Health ===", Style::default().fg(Color::Cyan))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Connectivity   : ", Style::default().fg(Color::Gray)),
            check(s.risk_state.connectivity_healthy),
        ]),
        Line::from(vec![
            Span::styled("Artifact Valid : ", Style::default().fg(Color::Gray)),
            check(s.risk_state.artifact_valid),
        ]),
        Line::from(vec![
            Span::styled("Reconciliation : ", Style::default().fg(Color::Gray)),
            check(s.risk_state.reconciliation_complete),
        ]),
        Line::from(vec![
            Span::styled("Kill Switch    : ", Style::default().fg(Color::Gray)),
            if s.risk_state.kill_switch_triggered {
                Span::styled("TRIGGERED ⚠️", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            } else {
                Span::styled("OFF", Style::default().fg(Color::Green))
            },
        ]),
        Line::from(vec![
            Span::styled("Session Loss   : ", Style::default().fg(Color::Gray)),
            check(!s.risk_state.session_loss_limit_breached),
        ]),
        Line::from(vec![
            Span::styled("Daily Loss     : ", Style::default().fg(Color::Gray)),
            check(!s.risk_state.daily_loss_limit_breached),
        ]),
        Line::from(vec![
            Span::styled("Market Data    : ", Style::default().fg(Color::Gray)),
            check(!s.risk_state.stale_market_data),
        ]),
        Line::from(vec![
            Span::styled("Order Book     : ", Style::default().fg(Color::Gray)),
            check(!s.risk_state.stale_order_book),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Bot State      : ", Style::default().fg(Color::Gray)),
            Span::styled(s.bot_state.to_string(), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Trading Mode   : ", Style::default().fg(Color::Gray)),
            Span::styled(s.mode.to_string(), Style::default().fg(trading_mode_color(s.mode))),
        ]),
    ];

    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("System Health"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

fn draw_risk_alerts(f: &mut Frame, area: Rect, s: &pmkt_app::AppState) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "=== Risk Alerts ===",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    if s.risk_state.risk_flags.is_empty() {
        lines.push(Line::from(Span::styled(
            "No active risk alerts",
            Style::default().fg(Color::Green),
        )));
    } else {
        for flag in &s.risk_state.risk_flags {
            lines.push(Line::from(vec![
                Span::styled("⚠  ", Style::default().fg(Color::Red)),
                Span::styled(flag, Style::default().fg(Color::Yellow)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Limits:", Style::default().fg(Color::Gray))));
    lines.push(Line::from(format!("  Max session loss  : {:.2}", s.risk_limits.max_session_loss)));
    lines.push(Line::from(format!("  Max daily loss    : {:.2}", s.risk_limits.max_daily_loss)));
    lines.push(Line::from(format!("  Max notional/trade: {:.2}", s.risk_limits.max_notional_per_trade)));
    lines.push(Line::from(format!("  Max open orders   : {}", s.risk_limits.max_open_orders)));
    lines.push(Line::from(format!("  Max positions     : {}", s.risk_limits.max_active_positions)));
    lines.push(Line::from(format!("  No-entry within   : {}s", s.risk_limits.no_entry_within_secs)));

    let block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Risk Alerts"))
        .wrap(Wrap { trim: true });
    f.render_widget(block, area);
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

fn trading_mode_color(mode: TradingMode) -> Color {
    match mode {
        TradingMode::Off => Color::DarkGray,
        TradingMode::Dry => Color::Cyan,
        TradingMode::LiveDisarmed => Color::Yellow,
        TradingMode::LiveArmed => Color::Magenta,
        TradingMode::LiveTrading => Color::Red,
    }
}

fn mode_color(mode: TradingMode) -> Color {
    trading_mode_color(mode)
}

fn classification_color(c: pmkt_domain::StrategyClassification) -> Style {
    use pmkt_domain::StrategyClassification;
    let color = match c {
        StrategyClassification::Whitelisted => Color::Green,
        StrategyClassification::Blacklisted => Color::Red,
        StrategyClassification::Neutral => Color::Yellow,
        StrategyClassification::Unknown => Color::DarkGray,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
