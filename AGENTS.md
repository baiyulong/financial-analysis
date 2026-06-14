# AGENTS.md

This file provides guidance to the AI agent when working with code in this repository.

## Project

`fa` is a Rust TUI stock analysis tool (Ratatui) for A-shares and US stocks. Workspace with 5 library crates (`fa-core`, `fa-data`, `fa-indicator`, `fa-tui`, `fa-backtest`) plus the binary crate at `src/`.

## Build & Test

```bash
cargo run --release              # build and run the TUI
cargo test --workspace           # all tests
cargo test -p fa-tui             # chart/UI tests only (fast, no network)
cargo test -p fa-core            # core type tests (fast, no network)
cargo test -p fa-backtest        # backtest engine tests (fast, no network)
cargo check --workspace          # compile check without building
```

`fa-data` tests use `mockito` for HTTP mocking — they are self-contained and do not hit real APIs.

## Commit Style

Conventional commits: `feat(scope): ...`, `fix(scope): ...`, `refactor: ...`, `docs: ...`. Scope is usually the crate name or feature area (e.g., `feat(chart):`, `fix(i18n):`, `feat(fa-tui):`).

## Architecture

- **State flow**: `Arc<tokio::sync::RwLock<State>>` shared across async tasks. User input and data updates are sent as `AppAction` enum variants through a `tokio::sync::mpsc` channel. The main loop applies actions, then spawns async tasks for network fetches (OHLCV, search, backtest). Never mutate state directly from spawned tasks — send an `AppAction` back through the channel.
- **Data routing**: `ProviderRouter` holds an ordered list of `dyn DataProvider` and tries each in sequence. Provider order matters — see `build_router()` in `src/main.rs`.
- **Persistence**: SQLite at `./fa.db` (current working directory, **not** `~/.config/`). Schema auto-migrates on open. `config/default.toml` seeds the DB only on first run when tables are empty.
- **i18n**: All user-visible strings go through `fa_tui::i18n::Language`. Never add hardcoded Chinese or English strings to UI code — add both translations to the `i18n` module.

## Chart Rendering

K-line charts use Unicode block characters (`█` `│` `─`) rendered via Ratatui widgets — not a charting library. Red = up, green = down (A-share convention, opposite of Western). When modifying chart code, test with both positive and negative OHLCV data.

## Data Sources

- **Sina Finance** (default): A-share quotes and daily/weekly/monthly K-lines. No setup needed.
- **AkShare**: Minute-level K-lines via a local Python HTTP service (`python -m aktools` on port 8080). Requires the service running locally.
- **Yahoo Finance**: Fallback for US stocks and some historical data.
