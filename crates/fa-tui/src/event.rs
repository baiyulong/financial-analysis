use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use fa_core::Period;
use tokio::sync::mpsc;
use std::time::Duration;
use crate::app::{AppAction, AppScreen, AppState};

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
                    if matches!(s.screen, AppScreen::Chart(_)) {
                        Self::map_key_chart(code, modifiers)
                    } else if code == KeyCode::Enter && modifiers == KeyModifiers::NONE {
                        s.selected_symbol().map(|sym| AppAction::EnterChart(sym.clone()))
                    } else {
                        Self::map_key_main(code, modifiers)
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::Period;

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
    fn test_map_key_chart_unknown_key_returns_none() {
        assert!(EventHandler::map_key_chart(KeyCode::F(12), KeyModifiers::NONE).is_none());
        assert!(EventHandler::map_key_chart(KeyCode::Tab, KeyModifiers::NONE).is_none());
    }
}
