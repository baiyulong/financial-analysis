pub mod backtest;
pub mod chart;
pub mod detail;
pub mod layout;
pub mod portfolio;
pub mod settings;
pub mod statusbar;
pub mod watchlist;

use crate::app::{AppScreen, State};
use ratatui::Frame;

pub use settings::draw_settings;

pub fn draw(f: &mut Frame, state: &State, refresh_interval: u64) {
    match &state.screen {
        AppScreen::Main => {
            let areas = layout::compute(f.area());
            watchlist::render(f, state, areas.watchlist);
            portfolio::render(f, state, areas.portfolio);
            detail::render(f, state, areas.detail);
            statusbar::render(f, state, areas.statusbar, refresh_interval);
        }
        AppScreen::Chart(cs) => {
            chart::render(f, cs, f.area());
        }
        AppScreen::Backtest(bs) => {
            backtest::render(f, bs, f.area());
        }
        AppScreen::Settings(ss) => {
            draw_settings(f, f.area(), ss);
        }
    }
}
