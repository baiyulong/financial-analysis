use chrono::{DateTime, Utc};
use fa_core::{Portfolio, Quote, Symbol, OHLCV, Period};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    Watchlist,
    Portfolio,
}

impl Default for FocusedPanel {
    fn default() -> Self { FocusedPanel::Watchlist }
}

impl FocusedPanel {
    pub fn next(&self) -> Self {
        match self {
            FocusedPanel::Watchlist  => FocusedPanel::Portfolio,
            FocusedPanel::Portfolio  => FocusedPanel::Watchlist,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppScreen {
    Main,
    Chart(ChartState),
}

impl Default for AppScreen {
    fn default() -> Self { AppScreen::Main }
}

#[derive(Debug, Clone)]
pub struct ChartState {
    pub symbol: Symbol,
    pub period: Period,
    pub data: Vec<OHLCV>,
    pub cursor: usize,
    pub bar_width: u16,
    pub ma_periods: Vec<usize>,
    pub loading: bool,
}

impl ChartState {
    pub fn new(symbol: Symbol, period: Period) -> Self {
        Self {
            symbol,
            period,
            data: vec![],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5, 10, 20],
            loading: true,
        }
    }

    pub fn current_bar(&self) -> Option<&OHLCV> {
        self.data.get(self.cursor)
    }
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub watchlist: Vec<Symbol>,
    pub quotes: HashMap<String, Quote>,   // key: symbol.code
    pub portfolio: Portfolio,
    pub selected_watchlist: usize,
    pub selected_portfolio: usize,
    pub focused_panel: FocusedPanel,
    pub last_updated: Option<DateTime<Utc>>,
    pub status_message: Option<String>,
    pub is_search_active: bool,
    pub search_input: String,
    pub should_quit: bool,
    pub screen: AppScreen,
}

impl State {
    pub fn apply(&mut self, action: AppAction) {
        match action {
            AppAction::Quit => {
                self.should_quit = true;
            }
            AppAction::NextPanel => {
                self.focused_panel = self.focused_panel.next();
            }
            AppAction::MoveUp => {
                let (current, len) = self.focused_selection_mut();
                if len > 0 {
                    *current = if *current == 0 { len - 1 } else { *current - 1 };
                }
            }
            AppAction::MoveDown => {
                let (current, len) = self.focused_selection_mut();
                if len > 0 {
                    *current = (*current + 1) % len;
                }
            }
            AppAction::StartSearch => {
                self.is_search_active = true;
                self.search_input.clear();
            }
            AppAction::UpdateSearchInput(c) => {
                if self.is_search_active {
                    self.search_input.push(c);
                }
            }
            AppAction::BackspaceSearch => {
                if self.is_search_active {
                    self.search_input.pop();
                }
            }
            AppAction::CancelSearch => {
                self.is_search_active = false;
                self.search_input.clear();
            }
            AppAction::DeleteSelected => {
                if self.focused_panel == FocusedPanel::Watchlist
                    && self.selected_watchlist < self.watchlist.len()
                {
                    let removed = self.watchlist.remove(self.selected_watchlist);
                    self.quotes.remove(&removed.code);
                    if self.selected_watchlist > 0 {
                        self.selected_watchlist -= 1;
                    }
                }
            }
            AppAction::QuotesUpdated(quotes) => {
                self.last_updated = Some(Utc::now());
                for q in quotes {
                    self.quotes.insert(q.symbol.code.clone(), q);
                }
            }
            AppAction::StatusMessage(msg) => {
                self.status_message = Some(msg);
            }
            AppAction::Refresh => {
                self.status_message = None;
            }
            AppAction::EnterChart(sym) => {
                self.screen = AppScreen::Chart(ChartState::new(sym, Period::Month1));
            }
            AppAction::ExitChart => {
                self.screen = AppScreen::Main;
            }
            AppAction::ChartDataLoaded(data) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    let len = data.len();
                    cs.data = data;
                    cs.cursor = len.saturating_sub(1);
                    cs.loading = false;
                }
            }
            AppAction::ChartMoveCursor(delta) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    let len = cs.data.len();
                    if len > 0 {
                        cs.cursor = (cs.cursor as i64 + delta as i64)
                            .clamp(0, len as i64 - 1) as usize;
                    }
                }
            }
            AppAction::ChartZoom(zoom_in) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    if zoom_in && cs.bar_width < 8 { cs.bar_width += 1; }
                    else if !zoom_in && cs.bar_width > 2 { cs.bar_width -= 1; }
                }
            }
            AppAction::ChartChangePeriod(period) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    cs.period = period;
                    cs.loading = true;
                    cs.data.clear();
                }
            }
        }
    }

    fn focused_selection_mut(&mut self) -> (&mut usize, usize) {
        match self.focused_panel {
            FocusedPanel::Watchlist => (&mut self.selected_watchlist, self.watchlist.len()),
            FocusedPanel::Portfolio => (&mut self.selected_portfolio, self.portfolio.positions.len()),
        }
    }

    pub fn selected_symbol(&self) -> Option<&Symbol> {
        self.watchlist.get(self.selected_watchlist)
    }
}

pub type AppState = Arc<RwLock<State>>;

#[derive(Debug, Clone)]
pub enum AppAction {
    Quit,
    NextPanel,
    MoveUp,
    MoveDown,
    StartSearch,
    UpdateSearchInput(char),
    BackspaceSearch,
    CancelSearch,
    DeleteSelected,
    Refresh,
    QuotesUpdated(Vec<Quote>),
    StatusMessage(String),
    EnterChart(Symbol),
    ExitChart,
    ChartDataLoaded(Vec<OHLCV>),
    ChartMoveCursor(i32),
    ChartZoom(bool),
    ChartChangePeriod(Period),
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};

    fn make_state() -> State {
        State {
            watchlist: vec![
                Symbol::new("AAPL", Market::USStock),
                Symbol::new("TSLA", Market::USStock),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn test_move_down_wraps() {
        let mut s = make_state();
        s.selected_watchlist = 1;
        s.apply(AppAction::MoveDown);
        assert_eq!(s.selected_watchlist, 0);
    }

    #[test]
    fn test_move_up_wraps() {
        let mut s = make_state();
        s.selected_watchlist = 0;
        s.apply(AppAction::MoveUp);
        assert_eq!(s.selected_watchlist, 1);
    }

    #[test]
    fn test_tab_cycles_panels() {
        let mut s = make_state();
        assert_eq!(s.focused_panel, FocusedPanel::Watchlist);
        s.apply(AppAction::NextPanel);
        assert_eq!(s.focused_panel, FocusedPanel::Portfolio);
        s.apply(AppAction::NextPanel);
        assert_eq!(s.focused_panel, FocusedPanel::Watchlist);
    }

    #[test]
    fn test_start_search() {
        let mut s = make_state();
        s.apply(AppAction::StartSearch);
        assert!(s.is_search_active);
    }

    #[test]
    fn test_cancel_search() {
        let mut s = make_state();
        s.is_search_active = true;
        s.search_input = "AAPL".into();
        s.apply(AppAction::CancelSearch);
        assert!(!s.is_search_active);
        assert!(s.search_input.is_empty());
    }

    #[test]
    fn test_delete_from_watchlist() {
        let mut s = make_state();
        s.selected_watchlist = 0;
        s.apply(AppAction::DeleteSelected);
        assert_eq!(s.watchlist.len(), 1);
        assert_eq!(s.watchlist[0].code, "TSLA");
    }

    #[test]
    fn test_quit_sets_flag() {
        let mut s = make_state();
        s.apply(AppAction::Quit);
        assert!(s.should_quit);
    }

    #[test]
    fn test_delete_cleans_up_quotes() {
        let mut s = make_state();
        s.quotes.insert("AAPL".into(), fa_core::Quote {
            symbol: fa_core::Symbol::new("AAPL", fa_core::Market::USStock),
            price: rust_decimal::Decimal::ZERO,
            change: rust_decimal::Decimal::ZERO,
            change_pct: rust_decimal::Decimal::ZERO,
            open: None,
            high: None,
            low: None,
            volume: None,
            market_cap: None,
            pe_ratio: None,
            week_52_high: None,
            week_52_low: None,
            name: None,
            timestamp: chrono::Utc::now(),
        });
        s.selected_watchlist = 0;
        s.apply(AppAction::DeleteSelected);
        assert_eq!(s.watchlist.len(), 1);
        assert!(!s.quotes.contains_key("AAPL"));
    }

    #[test]
    fn test_enter_chart_sets_screen() {
        let mut s = make_state();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.apply(AppAction::EnterChart(sym));
        assert!(matches!(s.screen, AppScreen::Chart(_)));
    }

    #[test]
    fn test_exit_chart_returns_to_main() {
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        s.apply(AppAction::ExitChart);
        assert!(matches!(s.screen, AppScreen::Main));
    }

    #[test]
    fn test_chart_data_loaded_sets_cursor_to_last() {
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1));
        let bars = vec![
            OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                    open: dec!(100), high: dec!(110), low: dec!(90), close: dec!(105), volume: 1000 },
            OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                    open: dec!(105), high: dec!(115), low: dec!(95), close: dec!(110), volume: 2000 },
        ];
        s.apply(AppAction::ChartDataLoaded(bars));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.cursor, 1);
            assert!(!cs.loading);
            assert_eq!(cs.data.len(), 2);
        } else {
            panic!("expected Chart screen");
        }
    }

    #[test]
    fn test_chart_cursor_clamps_at_boundaries() {
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        let bar = OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                          open: dec!(100), high: dec!(110), low: dec!(90), close: dec!(105), volume: 0 };
        s.screen = AppScreen::Chart(ChartState {
            symbol: Symbol::new("AAPL", Market::USStock),
            period: Period::Month1,
            data: vec![bar.clone(), bar],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5],
            loading: false,
        });
        s.apply(AppAction::ChartMoveCursor(-1));
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.cursor, 0); }
        s.apply(AppAction::ChartMoveCursor(1));
        s.apply(AppAction::ChartMoveCursor(1));
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.cursor, 1); }
    }

    #[test]
    fn test_chart_zoom_clamps() {
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1));
        s.apply(AppAction::ChartZoom(false));
        s.apply(AppAction::ChartZoom(false));
        s.apply(AppAction::ChartZoom(false));
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.bar_width, 2); }
        for _ in 0..10 { s.apply(AppAction::ChartZoom(true)); }
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.bar_width, 8); }
    }

    #[test]
    fn test_chart_change_period_resets_data() {
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        let bar = OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                          open: dec!(100), high: dec!(110), low: dec!(90), close: dec!(105), volume: 0 };
        s.screen = AppScreen::Chart(ChartState {
            symbol: Symbol::new("AAPL", Market::USStock),
            period: Period::Month1,
            data: vec![bar],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5],
            loading: false,
        });
        s.apply(AppAction::ChartChangePeriod(Period::Year1));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Year1);
            assert!(cs.loading);
            assert!(cs.data.is_empty());
        }
    }
}
