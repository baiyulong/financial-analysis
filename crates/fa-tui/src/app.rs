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
    pub is_add_active: bool,
    pub add_input: String,
    pub should_quit: bool,
    pub screen: AppScreen,
}

/// Infer market and construct Symbol from user-typed ticker input.
/// Rules:
///   - starts with "sh"/"sz" (case-insensitive) → AShare, lowercase prefix preserved
///   - all ASCII digits → AShare, code unchanged
///   - otherwise → USStock, uppercased
fn parse_add_symbol(input: &str) -> Option<fa_core::Symbol> {
    use fa_core::Market;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let (market, code) = if lower.starts_with("sh") || lower.starts_with("sz") {
        (Market::AShare, lower)
    } else if trimmed.chars().all(|c| c.is_ascii_digit()) {
        (Market::AShare, trimmed.to_string())
    } else {
        (Market::USStock, trimmed.to_uppercase())
    };
    Some(fa_core::Symbol::new(code, market))
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
            AppAction::StartAdd => {
                self.is_add_active = true;
                self.add_input.clear();
            }
            AppAction::UpdateAddInput(c) => {
                if self.is_add_active {
                    self.add_input.push(c);
                }
            }
            AppAction::BackspaceAdd => {
                if self.is_add_active {
                    self.add_input.pop();
                }
            }
            AppAction::CancelAdd => {
                self.is_add_active = false;
                self.add_input.clear();
            }
            AppAction::ConfirmAdd => {
                if self.is_add_active {
                    self.is_add_active = false;
                    if let Some(sym) = parse_add_symbol(&self.add_input) {
                        if !self.watchlist.iter().any(|s| {
                            s.market == sym.market && s.yahoo_ticker() == sym.yahoo_ticker()
                        }) {
                            self.watchlist.push(sym);
                        }
                    }
                    self.add_input.clear();
                }
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
                    if zoom_in {
                        if cs.bar_width < 8 { cs.bar_width += 1; }
                    } else if cs.bar_width > 2 {
                        cs.bar_width -= 1;
                    }
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

    /// Returns the symbol currently highlighted in the watchlist.
    /// Chart entry is always from the watchlist; portfolio symbols are not chartable via Enter.
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
    StartAdd,
    UpdateAddInput(char),
    BackspaceAdd,
    CancelAdd,
    ConfirmAdd,
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
    fn test_start_add_activates_mode() {
        let mut s = make_state();
        s.apply(AppAction::StartAdd);
        assert!(s.is_add_active);
        assert!(s.add_input.is_empty());
    }

    #[test]
    fn test_cancel_add_clears_mode() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "AAP".into();
        s.apply(AppAction::CancelAdd);
        assert!(!s.is_add_active);
        assert!(s.add_input.is_empty());
    }

    #[test]
    fn test_confirm_add_us_stock() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "MSFT".into();
        let original_len = s.watchlist.len();
        s.apply(AppAction::ConfirmAdd);
        assert!(!s.is_add_active);
        assert_eq!(s.watchlist.len(), original_len + 1);
        let added = s.watchlist.last().unwrap();
        assert_eq!(added.code, "MSFT");
        assert_eq!(added.market, fa_core::Market::USStock);
    }

    #[test]
    fn test_confirm_add_a_share_sh_prefix() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "sh600519".into();
        s.apply(AppAction::ConfirmAdd);
        let added = s.watchlist.last().unwrap();
        assert_eq!(added.code, "sh600519");
        assert_eq!(added.market, fa_core::Market::AShare);
    }

    #[test]
    fn test_confirm_add_a_share_digits() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "000001".into();
        s.apply(AppAction::ConfirmAdd);
        let added = s.watchlist.last().unwrap();
        assert_eq!(added.code, "000001");
        assert_eq!(added.market, fa_core::Market::AShare);
    }

    #[test]
    fn test_confirm_add_empty_input_does_nothing() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "   ".into();
        let original_len = s.watchlist.len();
        s.apply(AppAction::ConfirmAdd);
        assert!(!s.is_add_active);
        assert_eq!(s.watchlist.len(), original_len);
    }

    #[test]
    fn test_confirm_add_duplicate_does_not_add() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "AAPL".into(); // AAPL already in make_state()
        let original_len = s.watchlist.len();
        s.apply(AppAction::ConfirmAdd);
        assert_eq!(s.watchlist.len(), original_len);
    }

    #[test]
    fn test_update_and_backspace_add_input() {
        let mut s = make_state();
        s.is_add_active = true;
        s.apply(AppAction::UpdateAddInput('A'));
        s.apply(AppAction::UpdateAddInput('A'));
        s.apply(AppAction::UpdateAddInput('P'));
        assert_eq!(s.add_input, "AAP");
        s.apply(AppAction::BackspaceAdd);
        assert_eq!(s.add_input, "AA");
    }

    #[test]
    fn test_confirm_add_ignored_when_mode_inactive() {
        let mut s = make_state();
        s.add_input = "MSFT".into();
        let original_len = s.watchlist.len();
        s.apply(AppAction::ConfirmAdd);
        assert!(!s.is_add_active);
        assert_eq!(s.watchlist.len(), original_len);
        assert_eq!(s.add_input, "MSFT");
    }

    #[test]
    fn test_confirm_add_a_share_duplicate_by_ticker_does_not_add() {
        let mut s = make_state();
        s.is_add_active = true;
        s.add_input = "sh600519".into();
        s.apply(AppAction::ConfirmAdd);

        s.is_add_active = true;
        s.add_input = "600519".into();
        let original_len = s.watchlist.len();
        s.apply(AppAction::ConfirmAdd);

        assert_eq!(s.watchlist.len(), original_len);
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
