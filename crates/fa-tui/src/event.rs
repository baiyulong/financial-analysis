use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;
use std::time::Duration;
use crate::app::AppAction;

pub struct EventHandler {
    tx: mpsc::Sender<AppAction>,
    tick_rate: Duration,
}

impl EventHandler {
    pub fn new(tx: mpsc::Sender<AppAction>) -> Self {
        Self { tx, tick_rate: Duration::from_millis(50) }
    }

    pub async fn run(&self) {
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

            let action = match maybe_ev {
                Some(Ok(Event::Key(KeyEvent { code, modifiers, .. }))) => {
                    Self::map_key(code, modifiers)
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

    fn map_key(code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
        match (code, modifiers) {
            (KeyCode::Char('q'), KeyModifiers::NONE) |
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => Some(AppAction::Quit),

            (KeyCode::Tab, _)   => Some(AppAction::NextPanel),
            (KeyCode::Up, _)    => Some(AppAction::MoveUp),
            (KeyCode::Down, _)  => Some(AppAction::MoveDown),

            (KeyCode::Char('/'), KeyModifiers::NONE) => Some(AppAction::StartSearch),
            (KeyCode::Esc, _)                        => Some(AppAction::CancelSearch),
            (KeyCode::Char('d'), KeyModifiers::NONE) => Some(AppAction::DeleteSelected),
            (KeyCode::Char('r'), KeyModifiers::NONE) => Some(AppAction::Refresh),

            (KeyCode::Backspace, _) => Some(AppAction::BackspaceSearch),

            // Character input for search
            (KeyCode::Char(c), KeyModifiers::NONE) |
            (KeyCode::Char(c), KeyModifiers::SHIFT) => Some(AppAction::UpdateSearchInput(c)),

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_quit() {
        assert!(matches!(
            EventHandler::map_key(KeyCode::Char('q'), KeyModifiers::NONE),
            Some(AppAction::Quit)
        ));
        assert!(matches!(
            EventHandler::map_key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(AppAction::Quit)
        ));
    }

    #[test]
    fn test_map_navigation() {
        assert!(matches!(EventHandler::map_key(KeyCode::Tab, KeyModifiers::NONE), Some(AppAction::NextPanel)));
        assert!(matches!(EventHandler::map_key(KeyCode::Up, KeyModifiers::NONE), Some(AppAction::MoveUp)));
        assert!(matches!(EventHandler::map_key(KeyCode::Down, KeyModifiers::NONE), Some(AppAction::MoveDown)));
    }

    #[test]
    fn test_map_search() {
        assert!(matches!(EventHandler::map_key(KeyCode::Char('/'), KeyModifiers::NONE), Some(AppAction::StartSearch)));
        assert!(matches!(EventHandler::map_key(KeyCode::Esc, KeyModifiers::NONE), Some(AppAction::CancelSearch)));
    }

    #[test]
    fn test_unknown_key_returns_none() {
        assert!(EventHandler::map_key(KeyCode::F(12), KeyModifiers::NONE).is_none());
    }
}
