mod config;

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
    ui::{backtest, chart, detail, layout, portfolio, settings, statusbar, watchlist},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    sync::{Arc, RwLock},
    time::Duration,
};
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

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::Config::load()?;

    let initial_state = State {
        watchlist: cfg.to_watchlist_symbols(),
        portfolio: cfg.to_portfolio(),
        ..Default::default()
    };
    let router = Arc::new(RwLock::new(Arc::new(build_router(
        initial_state.data_source.clone(),
        &initial_state.akshare_url,
    ))));
    let app_state: AppState = Arc::new(tokio::sync::RwLock::new(initial_state));

    let (tx, mut rx) = mpsc::channel::<AppAction>(64);

    // Spawn EventHandler — now receives AppState for screen-aware key mapping
    let event_tx = tx.clone();
    let event_state = Arc::clone(&app_state);
    tokio::spawn(async move {
        EventHandler::new(event_tx).run(event_state).await;
    });

    // Spawn DataFetcher (periodic quote refresh)
    let fetcher_state = Arc::clone(&app_state);
    let fetcher_tx = tx.clone();
    let refresh_secs = cfg.general.refresh_interval;
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
) -> Result<()> {
    let tick = Duration::from_millis(16);

    loop {
        {
            let state = app_state.read().await;
            terminal.draw(|f| match &state.screen {
                AppScreen::Main => {
                    let areas = layout::compute(f.area());
                    watchlist::render(f, &state, areas.watchlist);
                    portfolio::render(f, &state, areas.portfolio);
                    detail::render(f, &state, areas.detail);
                    statusbar::render(f, &state, areas.statusbar, refresh_secs);
                }
                AppScreen::Chart(cs) => {
                    chart::render(f, cs, f.area());
                }
                AppScreen::Backtest(bs) => {
                    backtest::render(f, bs, f.area());
                }
                AppScreen::Settings(ss) => {
                    settings::draw_settings(f, f.area(), ss);
                }
            })?;
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

                    let needs_router_rebuild = matches!(&action, AppAction::SettingsSaved);

                    let (symbol_period, backtest_params, search_query, router_config, should_quit) = {
                        let mut state = app_state.write().await;
                        let is_already_running = if needs_backtest_run {
                            if let AppScreen::Backtest(bs) = &state.screen {
                                matches!(bs.status, BacktestStatus::Running)
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        state.apply(action);
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
                        let rc = if needs_router_rebuild {
                            Some((state.data_source.clone(), state.akshare_url.clone()))
                        } else {
                            None
                        };
                        (sp, bp, sq, rc, state.should_quit)
                    }; // write lock released here

                    if let Some((kind, akshare_url)) = router_config {
                        let mut guard = router.write().unwrap();
                        *guard = Arc::new(build_router(kind, &akshare_url));
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
