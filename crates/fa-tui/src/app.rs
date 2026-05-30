use chrono::{DateTime, Utc};
use fa_backtest::{BacktestConfig, BacktestResult, BuiltinStrategy};
use fa_core::{Portfolio, Quote, Symbol, OHLCV, Period};
use fa_data::sina::StockSuggestion;
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
    Backtest(BacktestState),
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
    /// Set by `ChartLoadMoreHistory` so that `ChartDataLoaded` can place
    /// the cursor at the oldest bar (index 0) instead of the newest.
    pub is_load_more: bool,
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
            is_load_more: false,
        }
    }

    pub fn current_bar(&self) -> Option<&OHLCV> {
        self.data.get(self.cursor)
    }
}

#[derive(Debug, Clone)]
pub enum BacktestStatus {
    Idle,
    Running,
    Done,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct BacktestState {
    pub symbol: fa_core::Symbol,
    pub strategy_idx: usize,
    pub config: BacktestConfig,
    pub result: Option<BacktestResult>,
    pub trade_scroll: usize,
    pub status: BacktestStatus,
}

impl BacktestState {
    pub fn new(symbol: fa_core::Symbol) -> Self {
        Self {
            symbol,
            strategy_idx: 0,
            config: BacktestConfig::default(),
            result: None,
            trade_scroll: 0,
            status: BacktestStatus::Idle,
        }
    }

    pub fn strategy_count() -> usize { BuiltinStrategy::all().len() }

    pub fn strategy_name(&self) -> &'static str {
        let all = BuiltinStrategy::all();
        let idx = self.strategy_idx.min(all.len().saturating_sub(1));
        all[idx].name()
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
    pub search_results: Vec<StockSuggestion>,
    pub search_selected: usize,
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
                self.is_add_active = false;
                self.add_input.clear();
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
                self.is_search_active = false;
                self.search_input.clear();
                self.is_add_active = true;
                self.add_input.clear();
                self.search_results.clear();
                self.search_selected = 0;
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
                self.search_results.clear();
                self.search_selected = 0;
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
                    self.search_results.clear();
                    self.search_selected = 0;
                }
            }
            AppAction::SearchResultsUpdated(results) => {
                self.search_results = results;
                self.search_selected = 0;
            }
            AppAction::SearchSelectNext => {
                if !self.search_results.is_empty() {
                    self.search_selected = (self.search_selected + 1) % self.search_results.len();
                }
            }
            AppAction::SearchSelectPrev => {
                if !self.search_results.is_empty() {
                    self.search_selected = self.search_selected
                        .checked_sub(1)
                        .unwrap_or(self.search_results.len() - 1);
                }
            }
            AppAction::ConfirmSearchSelection => {
                if self.is_add_active && !self.search_results.is_empty() {
                    let suggestion = &self.search_results[self.search_selected];
                    let market = fa_core::Market::AShare;
                    let sym = fa_core::Symbol::new(&suggestion.code, market)
                        .with_name(&suggestion.name);
                    if !self.watchlist.iter().any(|s| s.code == sym.code) {
                        self.watchlist.push(sym);
                    }
                    self.is_add_active = false;
                    self.add_input.clear();
                    self.search_results.clear();
                    self.search_selected = 0;
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
                    // Propagate name from quote to the symbol in watchlist
                    if let Some(ref name) = q.name {
                        if !name.is_empty() {
                            if let Some(sym) = self.watchlist.iter_mut().find(|s| s.code == q.symbol.code) {
                                sym.name = Some(name.clone());
                            }
                        }
                    }
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
                    let was_load_more = cs.is_load_more;
                    let len = data.len();
                    cs.data = data;
                    cs.cursor = if was_load_more { 0 } else { len.saturating_sub(1) };
                    cs.loading = false;
                    cs.is_load_more = false;
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
            AppAction::ChartLoadMoreHistory => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    if let Some(longer) = next_longer_period(&cs.period) {
                        cs.period = longer;
                        cs.loading = true;
                        cs.data.clear();
                        cs.cursor = 0;
                        cs.is_load_more = true;
                    }
                }
            }
            AppAction::StartBacktest(sym) => {
                self.screen = AppScreen::Backtest(BacktestState::new(sym));
            }
            AppAction::RunBacktest => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    bs.status = BacktestStatus::Running;
                    bs.result = None;
                }
            }
            AppAction::BacktestComplete(result) => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    bs.result = Some(result);
                    bs.status = BacktestStatus::Done;
                    bs.trade_scroll = 0;
                }
            }
            AppAction::BacktestFailed(msg) => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    bs.status = BacktestStatus::Error(msg);
                }
            }
            AppAction::BacktestNextStrategy => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    let count = BacktestState::strategy_count();
                    bs.strategy_idx = (bs.strategy_idx + 1) % count;
                }
            }
            AppAction::BacktestPrevStrategy => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    let count = BacktestState::strategy_count();
                    bs.strategy_idx = (bs.strategy_idx + count - 1) % count;
                }
            }
            AppAction::BacktestScrollUp => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    bs.trade_scroll = bs.trade_scroll.saturating_sub(1);
                }
            }
            AppAction::BacktestScrollDown => {
                if let AppScreen::Backtest(ref mut bs) = self.screen {
                    if let Some(ref result) = bs.result {
                        let max = result.trades.len().saturating_sub(1);
                        bs.trade_scroll = (bs.trade_scroll + 1).min(max);
                    }
                }
            }
            AppAction::ExitBacktest => {
                self.screen = AppScreen::Main;
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

/// Returns the next longer time period, or None if already at the maximum.
pub fn next_longer_period(p: &Period) -> Option<Period> {
    match p {
        Period::Day1   => Some(Period::Week1),
        Period::Week1  => Some(Period::Month1),
        Period::Month1 => Some(Period::Month3),
        Period::Month3 => Some(Period::Month6),
        Period::Month6 => Some(Period::Year1),
        Period::Year1  => Some(Period::Year5),
        Period::Year5  => None,
    }
}

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
    SearchResultsUpdated(Vec<StockSuggestion>),
    SearchSelectNext,
    SearchSelectPrev,
    ConfirmSearchSelection,
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
    ChartLoadMoreHistory,
    StartBacktest(fa_core::Symbol),
    RunBacktest,
    BacktestComplete(BacktestResult),
    BacktestFailed(String),
    BacktestNextStrategy,
    BacktestPrevStrategy,
    BacktestScrollUp,
    BacktestScrollDown,
    ExitBacktest,
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
            is_load_more: false,
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
            is_load_more: false,
        });
        s.apply(AppAction::ChartChangePeriod(Period::Year1));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Year1);
            assert!(cs.loading);
            assert!(cs.data.is_empty());
        }
    }

    #[test]
    fn test_start_backtest_sets_screen() {
        use fa_core::{Market, Symbol};
        let mut s = make_state();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.apply(AppAction::StartBacktest(sym.clone()));
        assert!(matches!(s.screen, AppScreen::Backtest(_)));
    }

    #[test]
    fn test_exit_backtest_returns_to_main() {
        use fa_core::{Market, Symbol};
        let mut s = make_state();
        s.apply(AppAction::StartBacktest(Symbol::new("AAPL", Market::USStock)));
        s.apply(AppAction::ExitBacktest);
        assert!(matches!(s.screen, AppScreen::Main));
    }

    #[test]
    fn test_backtest_next_prev_strategy_wraps() {
        use fa_core::{Market, Symbol};
        let mut s = make_state();
        s.apply(AppAction::StartBacktest(Symbol::new("AAPL", Market::USStock)));
        if let AppScreen::Backtest(ref bs) = s.screen {
            assert_eq!(bs.strategy_idx, 0);
        }
        s.apply(AppAction::BacktestNextStrategy);
        if let AppScreen::Backtest(ref bs) = s.screen { assert_eq!(bs.strategy_idx, 1); }
        s.apply(AppAction::BacktestNextStrategy);
        if let AppScreen::Backtest(ref bs) = s.screen { assert_eq!(bs.strategy_idx, 2); }
        s.apply(AppAction::BacktestNextStrategy);
        if let AppScreen::Backtest(ref bs) = s.screen { assert_eq!(bs.strategy_idx, 0); }
    }

    #[test]
    fn test_run_backtest_sets_running_status() {
        use fa_core::{Market, Symbol};
        use crate::app::BacktestStatus;
        let mut s = make_state();
        s.apply(AppAction::StartBacktest(Symbol::new("AAPL", Market::USStock)));
        s.apply(AppAction::RunBacktest);
        if let AppScreen::Backtest(ref bs) = s.screen {
            assert!(matches!(bs.status, BacktestStatus::Running));
        }
    }

    #[test]
    fn test_quotes_updated_propagates_name() {
        let mut s = State {
            watchlist: vec![{
                let mut sym = Symbol::new("sh600519", fa_core::Market::AShare);
                sym.name = None;
                sym
            }],
            ..Default::default()
        };

        // Apply QuotesUpdated with a name — should propagate to watchlist symbol
        let q_with_name = fa_core::Quote {
            symbol: Symbol::new("sh600519", fa_core::Market::AShare),
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
            name: Some("贵州茅台".to_string()),
            timestamp: chrono::Utc::now(),
        };
        s.apply(AppAction::QuotesUpdated(vec![q_with_name]));
        assert_eq!(s.watchlist[0].name, Some("贵州茅台".to_string()));

        // Apply QuotesUpdated with name: None — cached name must survive
        let q_no_name = fa_core::Quote {
            symbol: Symbol::new("sh600519", fa_core::Market::AShare),
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
        };
        s.apply(AppAction::QuotesUpdated(vec![q_no_name]));
        assert_eq!(s.watchlist[0].name, Some("贵州茅台".to_string()), "None update must not overwrite cached name");
    }

    fn make_dummy_ohlcv(symbol: Symbol, count: usize) -> Vec<fa_core::OHLCV> {
        use chrono::Utc;
        use rust_decimal_macros::dec;
        (0..count).map(|i| fa_core::OHLCV {
            symbol: symbol.clone(),
            timestamp: Utc::now() - chrono::Duration::days((count - i) as i64),
            open: dec!(100),
            high: dec!(105),
            low: dec!(95),
            close: dec!(102),
            volume: 1000,
        }).collect()
    }

    #[test]
    fn test_next_longer_period_full_chain() {
        use fa_core::Period;
        assert_eq!(next_longer_period(&Period::Day1),   Some(Period::Week1));
        assert_eq!(next_longer_period(&Period::Week1),  Some(Period::Month1));
        assert_eq!(next_longer_period(&Period::Month1), Some(Period::Month3));
        assert_eq!(next_longer_period(&Period::Month3), Some(Period::Month6));
        assert_eq!(next_longer_period(&Period::Month6), Some(Period::Year1));
        assert_eq!(next_longer_period(&Period::Year1),  Some(Period::Year5));
        assert_eq!(next_longer_period(&Period::Year5),  None);
    }

    #[test]
    fn test_chart_load_more_history_upgrades_period_and_resets() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        s.screen = AppScreen::Chart(ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Day1));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.cursor = 0;
            cs.loading = false;
        }
        s.apply(AppAction::ChartLoadMoreHistory);
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Week1);
            assert!(cs.loading);
            assert!(cs.data.is_empty());
            assert_eq!(cs.cursor, 0);
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_load_more_history_noop_at_max_period() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        s.screen = AppScreen::Chart(ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Year5));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false;
        }
        s.apply(AppAction::ChartLoadMoreHistory);
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Year5);
            assert!(!cs.loading);
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_data_loaded_load_more_keeps_cursor_at_start() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.screen = AppScreen::Chart(ChartState::new(sym.clone(), Period::Day1));
        s.apply(AppAction::ChartLoadMoreHistory);
        let dummy_ohlcv = make_dummy_ohlcv(sym.clone(), 50);
        s.apply(AppAction::ChartDataLoaded(dummy_ohlcv));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.cursor, 0, "load-more should leave cursor at oldest bar");
            assert!(!cs.loading);
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_data_loaded_normal_sets_cursor_to_newest() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.screen = AppScreen::Chart(ChartState::new(sym.clone(), Period::Month1));
        let dummy_ohlcv = make_dummy_ohlcv(sym.clone(), 30);
        s.apply(AppAction::ChartDataLoaded(dummy_ohlcv));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.cursor, 29, "normal load should place cursor at newest bar");
        } else {
            panic!("Expected Chart screen");
        }
    }
}
