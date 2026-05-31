mod config;
mod storage;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fa_backtest::{BuiltinStrategy, Engine};
use fa_core::DataProvider;
use fa_data::{
    akshare::AkShareProvider, router::ProviderRouter, sina::SinaFinanceProvider,
    yahoo::YahooFinanceProvider,
};
use fa_tui::{
    app::{AppAction, AppScreen, AppState, BacktestStatus, DataSourceKind, State},
    event::EventHandler,
    ui,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use storage::Storage;
use tokio::sync::mpsc;

fn build_router(kind: DataSourceKind, akshare_url: &str) -> ProviderRouter {
    match kind {
        DataSourceKind::Sina => ProviderRouter::new(vec![
            Arc::new(SinaFinanceProvider::new()) as Arc<dyn DataProvider>,
            Arc::new(YahooFinanceProvider::new()) as Arc<dyn DataProvider>,
        ]),
        DataSourceKind::AkShare => ProviderRouter::new(vec![
            Arc::new(AkShareProvider::with_base_url(akshare_url.to_string()))
                as Arc<dyn DataProvider>,
            Arc::new(SinaFinanceProvider::new()) as Arc<dyn DataProvider>,
            Arc::new(YahooFinanceProvider::new()) as Arc<dyn DataProvider>,
        ]),
    }
}

fn build_initial_state(db: &Storage) -> State {
    let (provider_opt, url_opt) = db.load_data_source();
    let data_source = match provider_opt.as_deref() {
        Some("akshare") => DataSourceKind::AkShare,
        _ => DataSourceKind::Sina,
    };
    let akshare_url = url_opt.unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

    State {
        watchlist: db.load_watchlist(),
        portfolio: db.load_portfolio(),
        data_source,
        akshare_url,
        ..Default::default()
    }
}

fn router_config_after_action(
    state: &mut State,
    action: AppAction,
) -> Option<(DataSourceKind, String)> {
    let should_capture = matches!(action, AppAction::SettingsSaved)
        && matches!(state.screen, AppScreen::Settings(_));

    state.apply(action);

    if should_capture {
        Some((state.data_source.clone(), state.akshare_url.clone()))
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Open SQLite database (creates fa.db in current working directory)
    let db = Storage::open("fa.db")?;

    // On first run, seed watchlist/portfolio from config/default.toml
    let default_cfg = config::Config::load_defaults();
    db.seed_if_empty(&default_cfg)?;

    let initial_state = build_initial_state(&db);
    let storage: Arc<Mutex<Storage>> = Arc::new(Mutex::new(db));
    let router = Arc::new(RwLock::new(Arc::new(build_router(
        initial_state.data_source.clone(),
        &initial_state.akshare_url,
    ))));
    let app_state: AppState = Arc::new(tokio::sync::RwLock::new(initial_state));

    let (tx, mut rx) = mpsc::channel::<AppAction>(64);

    // Spawn EventHandler — receives AppState for screen-aware key mapping
    let event_tx = tx.clone();
    let event_state = Arc::clone(&app_state);
    tokio::spawn(async move {
        EventHandler::new(event_tx).run(event_state).await;
    });

    // Spawn DataFetcher (periodic quote refresh)
    let fetcher_state = Arc::clone(&app_state);
    let fetcher_tx = tx.clone();
    let refresh_secs = 30u64; // default refresh interval
    let router_clone = Arc::clone(&router);
    tokio::spawn(async move {
        loop {
            let symbols = {
                let s = fetcher_state.read().await;
                s.watchlist.clone()
            };
            let router = {
                let guard = router_clone.read().unwrap();
                Arc::clone(&*guard)
            };

            let mut quotes = Vec::new();
            for sym in &symbols {
                match router.fetch_quote(sym).await {
                    Ok(q) => quotes.push(q),
                    Err(e) => {
                        let _ = fetcher_tx
                            .send(AppAction::StatusMessage(format!(
                                "[!] {} fetch failed: {}",
                                sym.code, e
                            )))
                            .await;
                    }
                }
            }
            if !quotes.is_empty() {
                let _ = fetcher_tx.send(AppAction::QuotesUpdated(quotes)).await;
            }

            tokio::time::sleep(Duration::from_secs(refresh_secs)).await;
        }
    });

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    let result = run_app(
        &mut terminal,
        &app_state,
        &mut rx,
        refresh_secs,
        &router,
        &tx,
        &storage,
    )
    .await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: &AppState,
    rx: &mut mpsc::Receiver<AppAction>,
    refresh_secs: u64,
    router: &Arc<RwLock<Arc<ProviderRouter>>>,
    tx: &mpsc::Sender<AppAction>,
    storage: &Arc<Mutex<Storage>>,
) -> Result<()> {
    let tick = Duration::from_millis(16);

    loop {
        {
            let state = app_state.read().await;
            terminal.draw(|f| ui::draw(f, &state, refresh_secs))?;
            if state.should_quit {
                break;
            }
        }

        let deadline = tokio::time::Instant::now() + tick;
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(action)) => {
                    let needs_ohlcv_fetch = matches!(
                        &action,
                        AppAction::EnterChart(_)
                            | AppAction::ChartChangePeriod(_)
                            | AppAction::ChartLoadMoreHistory
                    );
                    let needs_backtest_run = matches!(&action, AppAction::RunBacktest);
                    let needs_search = matches!(
                        &action,
                        AppAction::UpdateAddInput(_) | AppAction::BackspaceAdd
                    );
                    let is_confirm_add = matches!(&action, AppAction::ConfirmAdd);
                    let is_delete = matches!(&action, AppAction::DeleteSelected);

                    let (symbol_period, backtest_params, search_query, router_config, should_quit, added_symbol, removed_symbol) = {
                        let mut state = app_state.write().await;
                        let prev_len = state.watchlist.len();
                        // Capture the symbol about to be deleted BEFORE apply() removes it
                        let symbol_to_delete = if is_delete {
                            state.watchlist.get(state.selected_watchlist).cloned()
                        } else {
                            None
                        };
                        let is_already_running = if needs_backtest_run {
                            if let AppScreen::Backtest(bs) = &state.screen {
                                matches!(bs.status, BacktestStatus::Running)
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        let router_config = router_config_after_action(&mut state, action);
                        let sp = if needs_ohlcv_fetch {
                            if let AppScreen::Chart(cs) = &state.screen {
                                Some((cs.symbol.clone(), cs.period))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let bp = if needs_backtest_run && !is_already_running {
                            if let AppScreen::Backtest(bs) = &state.screen {
                                Some((bs.symbol.clone(), bs.strategy_idx, bs.config.clone()))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let sq = if needs_search && state.is_add_active {
                            Some((state.add_input.clone(), state.search_generation))
                        } else {
                            None
                        };
                        // Detect newly added symbol
                        let added = if is_confirm_add && state.watchlist.len() > prev_len {
                            state.watchlist.last().cloned()
                        } else {
                            None
                        };
                        // Symbol was removed if watchlist shrank
                        let removed = if is_delete && state.watchlist.len() < prev_len {
                            symbol_to_delete
                        } else {
                            None
                        };
                        (sp, bp, sq, router_config, state.should_quit, added, removed)
                    }; // write lock released here

                    // Persist newly added stock
                    if let Some(ref sym) = added_symbol {
                        if let Ok(db) = storage.lock() {
                            let _ = db.add_to_watchlist(sym);
                        }
                    }
                    // Persist deleted stock
                    if let Some(ref sym) = removed_symbol {
                        if let Ok(db) = storage.lock() {
                            let _ = db.remove_from_watchlist(sym);
                        }
                    }

                    if let Some((kind, akshare_url)) = router_config {
                        let provider_str = match &kind {
                            DataSourceKind::Sina => "sina",
                            DataSourceKind::AkShare => "akshare",
                        };
                        {
                            let mut guard = router.write().unwrap();
                            *guard = Arc::new(build_router(kind, &akshare_url));
                        }
                        if let Ok(db) = storage.lock() {
                            if let Err(err) = db.save_data_source(provider_str, &akshare_url) {
                                let _ = tx
                                    .send(AppAction::StatusMessage(format!(
                                        "[!] 保存数据源配置失败: {}",
                                        err
                                    )))
                                    .await;
                            }
                        }
                    }

                    if let Some((symbol, period)) = symbol_period {
                        let router = {
                            let guard = router.read().unwrap();
                            Arc::clone(&*guard)
                        };
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            match router.fetch_ohlcv(&symbol, period).await {
                                Ok(data) => {
                                    let _ = tx.send(AppAction::ChartDataLoaded(data)).await;
                                }
                                Err(e) => {
                                    let _ = tx
                                        .send(AppAction::StatusMessage(format!(
                                            "K线获取失败: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                    }

                    if let Some((symbol, strategy_idx, config)) = backtest_params {
                        let router = {
                            let guard = router.read().unwrap();
                            Arc::clone(&*guard)
                        };
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            match router.fetch_ohlcv(&symbol, fa_core::Period::Year1).await {
                                Ok(data) => {
                                    let engine = Engine::new(config);
                                    let presets = BuiltinStrategy::all();
                                    let preset =
                                        &presets[strategy_idx.min(presets.len().saturating_sub(1))];
                                    let strategy = preset.to_boxed();
                                    let result = tokio::task::spawn_blocking(move || {
                                        let mut strat = strategy;
                                        engine.run(&data, strat.as_mut())
                                    })
                                    .await
                                    .expect("backtest task panicked");
                                    let _ = tx.send(AppAction::BacktestComplete(result)).await;
                                }
                                Err(e) => {
                                    let _ = tx.send(AppAction::BacktestFailed(e.to_string())).await;
                                }
                            }
                        });
                    }

                    if let Some((query, gen)) = search_query {
                        let tx = tx.clone();
                        if query.is_empty() {
                            let _ = tx.send(AppAction::SearchResultsUpdated(gen, vec![])).await;
                        } else {
                            let router = {
                                let guard = router.read().unwrap();
                                Arc::clone(&*guard)
                            };
                            tokio::spawn(async move {
                                let results = router.search_stocks(&query).await;
                                let _ =
                                    tx.send(AppAction::SearchResultsUpdated(gen, results)).await;
                            });
                        }
                    }

                    if should_quit {
                        return Ok(());
                    }
                }
                Ok(None) => return Ok(()),
                Err(_) => break,
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::Market;
    use std::sync::{Arc, RwLock};

    #[test]
    fn build_initial_state_applies_persisted_data_source_config() {
        let db = crate::storage::Storage::open(":memory:").unwrap();
        db.save_data_source("akshare", "http://127.0.0.1:9000").unwrap();
        let sym = fa_core::Symbol::new("600519", Market::AShare);
        db.add_to_watchlist(&sym).unwrap();

        let state = build_initial_state(&db);

        assert_eq!(state.data_source, DataSourceKind::AkShare);
        assert_eq!(state.akshare_url, "http://127.0.0.1:9000");
        assert_eq!(state.watchlist.len(), 1);
        assert_eq!(state.watchlist[0].code, "600519");
    }

    #[test]
    fn settings_saved_outside_settings_screen_does_not_rebuild_router() {
        let mut state = State::default();
        let router_config = router_config_after_action(&mut state, AppAction::SettingsSaved);

        assert!(router_config.is_none());
        assert_eq!(state.data_source, DataSourceKind::Sina);
    }

    #[test]
    fn build_router_supports_us_stocks_for_sina_mode() {
        let router = build_router(DataSourceKind::Sina, "http://127.0.0.1:8080");

        assert!(router.supports(&Market::USStock));
        assert!(router.supports(&Market::AShare));
    }

    #[test]
    fn shared_router_can_be_rebuilt_for_akshare_mode_without_losing_us_stock_support() {
        let router = Arc::new(RwLock::new(Arc::new(build_router(
            DataSourceKind::Sina,
            "http://127.0.0.1:8080",
        ))));

        {
            let current = router.read().unwrap();
            assert!(current.supports(&Market::USStock));
        }

        {
            let mut guard = router.write().unwrap();
            *guard = Arc::new(build_router(
                DataSourceKind::AkShare,
                "http://127.0.0.1:9999",
            ));
        }

        let current = router.read().unwrap();
        assert!(current.supports(&Market::USStock));
        assert!(current.supports(&Market::AShare));
    }
}
