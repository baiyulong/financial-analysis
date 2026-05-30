// crates/fa-tui/src/ui/layout.rs
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub watchlist: Rect,
    pub portfolio: Rect,
    pub detail: Rect,
    pub statusbar: Rect,
}

pub fn compute(area: Rect) -> AppLayout {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let main_area = vertical[0];
    let statusbar = vertical[1];

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(main_area);

    let left = horizontal[0];
    let right = horizontal[1];

    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(left);

    AppLayout {
        watchlist: left_panels[0],
        portfolio: left_panels[1],
        detail: right,
        statusbar,
    }
}
