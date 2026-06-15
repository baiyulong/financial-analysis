use chrono::{DateTime, Utc};
use fa_backtest::{BacktestConfig, BacktestResult, BuiltinStrategy};
use fa_core::{Period, Portfolio, Quote, Symbol, OHLCV};
use fa_data::sina::StockSuggestion;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Which OHLCV data provider to use (real-time quotes always use Sina).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DataSourceKind {
    #[default]
    Sina,
    AkShare,
    Zhitu,
}

impl DataSourceKind {
    pub fn label(&self) -> &'static str {
        match self {
            DataSourceKind::Sina => "新浪 (Sina)",
            DataSourceKind::AkShare => "AkShare (AKTools HTTP)",
            DataSourceKind::Zhitu => "ZhituAPI",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SettingsState {
    pub provider: DataSourceKind,
    /// AKTools server URL, e.g. "http://127.0.0.1:8080"
    pub akshare_url: String,
    pub language: crate::i18n::Language,
    /// Which field the cursor is on: 0 = provider, 1 = URL, 2 = language
    pub focused_field: usize,
    /// Whether the URL text field is being edited
    pub editing_url: bool,
}

impl SettingsState {
    pub fn new(provider: DataSourceKind, akshare_url: String, language: crate::i18n::Language) -> Self {
        Self {
            provider,
            akshare_url,
            language,
            focused_field: 0,
            editing_url: false,
        }
    }

    pub fn field_count() -> usize {
        3
    }

    pub fn move_up(&mut self) {
        if self.focused_field > 0 {
            self.focused_field -= 1;
            self.editing_url = false;
        }
    }

    pub fn move_down(&mut self) {
        if self.focused_field + 1 < Self::field_count() {
            self.focused_field += 1;
            self.editing_url = false;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    Watchlist,
    Portfolio,
}

impl Default for FocusedPanel {
    fn default() -> Self {
        FocusedPanel::Watchlist
    }
}

impl FocusedPanel {
    pub fn next(&self) -> Self {
        match self {
            FocusedPanel::Watchlist => FocusedPanel::Portfolio,
            FocusedPanel::Portfolio => FocusedPanel::Watchlist,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AppScreen {
    Main,
    Chart(ChartState),
    Backtest(BacktestState),
    Settings(SettingsState),
}

impl Default for AppScreen {
    fn default() -> Self {
        AppScreen::Main
    }
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
    /// Error message from the most recent OHLCV fetch attempt.
    /// Shown as a centered overlay in the chart panel until new data arrives
    /// or the user navigates away.
    pub error: Option<String>,
    /// Set by `ChartLoadMoreHistory` so that `ChartDataLoaded` places
    /// the cursor at the oldest bar (index 0) instead of the newest.
    pub is_load_more: bool,
    /// True after the first "load more history" fetch; the next fetch uses
    /// `extended_yahoo_range()` instead of `yahoo_range()`.
    pub history_extended: bool,
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
            error: None,
            is_load_more: false,
            history_extended: false,
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

    pub fn strategy_count() -> usize {
        BuiltinStrategy::all().len()
    }

    pub fn strategy_name(&self) -> &'static str {
        let all = BuiltinStrategy::all();
        let idx = self.strategy_idx.min(all.len().saturating_sub(1));
        all[idx].name()
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub watchlist: Vec<Symbol>,
    pub quotes: HashMap<String, Quote>, // key: symbol.code
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
    pub search_generation: u64,
    pub should_quit: bool,
    pub screen: AppScreen,
    pub data_source: DataSourceKind,
    pub akshare_url: String,
    pub language: crate::i18n::Language,
    pub confirm_quit: bool,
    /// Which button in the quit-confirm dialog has focus.
    /// `false` = Confirm/Yes button, `true` = Cancel/No button.
    pub confirm_quit_focused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            watchlist: vec![],
            quotes: HashMap::new(),
            portfolio: Portfolio::default(),
            selected_watchlist: 0,
            selected_portfolio: 0,
            focused_panel: FocusedPanel::default(),
            last_updated: None,
            status_message: None,
            is_search_active: false,
            search_input: String::new(),
            is_add_active: false,
            add_input: String::new(),
            search_results: vec![],
            search_selected: 0,
            search_generation: 0,
            should_quit: false,
            screen: AppScreen::Main,
            data_source: DataSourceKind::Zhitu,
            akshare_url: "http://127.0.0.1:8080".to_string(),
            language: crate::i18n::Language::default(),
            confirm_quit: false,
            confirm_quit_focused: false,
        }
    }
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
                self.confirm_quit = false;
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
                    self.search_generation = self.search_generation.wrapping_add(1);
                }
            }
            AppAction::BackspaceAdd => {
                if self.is_add_active {
                    self.add_input.pop();
                    self.search_generation = self.search_generation.wrapping_add(1);
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
            AppAction::SearchResultsUpdated(gen, results) => {
                if gen == self.search_generation {
                    self.search_results = results;
                    self.search_selected = 0;
                }
            }
            AppAction::SearchSelectNext => {
                if !self.search_results.is_empty() {
                    self.search_selected = (self.search_selected + 1) % self.search_results.len();
                }
            }
            AppAction::SearchSelectPrev => {
                if !self.search_results.is_empty() {
                    self.search_selected = self
                        .search_selected
                        .checked_sub(1)
                        .unwrap_or(self.search_results.len() - 1);
                }
            }
            AppAction::ConfirmSearchSelection => {
                if self.is_add_active && !self.search_results.is_empty() {
                    let suggestion = &self.search_results[self.search_selected];
                    let market = fa_core::Market::AShare;
                    let sym =
                        fa_core::Symbol::new(&suggestion.code, market).with_name(&suggestion.name);
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
                            if let Some(sym) =
                                self.watchlist.iter_mut().find(|s| s.code == q.symbol.code)
                            {
                                sym.name = Some(name.clone());
                            }
                        }
                    }
                    self.quotes.insert(q.symbol.code.clone(), q);
                }
            }
            AppAction::NamesResolved(pairs) => {
                for (code, name) in pairs {
                    if name.is_empty() {
                        continue;
                    }
                    if let Some(sym) = self.watchlist.iter_mut().find(|s| s.code == code) {
                        if sym.name.is_none() {
                            sym.name = Some(name);
                        }
                    }
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
                    cs.cursor = if was_load_more {
                        0
                    } else {
                        len.saturating_sub(1)
                    };
                    cs.loading = false;
                    cs.is_load_more = false;
                    cs.error = None;
                }
            }
            AppAction::ChartFetchFailed(msg) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    cs.loading = false;
                    cs.error = Some(msg);
                }
            }
            AppAction::ChartMoveCursor(delta) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    let len = cs.data.len();
                    if len > 0 {
                        cs.cursor =
                            (cs.cursor as i64 + delta as i64).clamp(0, len as i64 - 1) as usize;
                    }
                }
            }
            AppAction::ChartZoom(zoom_in) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    if zoom_in {
                        if cs.bar_width < 8 {
                            cs.bar_width += 1;
                        }
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
                    cs.error = None;
                    cs.history_extended = false;
                }
            }
            AppAction::ChartLoadMoreHistory => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    // Only extend if the period supports it and we haven't already extended.
                    if cs.period.can_extend_history() && !cs.history_extended {
                        cs.history_extended = true;
                        cs.loading = true;
                        cs.data.clear();
                        cs.error = None;
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
            AppAction::OpenSettings => {
                let ss = SettingsState::new(self.data_source.clone(), self.akshare_url.clone(), self.language);
                self.screen = AppScreen::Settings(ss);
            }
            AppAction::ExitSettings => {
                if matches!(self.screen, AppScreen::Settings(_)) {
                    self.screen = AppScreen::Main;
                }
            }
            AppAction::SettingsNavUp => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    ss.move_up();
                }
            }
            AppAction::SettingsNavDown => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    ss.move_down();
                }
            }
            AppAction::SettingsSelectProvider(kind) => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    ss.provider = kind;
                }
            }
            AppAction::SettingsEditUrlChar(c) => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    if ss.editing_url {
                        ss.akshare_url.push(c);
                    }
                }
            }
            AppAction::SettingsEditUrlBackspace => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    if ss.editing_url {
                        ss.akshare_url.pop();
                    }
                }
            }
            AppAction::SettingsToggleUrlEdit => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    if ss.focused_field == 1 {
                        ss.editing_url = !ss.editing_url;
                    }
                }
            }
            AppAction::SettingsSaved => {
                if let AppScreen::Settings(ref ss) = self.screen {
                    self.data_source = ss.provider.clone();
                    self.akshare_url = ss.akshare_url.clone();
                    self.language = ss.language;
                    self.screen = AppScreen::Main;
                }
            }
            AppAction::RequestQuit => {
                self.confirm_quit = true;
                self.confirm_quit_focused = false;
            }
            AppAction::CancelQuit => {
                self.confirm_quit = false;
                self.confirm_quit_focused = false;
            }
            AppAction::ToggleQuitButton => {
                self.confirm_quit_focused = !self.confirm_quit_focused;
            }
            AppAction::SettingsSelectLanguage(lang) => {
                if let AppScreen::Settings(ref mut ss) = self.screen {
                    ss.language = lang;
                }
            }
        }
    }

    fn focused_selection_mut(&mut self) -> (&mut usize, usize) {
        match self.focused_panel {
            FocusedPanel::Watchlist => (&mut self.selected_watchlist, self.watchlist.len()),
            FocusedPanel::Portfolio => {
                (&mut self.selected_portfolio, self.portfolio.positions.len())
            }
        }
    }

    /// Returns the symbol currently highlighted in the watchlist.
    /// Chart entry is always from the watchlist; portfolio symbols are not chartable via Enter.
    pub fn selected_symbol(&self) -> Option<&Symbol> {
        self.watchlist.get(self.selected_watchlist)
    }

    pub fn strings(&self) -> &'static crate::i18n::Strings {
        match self.language {
            crate::i18n::Language::Zh => &crate::i18n::ZH,
            crate::i18n::Language::En => &crate::i18n::EN,
        }
    }
}

pub type AppState = Arc<RwLock<State>>;

/// Returns the next longer time period, or None if already at the maximum.
pub fn next_longer_period(p: &Period) -> Option<Period> {
    match p {
        Period::Min1 | Period::Min5 | Period::Min15 | Period::Min30 | Period::Min60 => None,
        Period::Day1 => Some(Period::Week1),
        Period::Week1 => Some(Period::Month1),
        Period::Month1 => Some(Period::Month3),
        Period::Month3 => Some(Period::Month6),
        Period::Month6 => Some(Period::Year1),
        Period::Year1 => Some(Period::Year5),
        Period::Year5 => None,
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
    SearchResultsUpdated(u64, Vec<StockSuggestion>),
    SearchSelectNext,
    SearchSelectPrev,
    ConfirmSearchSelection,
    DeleteSelected,
    Refresh,
    QuotesUpdated(Vec<Quote>),
    /// Names resolved via search fallback for watchlist symbols that had no name.
    NamesResolved(Vec<(String, String)>),
    StatusMessage(String),
    EnterChart(Symbol),
    ExitChart,
    ChartDataLoaded(Vec<OHLCV>),
    /// OHLCV fetch failed — error message stored on ChartState and rendered
    /// as a centered overlay in the chart panel.
    ChartFetchFailed(String),
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
    OpenSettings,
    ExitSettings,
    SettingsNavUp,
    SettingsNavDown,
    SettingsSelectProvider(DataSourceKind),
    SettingsEditUrlChar(char),
    SettingsEditUrlBackspace,
    SettingsToggleUrlEdit,
    SettingsSaved,
    RequestQuit,
    CancelQuit,
    ToggleQuitButton,
    SettingsSelectLanguage(crate::i18n::Language),
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
        s.quotes.insert(
            "AAPL".into(),
            fa_core::Quote {
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
            },
        );
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
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        let bars = vec![
            OHLCV {
                symbol: Symbol::new("AAPL", Market::USStock),
                timestamp: Utc::now(),
                open: dec!(100),
                high: dec!(110),
                low: dec!(90),
                close: dec!(105),
                volume: 1000,
            },
            OHLCV {
                symbol: Symbol::new("AAPL", Market::USStock),
                timestamp: Utc::now(),
                open: dec!(105),
                high: dec!(115),
                low: dec!(95),
                close: dec!(110),
                volume: 2000,
            },
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
        let bar = OHLCV {
            symbol: Symbol::new("AAPL", Market::USStock),
            timestamp: Utc::now(),
            open: dec!(100),
            high: dec!(110),
            low: dec!(90),
            close: dec!(105),
            volume: 0,
        };
        s.screen = AppScreen::Chart(ChartState {
            symbol: Symbol::new("AAPL", Market::USStock),
            period: Period::Month1,
            data: vec![bar.clone(), bar],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5],
            loading: false,
            error: None,
            is_load_more: false,
            history_extended: false,
        });
        s.apply(AppAction::ChartMoveCursor(-1));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.cursor, 0);
        }
        s.apply(AppAction::ChartMoveCursor(1));
        s.apply(AppAction::ChartMoveCursor(1));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.cursor, 1);
        }
    }

    #[test]
    fn test_chart_zoom_clamps() {
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        s.apply(AppAction::ChartZoom(false));
        s.apply(AppAction::ChartZoom(false));
        s.apply(AppAction::ChartZoom(false));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.bar_width, 2);
        }
        for _ in 0..10 {
            s.apply(AppAction::ChartZoom(true));
        }
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.bar_width, 8);
        }
    }

    #[test]
    fn test_chart_change_period_resets_data() {
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        let bar = OHLCV {
            symbol: Symbol::new("AAPL", Market::USStock),
            timestamp: Utc::now(),
            open: dec!(100),
            high: dec!(110),
            low: dec!(90),
            close: dec!(105),
            volume: 0,
        };
        s.screen = AppScreen::Chart(ChartState {
            symbol: Symbol::new("AAPL", Market::USStock),
            period: Period::Month1,
            data: vec![bar],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5],
            loading: false,
            error: None,
            is_load_more: false,
            history_extended: false,
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
        s.apply(AppAction::StartBacktest(Symbol::new(
            "AAPL",
            Market::USStock,
        )));
        s.apply(AppAction::ExitBacktest);
        assert!(matches!(s.screen, AppScreen::Main));
    }

    #[test]
    fn test_backtest_next_prev_strategy_wraps() {
        use fa_core::{Market, Symbol};
        let mut s = make_state();
        s.apply(AppAction::StartBacktest(Symbol::new(
            "AAPL",
            Market::USStock,
        )));
        if let AppScreen::Backtest(ref bs) = s.screen {
            assert_eq!(bs.strategy_idx, 0);
        }
        s.apply(AppAction::BacktestNextStrategy);
        if let AppScreen::Backtest(ref bs) = s.screen {
            assert_eq!(bs.strategy_idx, 1);
        }
        s.apply(AppAction::BacktestNextStrategy);
        if let AppScreen::Backtest(ref bs) = s.screen {
            assert_eq!(bs.strategy_idx, 2);
        }
        s.apply(AppAction::BacktestNextStrategy);
        if let AppScreen::Backtest(ref bs) = s.screen {
            assert_eq!(bs.strategy_idx, 0);
        }
    }

    #[test]
    fn test_run_backtest_sets_running_status() {
        use crate::app::BacktestStatus;
        use fa_core::{Market, Symbol};
        let mut s = make_state();
        s.apply(AppAction::StartBacktest(Symbol::new(
            "AAPL",
            Market::USStock,
        )));
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
        assert_eq!(
            s.watchlist[0].name,
            Some("贵州茅台".to_string()),
            "None update must not overwrite cached name"
        );
    }

    #[test]
    fn test_names_resolved_fills_missing_names() {
        use fa_core::{Market, Symbol};
        let mut s = State {
            watchlist: vec![
                {
                    let mut sym = Symbol::new("sh600519", Market::AShare);
                    sym.name = None;
                    sym
                },
                {
                    let mut sym = Symbol::new("000001", Market::AShare);
                    sym.name = Some("已缓存".to_string());
                    sym
                },
                Symbol::new("AAPL", Market::USStock), // name=None, but no resolution provided
            ],
            ..Default::default()
        };

        s.apply(AppAction::NamesResolved(vec![
            ("sh600519".to_string(), "贵州茅台".to_string()),
            ("000001".to_string(), "平安银行".to_string()), // must NOT overwrite cached
            ("MSFT".to_string(), "Microsoft".to_string()),  // not in watchlist → ignored
        ]));

        assert_eq!(s.watchlist[0].name, Some("贵州茅台".to_string()));
        assert_eq!(
            s.watchlist[1].name,
            Some("已缓存".to_string()),
            "existing name must not be overwritten"
        );
        assert_eq!(s.watchlist[2].name, None, "unmatched code is ignored");
    }

    fn make_dummy_ohlcv(symbol: Symbol, count: usize) -> Vec<fa_core::OHLCV> {
        use chrono::Utc;
        use rust_decimal_macros::dec;
        (0..count)
            .map(|i| fa_core::OHLCV {
                symbol: symbol.clone(),
                timestamp: Utc::now() - chrono::Duration::days((count - i) as i64),
                open: dec!(100),
                high: dec!(105),
                low: dec!(95),
                close: dec!(102),
                volume: 1000,
            })
            .collect()
    }

    #[test]
    fn test_next_longer_period_full_chain() {
        use fa_core::Period;
        assert_eq!(next_longer_period(&Period::Min1), None);
        assert_eq!(next_longer_period(&Period::Min5), None);
        assert_eq!(next_longer_period(&Period::Min15), None);
        assert_eq!(next_longer_period(&Period::Min30), None);
        assert_eq!(next_longer_period(&Period::Min60), None);
        assert_eq!(next_longer_period(&Period::Day1), Some(Period::Week1));
        assert_eq!(next_longer_period(&Period::Week1), Some(Period::Month1));
        assert_eq!(next_longer_period(&Period::Month1), Some(Period::Month3));
        assert_eq!(next_longer_period(&Period::Month3), Some(Period::Month6));
        assert_eq!(next_longer_period(&Period::Month6), Some(Period::Year1));
        assert_eq!(next_longer_period(&Period::Year1), Some(Period::Year5));
        assert_eq!(next_longer_period(&Period::Year5), None);
    }

    #[test]
    fn test_chart_load_more_history_sets_extended_flag() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        // Month1 supports extended history
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false;
        }
        s.apply(AppAction::ChartLoadMoreHistory);
        if let AppScreen::Chart(ref cs) = s.screen {
            // Period must NOT change — only history_extended flag is set
            assert_eq!(cs.period, Period::Month1);
            assert!(cs.history_extended);
            assert!(cs.loading);
            assert!(cs.data.is_empty());
            assert_eq!(cs.cursor, 0);
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_load_more_history_noop_for_day1() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        // Day1 does not support extended history (Yahoo intraday range)
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Day1,
        ));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false;
        }
        s.apply(AppAction::ChartLoadMoreHistory);
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Day1);
            assert!(!cs.history_extended, "Day1 should not set history_extended");
            assert!(!cs.loading, "no-op: loading should remain false");
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_load_more_history_noop_when_already_extended() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false;
            cs.history_extended = true; // simulate already extended
        }
        s.apply(AppAction::ChartLoadMoreHistory);
        if let AppScreen::Chart(ref cs) = s.screen {
            // Second press is a no-op
            assert!(!cs.loading, "no-op: should not trigger another fetch");
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_change_period_resets_history_extended() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false;
            cs.history_extended = true;
        }
        s.apply(AppAction::ChartChangePeriod(Period::Year1));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Year1);
            assert!(!cs.history_extended, "ChartChangePeriod must reset history_extended");
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_load_more_history_noop_when_already_at_max_extended() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        // Year5 with history_extended=true: already at the maximum (extended_yahoo_range="max")
        // A second ChartLoadMoreHistory should be a no-op.
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Year5,
        ));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false;
            cs.history_extended = true; // already extended
        }
        s.apply(AppAction::ChartLoadMoreHistory);
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Year5);
            assert!(!cs.loading, "second load-more press should be no-op");
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_chart_data_loaded_load_more_keeps_cursor_at_start() {
        use fa_core::{Market, Period};
        let mut s = State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        // Month1 supports extended history, so ChartLoadMoreHistory sets is_load_more=true
        s.screen = AppScreen::Chart(ChartState::new(sym.clone(), Period::Month1));
        if let AppScreen::Chart(ref mut cs) = s.screen {
            cs.loading = false; // allow ChartLoadMoreHistory to trigger
        }
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
            assert_eq!(
                cs.cursor, 29,
                "normal load should place cursor at newest bar"
            );
        } else {
            panic!("Expected Chart screen");
        }
    }

    #[test]
    fn test_search_select_next_wraps() {
        let mut s = State::default();
        s.is_add_active = true;
        s.search_results = vec![
            fa_data::sina::StockSuggestion {
                code: "600519".into(),
                name: "贵州茅台".into(),
            },
            fa_data::sina::StockSuggestion {
                code: "000001".into(),
                name: "平安银行".into(),
            },
        ];
        s.search_selected = 1;
        s.apply(AppAction::SearchSelectNext);
        assert_eq!(s.search_selected, 0);
    }

    #[test]
    fn test_search_select_prev_wraps() {
        let mut s = State::default();
        s.is_add_active = true;
        s.search_results = vec![
            fa_data::sina::StockSuggestion {
                code: "600519".into(),
                name: "A".into(),
            },
            fa_data::sina::StockSuggestion {
                code: "000001".into(),
                name: "B".into(),
            },
        ];
        s.search_selected = 0;
        s.apply(AppAction::SearchSelectPrev);
        assert_eq!(s.search_selected, 1);
    }

    #[test]
    fn test_confirm_search_selection_adds_to_watchlist() {
        let mut s = State::default();
        s.is_add_active = true;
        s.search_results = vec![fa_data::sina::StockSuggestion {
            code: "600519".into(),
            name: "贵州茅台".into(),
        }];
        s.search_selected = 0;
        s.apply(AppAction::ConfirmSearchSelection);
        assert!(!s.is_add_active);
        assert!(s.search_results.is_empty());
        let added = s.watchlist.last().expect("symbol added");
        assert_eq!(added.code, "600519");
        assert_eq!(added.name.as_deref(), Some("贵州茅台"));
    }

    #[test]
    fn test_search_generation_increments_on_input() {
        let mut s = State::default();
        s.is_add_active = true;
        let gen_before = s.search_generation;
        s.apply(AppAction::UpdateAddInput('a'));
        assert_eq!(s.search_generation, gen_before + 1);
    }

    #[test]
    fn test_stale_search_results_discarded() {
        let mut s = State::default();
        s.is_add_active = true;
        s.search_generation = 5;
        s.apply(AppAction::SearchResultsUpdated(
            3,
            vec![fa_data::sina::StockSuggestion {
                code: "STALE".into(),
                name: "Stale".into(),
            }],
        ));
        assert!(s.search_results.is_empty());
        s.apply(AppAction::SearchResultsUpdated(
            5,
            vec![fa_data::sina::StockSuggestion {
                code: "600519".into(),
                name: "茅台".into(),
            }],
        ));
        assert_eq!(s.search_results.len(), 1);
    }

    #[test]
    fn test_settings_state_toggle_provider() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        assert!(matches!(state.screen, AppScreen::Settings(_)));
        state.apply(AppAction::SettingsSelectProvider(DataSourceKind::AkShare));
        if let AppScreen::Settings(ref ss) = state.screen {
            assert_eq!(ss.provider, DataSourceKind::AkShare);
        } else {
            panic!("expected Settings screen");
        }
    }

    #[test]
    fn test_settings_save_applies_and_closes() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        state.apply(AppAction::SettingsSelectProvider(DataSourceKind::AkShare));
        state.apply(AppAction::SettingsSaved);
        assert!(matches!(state.screen, AppScreen::Main));
        assert_eq!(state.data_source, DataSourceKind::AkShare);
    }

    #[test]
    fn test_settings_save_ignored_outside_settings_screen() {
        let mut state = State::default();
        state.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        state.apply(AppAction::SettingsSaved);
        assert!(matches!(state.screen, AppScreen::Chart(_)));
        assert_eq!(state.data_source, DataSourceKind::Zhitu);
    }

    #[test]
    fn test_settings_exit_ignored_outside_settings_screen() {
        let mut state = State::default();
        state.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            Period::Month1,
        ));
        state.apply(AppAction::ExitSettings);
        assert!(matches!(state.screen, AppScreen::Chart(_)));
    }

    #[test]
    fn test_settings_cancel_closes_screen() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        state.apply(AppAction::SettingsSelectProvider(DataSourceKind::AkShare));
        state.apply(AppAction::ExitSettings);
        assert!(matches!(state.screen, AppScreen::Main));
        assert_eq!(state.data_source, DataSourceKind::Zhitu);
        assert_eq!(state.akshare_url, "http://127.0.0.1:8080");
    }

    #[test]
    fn test_settings_nav() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        if let AppScreen::Settings(ref ss) = state.screen {
            assert_eq!(ss.focused_field, 0);
        } else {
            panic!("expected Settings screen");
        }
        state.apply(AppAction::SettingsNavDown);
        if let AppScreen::Settings(ref ss) = state.screen {
            assert_eq!(ss.focused_field, 1);
        } else {
            panic!("expected Settings screen");
        }
        state.apply(AppAction::SettingsNavUp);
        if let AppScreen::Settings(ref ss) = state.screen {
            assert_eq!(ss.focused_field, 0);
            assert!(!ss.editing_url);
        } else {
            panic!("expected Settings screen");
        }
    }

    #[test]
    fn test_settings_toggle_url_edit_noop_on_provider_field() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        state.apply(AppAction::SettingsToggleUrlEdit);
        if let AppScreen::Settings(ref ss) = state.screen {
            assert!(!ss.editing_url);
        } else {
            panic!("expected Settings screen");
        }
    }

    #[test]
    fn test_settings_url_editing() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        state.apply(AppAction::SettingsNavDown);
        state.apply(AppAction::SettingsToggleUrlEdit);
        state.apply(AppAction::SettingsEditUrlBackspace);
        state.apply(AppAction::SettingsEditUrlChar('X'));
        if let AppScreen::Settings(ref ss) = state.screen {
            assert!(ss.editing_url);
            assert!(ss.akshare_url.ends_with('X'));
        } else {
            panic!("expected Settings screen");
        }
    }

    #[test]
    fn test_settings_nav_stops_url_editing_when_leaving_url_field() {
        let mut state = State::default();
        state.apply(AppAction::OpenSettings);
        state.apply(AppAction::SettingsNavDown);
        state.apply(AppAction::SettingsToggleUrlEdit);
        state.apply(AppAction::SettingsNavUp);
        state.apply(AppAction::SettingsEditUrlChar('X'));
        if let AppScreen::Settings(ref ss) = state.screen {
            assert_eq!(ss.focused_field, 0);
            assert!(!ss.editing_url);
            assert_eq!(ss.akshare_url, "http://127.0.0.1:8080");
        } else {
            panic!("expected Settings screen");
        }
    }

    #[test]
    fn test_request_quit_sets_confirm_flag() {
        let mut s = State::default();
        assert!(!s.confirm_quit);
        s.apply(AppAction::RequestQuit);
        assert!(s.confirm_quit);
        assert!(!s.should_quit);
    }

    #[test]
    fn test_cancel_quit_clears_flag() {
        let mut s = State::default();
        s.confirm_quit = true;
        s.apply(AppAction::CancelQuit);
        assert!(!s.confirm_quit);
    }

    #[test]
    fn test_toggle_quit_button_flips_focus() {
        let mut s = State::default();
        assert!(!s.confirm_quit_focused);
        s.apply(AppAction::ToggleQuitButton);
        assert!(s.confirm_quit_focused);
        s.apply(AppAction::ToggleQuitButton);
        assert!(!s.confirm_quit_focused);
    }

    #[test]
    fn test_state_strings_returns_zh_by_default() {
        let s = State::default();
        assert_eq!(s.strings().watchlist_title, crate::i18n::ZH.watchlist_title);
    }

    #[test]
    fn test_state_strings_returns_en_when_set() {
        let mut s = State::default();
        s.language = crate::i18n::Language::En;
        assert_eq!(s.strings().watchlist_title, crate::i18n::EN.watchlist_title);
    }

    #[test]
    fn test_settings_save_applies_language() {
        let mut s = State::default();
        s.apply(AppAction::OpenSettings);
        s.apply(AppAction::SettingsSelectLanguage(crate::i18n::Language::En));
        s.apply(AppAction::SettingsSaved);
        assert_eq!(s.language, crate::i18n::Language::En);
    }
}
