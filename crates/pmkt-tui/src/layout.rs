// Layout helpers — currently layout is handled inline in ui.rs.
// This module can hold reusable layout primitives in future.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Split a rect into N equal columns.
pub fn equal_columns(area: Rect, n: usize) -> Vec<Rect> {
    let constraints: Vec<Constraint> = (0..n).map(|_| Constraint::Ratio(1, n as u32)).collect();
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}
