use chrono::{DateTime, Utc};
use fa_core::{Portfolio, Quote, Symbol};
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
                    self.watchlist.remove(self.selected_watchlist);
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
}
