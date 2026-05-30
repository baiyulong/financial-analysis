use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use fa_core::Period;
use tokio::sync::mpsc;
use std::time::Duration;
use crate::app::{AppAction, AppScreen, AppState, State};

pub struct EventHandler {
    tx: mpsc::Sender<AppAction>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tx: mpsc::Sender<AppAction>) -> Self {
        Self { tx, tick_rate: Duration::from_millis(50) }
    }

    /// Run the event loop. Accepts AppState to determine current screen mode
    /// and to read the selected symbol for Enter key in Main mode.
    pub async fn run(&self, state: AppState) {
        loop {
            if self.tx.is_closed() { break; }

            let tick = self.tick_rate;
            let ev = tokio::task::spawn_blocking(move || {
                if event::poll(tick).unwrap_or(false) {
                    Some(event::read())
                } else {
                    None
                }
            }).await;

            let Ok(maybe_ev) = ev else { break };

            let action: Option<AppAction> = match maybe_ev {
                Some(Ok(Event::Key(KeyEvent { code, modifiers, .. }))) => {
                    let s = state.read().await;
                    Self::resolve_action(&s, code, modifiers)
                }
                _ => None,
            };

            if let Some(action) = action {
                if self.tx.send(action).await.is_err() {
                    break;
                }
            }
        }
    }

    /// Key mappings for Main screen (watchlist/portfolio mode).
    ///
    /// Note: only intended for non-chart, non-add contexts; see `resolve_action` for routing.
    pub fn map_key_main(code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
        match (code, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::NONE) |
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(AppAction::Quit),

            (KeyCode::Tab, _)    => Some(AppAction::NextPanel),
            (KeyCode::Up, _)     => Some(AppAction::MoveUp),
            (KeyCode::Down, _)   => Some(AppAction::MoveDown),

            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(AppAction::StartSearch),
            (KeyCode::Esc, _)                        => Some(AppAction::CancelSearch),
            (KeyCode::Char('d'), KeyModifiers::NONE) => Some(AppAction::DeleteSelected),
            (KeyCode::Char('r'), KeyModifiers::NONE) => Some(AppAction::Refresh),
            (KeyCode::Backspace, _)                  => Some(AppAction::BackspaceSearch),

            (KeyCode::Char(c), KeyModifiers::NONE) |
            (KeyCode::Char(c), KeyModifiers::SHIFT) => Some(AppAction::UpdateSearchInput(c)),

            _ => None,
        }
    }

    /// Key mappings for Chart screen (K-line view mode).
    pub fn map_key_chart(code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
        match (code, modifiers) {
            (KeyCode::Esc, _)                         => Some(AppAction::ExitChart),
            (KeyCode::Left, _)                        => Some(AppAction::ChartMoveCursor(-1)),
            (KeyCode::Right, _)                       => Some(AppAction::ChartMoveCursor(1)),
            (KeyCode::Char('['), KeyModifiers::NONE)  => Some(AppAction::ChartZoom(false)),
            (KeyCode::Char(']'), KeyModifiers::NONE)  => Some(AppAction::ChartZoom(true)),
            (KeyCode::Char('1'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Day1)),
            (KeyCode::Char('5'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Week1)),
            (KeyCode::Char('m'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Month1)),
            (KeyCode::Char('q'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Month3)), // 'q' = quarter (3 months)
            (KeyCode::Char('y'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Year1)),
            _ => None,
        }
    }

    /// Key mappings for Add mode (user is typing a new ticker to add).
    ///
    /// Note: Ctrl+C quit is special-cased in `resolve_action` before this
    /// function is reached, so it is not handled here.
    pub fn map_key_add(code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
        match (code, modifiers) {
            (KeyCode::Enter, _) => Some(AppAction::ConfirmAdd),
            (KeyCode::Esc, _) => Some(AppAction::CancelAdd),
            (KeyCode::Backspace, _) => Some(AppAction::BackspaceAdd),
            (KeyCode::Char(c), KeyModifiers::NONE) |
            (KeyCode::Char(c), KeyModifiers::SHIFT) => Some(AppAction::UpdateAddInput(c)),
            _ => None,
        }
    }

    /// Key mappings for the Backtest screen.
    pub fn map_key_backtest(code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
        match (code, modifiers) {
            (KeyCode::Esc, _)                        => Some(AppAction::ExitBacktest),
            (KeyCode::Char('r'), KeyModifiers::NONE) => Some(AppAction::RunBacktest),
            (KeyCode::Left, _)                       => Some(AppAction::BacktestPrevStrategy),
            (KeyCode::Right, _)                      => Some(AppAction::BacktestNextStrategy),
            (KeyCode::Up, _)                         => Some(AppAction::BacktestScrollUp),
            (KeyCode::Down, _)                       => Some(AppAction::BacktestScrollDown),
            _ => None,
        }
    }

    fn resolve_action(state: &State, code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
        // Always quit on Ctrl+C, regardless of screen or mode.
        if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
            return Some(AppAction::Quit);
        }

        if matches!(state.screen, AppScreen::Chart(_)) {
            Self::map_key_chart(code, modifiers)
        } else if matches!(state.screen, AppScreen::Backtest(_)) {
            Self::map_key_backtest(code, modifiers)
        } else if state.is_add_active {
            Self::map_key_add(code, modifiers)
        } else if !state.is_search_active
            && code == KeyCode::Char('b')
            && modifiers == KeyModifiers::NONE
        {
            state.selected_symbol().map(|sym| AppAction::StartBacktest(sym.clone()))
        } else if !state.is_search_active
            && code == KeyCode::Char('a')
            && modifiers == KeyModifiers::NONE
        {
            Some(AppAction::StartAdd)
        } else if code == KeyCode::Enter && modifiers == KeyModifiers::NONE {
            state.selected_symbol().map(|sym| AppAction::EnterChart(sym.clone()))
        } else {
            Self::map_key_main(code, modifiers)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::State;
    use fa_core::{Market, Period, Symbol};

    #[test]
    fn test_map_quit() {
        assert!(matches!(
            EventHandler::map_key_main(KeyCode::Char('q'), KeyModifiers::NONE),
            Some(AppAction::Quit)
        ));
        assert!(matches!(
            EventHandler::map_key_main(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(AppAction::Quit)
        ));
    }

    #[test]
    fn test_map_navigation() {
        assert!(matches!(EventHandler::map_key_main(KeyCode::Tab, KeyModifiers::NONE), Some(AppAction::NextPanel)));
        assert!(matches!(EventHandler::map_key_main(KeyCode::Up, KeyModifiers::NONE), Some(AppAction::MoveUp)));
        assert!(matches!(EventHandler::map_key_main(KeyCode::Down, KeyModifiers::NONE), Some(AppAction::MoveDown)));
    }

    #[test]
    fn test_map_search() {
        assert!(matches!(EventHandler::map_key_main(KeyCode::Char('/'), KeyModifiers::NONE), Some(AppAction::StartSearch)));
        assert!(matches!(EventHandler::map_key_main(KeyCode::Esc, KeyModifiers::NONE), Some(AppAction::CancelSearch)));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        assert!(EventHandler::map_key_main(KeyCode::F(12), KeyModifiers::NONE).is_none());
    }

    #[test]
    fn test_map_key_chart_navigation() {
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Left, KeyModifiers::NONE),
            Some(AppAction::ChartMoveCursor(-1))
        ));
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Right, KeyModifiers::NONE),
            Some(AppAction::ChartMoveCursor(1))
        ));
    }

    #[test]
    fn test_map_key_chart_zoom() {
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char('['), KeyModifiers::NONE),
            Some(AppAction::ChartZoom(false))
        ));
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char(']'), KeyModifiers::NONE),
            Some(AppAction::ChartZoom(true))
        ));
    }

    #[test]
    fn test_map_key_chart_period() {
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char('1'), KeyModifiers::NONE),
            Some(AppAction::ChartChangePeriod(Period::Day1))
        ));
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char('5'), KeyModifiers::NONE),
            Some(AppAction::ChartChangePeriod(Period::Week1))
        ));
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char('m'), KeyModifiers::NONE),
            Some(AppAction::ChartChangePeriod(Period::Month1))
        ));
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char('q'), KeyModifiers::NONE),
            Some(AppAction::ChartChangePeriod(Period::Month3))
        ));
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Char('y'), KeyModifiers::NONE),
            Some(AppAction::ChartChangePeriod(Period::Year1))
        ));
    }

    #[test]
    fn test_map_key_chart_esc_exits() {
        assert!(matches!(
            EventHandler::map_key_chart(KeyCode::Esc, KeyModifiers::NONE),
            Some(AppAction::ExitChart)
        ));
    }

    #[test]
    fn test_map_key_add_routing() {
        // Enter → ConfirmAdd
        assert!(matches!(
            EventHandler::map_key_add(KeyCode::Enter, KeyModifiers::NONE),
            Some(AppAction::ConfirmAdd)
        ));
        // Esc → CancelAdd
        assert!(matches!(
            EventHandler::map_key_add(KeyCode::Esc, KeyModifiers::NONE),
            Some(AppAction::CancelAdd)
        ));
        // Backspace → BackspaceAdd
        assert!(matches!(
            EventHandler::map_key_add(KeyCode::Backspace, KeyModifiers::NONE),
            Some(AppAction::BackspaceAdd)
        ));
        // Char → UpdateAddInput
        assert!(matches!(
            EventHandler::map_key_add(KeyCode::Char('A'), KeyModifiers::NONE),
            Some(AppAction::UpdateAddInput('A'))
        ));
        assert!(matches!(
            EventHandler::map_key_add(KeyCode::Char('a'), KeyModifiers::SHIFT),
            Some(AppAction::UpdateAddInput('a'))
        ));
    }

    #[test]
    fn test_resolve_action_starts_add_when_idle() {
        let state = State::default();
        assert!(matches!(
            EventHandler::resolve_action(&state, KeyCode::Char('a'), KeyModifiers::NONE),
            Some(AppAction::StartAdd)
        ));
    }

    #[test]
    fn test_resolve_action_keeps_search_input_when_search_active() {
        let mut state = State::default();
        state.is_search_active = true;
        assert!(matches!(
            EventHandler::resolve_action(&state, KeyCode::Char('a'), KeyModifiers::NONE),
            Some(AppAction::UpdateSearchInput('a'))
        ));
    }

    #[test]
    fn test_resolve_action_routes_chars_to_add_when_add_active() {
        let mut state = State::default();
        state.is_add_active = true;
        assert!(matches!(
            EventHandler::resolve_action(&state, KeyCode::Char('x'), KeyModifiers::NONE),
            Some(AppAction::UpdateAddInput('x'))
        ));
    }

    #[test]
    fn test_resolve_action_enters_chart_for_selected_symbol() {
        let mut state = State::default();
        state.watchlist.push(Symbol::new("AAPL", Market::USStock));
        assert!(matches!(
            EventHandler::resolve_action(&state, KeyCode::Enter, KeyModifiers::NONE),
            Some(AppAction::EnterChart(sym)) if sym.code == "AAPL"
        ));
    }

    #[test]
    fn test_resolve_action_ctrlc_always_quits_even_in_add_mode() {
        let mut state = State::default();
        state.is_add_active = true;
        assert!(matches!(
            EventHandler::resolve_action(&state, KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(AppAction::Quit)
        ));
    }

    #[test]
    fn test_resolve_action_ctrlc_quits_in_chart_mode() {
        use crate::app::{AppScreen, ChartState};
        let mut state = State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        state.screen = AppScreen::Chart(ChartState::new(sym, fa_core::Period::Month1));
        assert!(matches!(
            EventHandler::resolve_action(&state, KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(AppAction::Quit)
        ));
    }

    #[test]
    fn test_map_key_chart_unknown_key_returns_none() {
        assert!(EventHandler::map_key_chart(KeyCode::F(12), KeyModifiers::NONE).is_none());
        assert!(EventHandler::map_key_chart(KeyCode::Tab, KeyModifiers::NONE).is_none());
    }

    #[test]
    fn test_map_key_backtest_run() {
        assert!(matches!(
            EventHandler::map_key_backtest(KeyCode::Char('r'), KeyModifiers::NONE),
            Some(AppAction::RunBacktest)
        ));
    }

    #[test]
    fn test_map_key_backtest_esc_exits() {
        assert!(matches!(
            EventHandler::map_key_backtest(KeyCode::Esc, KeyModifiers::NONE),
            Some(AppAction::ExitBacktest)
        ));
    }

    #[test]
    fn test_map_key_backtest_scroll() {
        assert!(matches!(
            EventHandler::map_key_backtest(KeyCode::Up, KeyModifiers::NONE),
            Some(AppAction::BacktestScrollUp)
        ));
        assert!(matches!(
            EventHandler::map_key_backtest(KeyCode::Down, KeyModifiers::NONE),
            Some(AppAction::BacktestScrollDown)
        ));
    }

    #[test]
    fn test_b_key_starts_backtest_when_symbol_selected() {
        let mut state = State::default();
        state.watchlist.push(Symbol::new("AAPL", Market::USStock));
        let action = EventHandler::resolve_action(&state, KeyCode::Char('b'), KeyModifiers::NONE);
        assert!(matches!(action, Some(AppAction::StartBacktest(_))));
    }

    #[test]
    fn test_resolve_backtest_screen_routes_to_map_key_backtest() {
        use crate::app::{AppScreen, BacktestState};
        let mut state = State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        state.screen = AppScreen::Backtest(BacktestState::new(sym));
        let action = EventHandler::resolve_action(&state, KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(matches!(action, Some(AppAction::RunBacktest)));
    }
}
