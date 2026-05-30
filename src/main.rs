mod config;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use fa_core::DataProvider;
use fa_data::{router::ProviderRouter, sina::SinaFinanceProvider, yahoo::YahooFinanceProvider};
use fa_tui::{
    app::{AppAction, AppScreen, AppState, State},
    event::EventHandler,
    ui::{backtest, chart, detail, layout, portfolio, statusbar, watchlist},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::Config::load()?;

    let router = Arc::new(ProviderRouter::new(vec![
        Arc::new(YahooFinanceProvider::new()),
        Arc::new(SinaFinanceProvider::new()),
    ]));

    let initial_state = State {
        watchlist: cfg.to_watchlist_symbols(),
        portfolio: cfg.to_portfolio(),
        ..Default::default()
    };
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

            let mut quotes = Vec::new();
            for sym in &symbols {
                match router_clone.fetch_quote(sym).await {
                    Ok(q) => quotes.push(q),
                    Err(e) => {
                        let _ = fetcher_tx
                            .send(AppAction::StatusMessage(format!(
                                "[!] {} fetch failed: {}", sym.code, e
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

    let result = run_app(&mut terminal, &app_state, &mut rx, refresh_secs, &router, &tx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: &AppState,
    rx: &mut mpsc::Receiver<AppAction>,
    refresh_secs: u64,
    router: &Arc<ProviderRouter>,
    tx: &mpsc::Sender<AppAction>,
) -> Result<()> {
    let tick = Duration::from_millis(16);

    loop {
        {
            let state = app_state.read().await;
            terminal.draw(|f| {
                match &state.screen {
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
                }
            })?;
            if state.should_quit { break; }
        }

        let deadline = tokio::time::Instant::now() + tick;
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(action)) => {
                    let needs_ohlcv_fetch = matches!(
                        &action,
                        AppAction::EnterChart(_) | AppAction::ChartChangePeriod(_)
                    );

                    let (symbol_period, should_quit) = {
                        let mut state = app_state.write().await;
                        state.apply(action);
                        let sp = if needs_ohlcv_fetch {
                            if let AppScreen::Chart(cs) = &state.screen {
                                Some((cs.symbol.clone(), cs.period))
                            } else { None }
                        } else { None };
                        (sp, state.should_quit)
                    }; // write lock released here

                    if let Some((symbol, period)) = symbol_period {
                        let router = Arc::clone(router);
                        let tx = tx.clone();
                        tokio::spawn(async move {
                            match router.fetch_ohlcv(&symbol, period).await {
                                Ok(data) => {
                                    let _ = tx.send(AppAction::ChartDataLoaded(data)).await;
                                }
                                Err(e) => {
                                    let _ = tx.send(AppAction::StatusMessage(
                                        format!("K线获取失败: {}", e)
                                    )).await;
                                }
                            }
                        });
                    }

                    if should_quit { return Ok(()); }
                }
                Ok(None) => return Ok(()),
                Err(_) => break,
            }
        }
    }

    Ok(())
}
