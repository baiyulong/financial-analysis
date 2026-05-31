pub mod backtest;
pub mod chart;
pub mod detail;
pub mod layout;
pub mod portfolio;
pub mod settings;
pub mod watchlist;

use crate::app::{AppScreen, DataSourceKind, State};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub use settings::draw_settings;

pub(crate) fn data_source_label(data_source: &DataSourceKind, s: &'static crate::i18n::Strings) -> &'static str {
    match data_source {
        DataSourceKind::Sina => s.data_source_sina,
        DataSourceKind::AkShare => "AkShare",
    }
}

fn render_main_statusbar(f: &mut Frame, state: &State, area: Rect, refresh_interval: u64) {
    let s = state.strings();
    let updated = state
        .last_updated
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| s.never.to_string());
    let status = state.status_message.as_deref().unwrap_or("");
    let search = if state.is_search_active {
        format!("  {}{}_", s.search_prompt, state.search_input)
    } else {
        String::new()
    };

    let line = Line::from(vec![Span::styled(
        format!(
            "{}: {} | [{}]: {} | [{}]: {}s  {}{}",
            s.data_source_label,
            data_source_label(&state.data_source, s),
            s.updated_label,
            updated,
            s.refresh_label,
            refresh_interval,
            status,
            search
        ),
        Style::default().fg(Color::DarkGray),
    )]);

    f.render_widget(Paragraph::new(line), area);
}

pub fn draw(f: &mut Frame, state: &State, refresh_interval: u64) {
    match &state.screen {
        AppScreen::Main => {
            let areas = layout::compute(f.area());
            watchlist::render(f, state, areas.watchlist);
            portfolio::render(f, state, areas.portfolio);
            detail::render(f, state, areas.detail);
            render_main_statusbar(f, state, areas.statusbar, refresh_interval);
        }
        AppScreen::Chart(cs) => {
            chart::render(f, cs, &state.data_source, state.strings(), f.area());
        }
        AppScreen::Backtest(bs) => {
            backtest::render(f, bs, state.strings(), f.area());
        }
        AppScreen::Settings(ss) => {
            draw_settings(f, f.area(), ss);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ChartState, DataSourceKind};
    use fa_core::{Market, Period, Symbol};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_draw_main_shows_data_source() {
        let backend = TestBackend::new(160, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = State::default();
        state.data_source = DataSourceKind::AkShare;

        terminal.draw(|f| draw(f, &state, 5)).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!("数据源:".chars().all(|c| content.contains(c)));
        assert!(content.contains("AkShare"));
    }

    #[test]
    fn test_draw_chart_shows_data_source_and_minute_help() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = State::default();
        state.data_source = DataSourceKind::AkShare;
        state.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));

        terminal.draw(|f| draw(f, &state, 5)).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!("数据源:".chars().all(|c| content.contains(c)));
        assert!(content.contains("AkShare"));
        assert!(content.contains("F1-F5"));
    }
}
