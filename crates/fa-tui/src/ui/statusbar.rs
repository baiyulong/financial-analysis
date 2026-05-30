// crates/fa-tui/src/ui/statusbar.rs
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::app::State;

pub fn render(f: &mut Frame, state: &State, area: Rect, refresh_interval: u64) {
    let updated = state.last_updated
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "Never".into());

    let status = state.status_message.as_deref().unwrap_or("");
    let search = if state.is_search_active {
        format!("  Search: {}_", state.search_input)
    } else {
        "".into()
    };

    let line = Line::from(vec![
        Span::styled(
            format!("[Updated: {}] [Refresh: {}s]  {}{}",
                updated, refresh_interval, status, search),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    f.render_widget(Paragraph::new(line), area);
}
