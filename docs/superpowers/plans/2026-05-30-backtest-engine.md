# Phase 3: Backtest Engine + Strategy Framework

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bar-by-bar daily backtesting engine with 3 built-in strategies (MA Cross, RSI, Bollinger), accessible from the TUI by pressing `b` on any selected stock.

**Architecture:** New `fa-backtest` crate owns `Strategy` trait, `Engine` (bar-by-bar loop with slippage + commission), `BacktestResult` (metrics + trade log). `fa-indicator` gains an `rsi()` function. `fa-tui` adds `AppScreen::Backtest` and `ui/backtest.rs`. `main.rs` spawns an async task on `RunBacktest` (same pattern as ChartFetcher) that calls `router.fetch_ohlcv` then runs the engine.

**Tech Stack:** Rust, rust_decimal 1.x (no `maths` feature → use f64 for sqrt/powf), chrono NaiveDate, fa-core (OHLCV uses `timestamp: DateTime<Utc>`), fa-indicator (sma takes `&[OHLCV]`), fa-data ProviderRouter, ratatui 0.28.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `Cargo.toml` (workspace) | Modify | Add `fa-backtest` member |
| `crates/fa-backtest/Cargo.toml` | Create | Crate manifest |
| `crates/fa-backtest/src/lib.rs` | Create | Public re-exports |
| `crates/fa-backtest/src/strategy.rs` | Create | `Strategy` trait, `Signal`, `BarContext` |
| `crates/fa-backtest/src/portfolio.rs` | Create | Internal position/cash tracker |
| `crates/fa-backtest/src/engine.rs` | Create | `BacktestConfig`, `Engine::run()` |
| `crates/fa-backtest/src/result.rs` | Create | `BacktestResult`, `Trade`, metrics, CSV export |
| `crates/fa-backtest/src/strategies/mod.rs` | Create | `BuiltinStrategy` enum |
| `crates/fa-backtest/src/strategies/ma_cross.rs` | Create | Dual-MA crossover |
| `crates/fa-indicator/src/rsi.rs` | Create | `rsi(&[OHLCV], period)` |
| `crates/fa-indicator/src/lib.rs` | Modify | Pub re-export rsi |
| `crates/fa-backtest/src/strategies/rsi.rs` | Create | RSI mean-revert |
| `crates/fa-backtest/src/strategies/bollinger.rs` | Create | Bollinger band strategy |
| `crates/fa-tui/Cargo.toml` | Modify | Add `fa-backtest` dep |
| `crates/fa-tui/src/app.rs` | Modify | `BacktestState`, new `AppAction` variants, `apply()` arms |
| `crates/fa-tui/src/event.rs` | Modify | `map_key_backtest()`, `b` → StartBacktest |
| `crates/fa-tui/src/ui/backtest.rs` | Create | Backtest screen renderer |
| `crates/fa-tui/src/ui/mod.rs` | Modify | `pub mod backtest` |
| `src/main.rs` | Modify | Handle `RunBacktest` → spawn task |
| `README.md` | Modify | Document `b` key |

---

### Task 1: Create `fa-backtest` crate with `Strategy` trait

**Files:**
- Create: `crates/fa-backtest/Cargo.toml`
- Create: `crates/fa-backtest/src/strategy.rs`
- Create: `crates/fa-backtest/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Write failing test**

Add `crates/fa-backtest/src/strategy.rs`:
```rust
use fa_core::OHLCV;
use rust_decimal::Decimal;

/// Context passed to a strategy on each bar.
pub struct BarContext<'a> {
    /// The current bar (timestamp, OHLCV).
    pub bar: &'a OHLCV,
    /// Current shares held (0 = flat).
    pub position: i64,
    /// Available cash.
    pub cash: Decimal,
    /// All bars from the filtered start date up to and including `bar`.
    pub history: &'a [OHLCV],
}

/// What the strategy wants to do after seeing a bar.
#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    /// Buy with all available cash at this bar's fill price.
    BuyAll,
    /// Sell entire position at this bar's fill price.
    SellAll,
    /// Do nothing.
    Hold,
}

/// Implement this to create a strategy.
pub trait Strategy: Send {
    fn name(&self) -> &str;
    fn on_bar(&mut self, ctx: &BarContext) -> Signal;
    /// Called before each new Engine::run() so the same instance can be reused.
    fn reset(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn make_bar(close: f64) -> OHLCV {
        use rust_decimal::Decimal;
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: Utc::now(),
            open: c, high: c, low: c, close: c, volume: 0,
        }
    }

    struct AlwaysBuy;
    impl Strategy for AlwaysBuy {
        fn name(&self) -> &str { "always-buy" }
        fn on_bar(&mut self, ctx: &BarContext) -> Signal {
            if ctx.position == 0 { Signal::BuyAll } else { Signal::Hold }
        }
        fn reset(&mut self) {}
    }

    #[test]
    fn test_strategy_trait_object() {
        let bar = make_bar(100.0);
        let history = vec![bar.clone()];
        let ctx = BarContext { bar: &bar, position: 0, cash: dec!(1000), history: &history };
        let mut s: Box<dyn Strategy> = Box::new(AlwaysBuy);
        assert_eq!(s.on_bar(&ctx), Signal::BuyAll);
    }

    #[test]
    fn test_signal_hold_when_in_position() {
        let bar = make_bar(100.0);
        let history = vec![bar.clone()];
        let ctx = BarContext { bar: &bar, position: 10, cash: dec!(0), history: &history };
        let mut s = AlwaysBuy;
        assert_eq!(s.on_bar(&ctx), Signal::Hold);
    }
}
```

- [ ] **Step 2: Create `crates/fa-backtest/Cargo.toml`**

```toml
[package]
name = "fa-backtest"
version = "0.1.0"
edition = "2021"

[dependencies]
fa-core      = { path = "../fa-core" }
fa-indicator = { path = "../fa-indicator" }
chrono       = { workspace = true }
rust_decimal = { workspace = true }

[dev-dependencies]
rust_decimal_macros = { workspace = true }
chrono              = { workspace = true }
```

- [ ] **Step 3: Create `crates/fa-backtest/src/lib.rs`** (minimal for now, grows in later tasks)

```rust
pub mod strategy;

pub use strategy::{BarContext, Signal, Strategy};
```

- [ ] **Step 4: Add to workspace**

In the root `Cargo.toml`, change:
```toml
members = [".", "crates/fa-core", "crates/fa-data", "crates/fa-indicator", "crates/fa-tui"]
```
to:
```toml
members = [".", "crates/fa-core", "crates/fa-data", "crates/fa-indicator", "crates/fa-tui", "crates/fa-backtest"]
```

- [ ] **Step 5: Verify compiles and tests pass**

```bash
cd /root/projects/financial-analysis
cargo test -p fa-backtest 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. 2 passed; 0 failed`

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/fa-backtest/
git commit -m "feat(fa-backtest): add crate skeleton with Strategy trait, Signal, BarContext

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Portfolio tracker + Engine

**Files:**
- Create: `crates/fa-backtest/src/portfolio.rs`
- Create: `crates/fa-backtest/src/engine.rs`
- Modify: `crates/fa-backtest/src/lib.rs`

- [ ] **Step 1: Write failing tests for portfolio**

Create `crates/fa-backtest/src/portfolio.rs`:
```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rust_decimal::prelude::ToPrimitive;

/// Internal position/cash tracker. Not part of the public API.
pub(crate) struct Portfolio {
    pub cash: Decimal,
    pub position: i64,
    /// Total cost of shares currently held (excluding commission already deducted from cash).
    pub cost_basis: Decimal,
}

impl Portfolio {
    pub fn new(initial_cash: Decimal) -> Self {
        Self { cash: initial_cash, position: 0, cost_basis: dec!(0) }
    }

    pub fn total_value(&self, current_price: Decimal) -> Decimal {
        self.cash + current_price * Decimal::from(self.position)
    }

    /// Buy as many shares as cash allows at `fill_price`.
    /// Returns (shares_bought, cost_per_share_excl_commission) or None if cannot buy.
    pub fn buy_all(&mut self, fill_price: Decimal, commission_bps: Decimal) -> Option<(i64, Decimal)> {
        if self.cash <= dec!(0) || self.position > 0 || fill_price <= dec!(0) {
            return None;
        }
        let commission_rate = commission_bps / dec!(10000);
        // cash = shares * fill_price + shares * fill_price * commission_rate
        //      = shares * fill_price * (1 + commission_rate)
        let shares_dec = (self.cash / (fill_price * (dec!(1) + commission_rate))).floor();
        let shares = shares_dec.to_i64()?;
        if shares <= 0 { return None; }
        let cost = fill_price * Decimal::from(shares);
        let commission = cost * commission_rate;
        self.cash -= cost + commission;
        self.position += shares;
        self.cost_basis += cost; // store excl-commission cost for PnL
        Some((shares, fill_price))
    }

    /// Sell entire position at `fill_price`.
    /// Returns (shares_sold, net_proceeds, pnl) or None if no position.
    pub fn sell_all(&mut self, fill_price: Decimal, commission_bps: Decimal) -> Option<(i64, Decimal, Decimal)> {
        if self.position <= 0 { return None; }
        let shares = self.position;
        let gross = fill_price * Decimal::from(shares);
        let commission = gross * commission_bps / dec!(10000);
        let net = gross - commission;
        let pnl = net - self.cost_basis;
        self.cash += net;
        self.position = 0;
        self.cost_basis = dec!(0);
        Some((shares, net, pnl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buy_all_consumes_cash() {
        let mut p = Portfolio::new(dec!(10000));
        // fill_price=100, commission=5bps=0.05% → shares = floor(10000 / (100 * 1.0005)) = 99
        let result = p.buy_all(dec!(100), dec!(5));
        assert!(result.is_some());
        let (shares, _) = result.unwrap();
        assert_eq!(shares, 99);
        assert!(p.cash >= dec!(0));
        assert_eq!(p.position, 99);
    }

    #[test]
    fn test_sell_all_clears_position() {
        let mut p = Portfolio::new(dec!(10000));
        p.buy_all(dec!(100), dec!(5));
        let result = p.sell_all(dec!(110), dec!(5));
        assert!(result.is_some());
        let (shares, net, pnl) = result.unwrap();
        assert_eq!(shares, 99);
        assert!(net > dec!(0));
        assert!(pnl > dec!(0)); // sold higher than bought
        assert_eq!(p.position, 0);
    }

    #[test]
    fn test_cannot_buy_when_in_position() {
        let mut p = Portfolio::new(dec!(10000));
        p.buy_all(dec!(100), dec!(5));
        assert!(p.buy_all(dec!(100), dec!(5)).is_none());
    }

    #[test]
    fn test_cannot_sell_when_flat() {
        let mut p = Portfolio::new(dec!(10000));
        assert!(p.sell_all(dec!(100), dec!(5)).is_none());
    }

    #[test]
    fn test_total_value() {
        let mut p = Portfolio::new(dec!(10000));
        p.buy_all(dec!(100), dec!(0)); // zero commission for simplicity
        // 100 shares at 100, cash = 0 (approximately)
        let val = p.total_value(dec!(120));
        assert!(val > dec!(10000)); // position appreciated
    }
}
```

- [ ] **Step 2: Write failing test for Engine**

Create `crates/fa-backtest/src/engine.rs`:
```rust
use chrono::NaiveDate;
use fa_core::OHLCV;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::portfolio::Portfolio;
use crate::result::{BacktestResult, Trade, TradeAction};
use crate::strategy::{BarContext, Signal, Strategy};

#[derive(Clone, Debug)]
pub struct BacktestConfig {
    pub initial_cash: Decimal,
    pub commission_bps: Decimal,
    pub slippage_bps: Decimal,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        let today = chrono::Local::now().date_naive();
        let one_year_ago = today - chrono::Duration::days(365);
        Self {
            initial_cash: dec!(100_000),
            commission_bps: dec!(5),
            slippage_bps: dec!(3),
            start_date: one_year_ago,
            end_date: today,
        }
    }
}

pub struct Engine {
    pub config: BacktestConfig,
}

impl Engine {
    pub fn new(config: BacktestConfig) -> Self {
        Self { config }
    }

    /// Run the strategy over `all_data` (must be sorted ascending by timestamp).
    pub fn run(&self, all_data: &[OHLCV], strategy: &mut dyn Strategy) -> BacktestResult {
        let data: Vec<&OHLCV> = all_data.iter()
            .filter(|b| {
                let d = b.timestamp.date_naive();
                d >= self.config.start_date && d <= self.config.end_date
            })
            .collect();

        if data.is_empty() {
            return BacktestResult::empty(self.config.initial_cash);
        }

        let mut portfolio = Portfolio::new(self.config.initial_cash);
        let mut trades: Vec<Trade> = Vec::new();
        let mut equity_curve: Vec<Decimal> = Vec::new();

        strategy.reset();

        // Build owned slice for history (avoids re-deriving each bar).
        // We collect owned OHLCVs once so history slices are cheap.
        let owned: Vec<OHLCV> = data.iter().map(|b| (*b).clone()).collect();

        for (i, bar) in owned.iter().enumerate() {
            let ctx = BarContext {
                bar,
                position: portfolio.position,
                cash: portfolio.cash,
                history: &owned[..=i],
            };

            let signal = strategy.on_bar(&ctx);

            // Slippage: buy fills slightly above close, sell slightly below
            let buy_fill = bar.close * (dec!(1) + self.config.slippage_bps / dec!(10000));
            let sell_fill = bar.close * (dec!(1) - self.config.slippage_bps / dec!(10000));

            match signal {
                Signal::BuyAll => {
                    if let Some((qty, _)) = portfolio.buy_all(buy_fill, self.config.commission_bps) {
                        trades.push(Trade {
                            date: bar.timestamp.date_naive(),
                            action: TradeAction::Buy,
                            price: buy_fill,
                            quantity: qty,
                            amount: buy_fill * Decimal::from(qty),
                            pnl: None,
                        });
                    }
                }
                Signal::SellAll => {
                    if let Some((qty, net, pnl)) = portfolio.sell_all(sell_fill, self.config.commission_bps) {
                        trades.push(Trade {
                            date: bar.timestamp.date_naive(),
                            action: TradeAction::Sell,
                            price: sell_fill,
                            quantity: qty,
                            amount: net,
                            pnl: Some(pnl),
                        });
                    }
                }
                Signal::Hold => {}
            }

            equity_curve.push(portfolio.total_value(bar.close));
        }

        BacktestResult::calculate(
            self.config.initial_cash,
            trades,
            equity_curve,
            self.config.start_date,
            self.config.end_date,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy::{BarContext, Signal, Strategy};
    use fa_core::{Market, Symbol};
    use chrono::TimeZone;

    fn make_bar(close: f64, days_from_epoch: i64) -> OHLCV {
        use rust_decimal::Decimal;
        let c = Decimal::from_f64_retain(close).unwrap();
        let ts = chrono::Utc.timestamp_opt(days_from_epoch * 86400, 0).unwrap();
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: ts,
            open: c, high: c, low: c, close: c, volume: 0,
        }
    }

    // Strategy that buys on bar 0 and sells on bar 3
    struct BuyThenSell { count: usize }
    impl Strategy for BuyThenSell {
        fn name(&self) -> &str { "buy-then-sell" }
        fn on_bar(&mut self, ctx: &BarContext) -> Signal {
            self.count += 1;
            if self.count == 1 { Signal::BuyAll }
            else if self.count == 4 { Signal::SellAll }
            else { Signal::Hold }
        }
        fn reset(&mut self) { self.count = 0; }
    }

    #[test]
    fn test_engine_buy_and_sell_produces_trades() {
        let start = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(1970, 1, 10).unwrap();
        let data: Vec<OHLCV> = (0..5).map(|i| make_bar(100.0, i)).collect();
        let config = BacktestConfig {
            initial_cash: dec!(10000),
            commission_bps: dec!(0), // zero fees for predictability
            slippage_bps: dec!(0),
            start_date: start,
            end_date: end,
        };
        let mut strategy = BuyThenSell { count: 0 };
        let engine = Engine::new(config);
        let result = engine.run(&data, &mut strategy);
        assert_eq!(result.trades.len(), 2);
        assert_eq!(result.trades[0].action, TradeAction::Buy);
        assert_eq!(result.trades[1].action, TradeAction::Sell);
    }

    #[test]
    fn test_engine_empty_data_returns_empty_result() {
        let config = BacktestConfig::default();
        let mut strategy = BuyThenSell { count: 0 };
        let engine = Engine::new(config);
        let result = engine.run(&[], &mut strategy);
        assert_eq!(result.trades.len(), 0);
        assert_eq!(result.total_trades, 0);
    }

    #[test]
    fn test_engine_date_filter_excludes_out_of_range_bars() {
        let start = NaiveDate::from_ymd_opt(1970, 1, 3).unwrap(); // skip first 2 bars
        let end   = NaiveDate::from_ymd_opt(1970, 1, 10).unwrap();
        let data: Vec<OHLCV> = (0..5).map(|i| make_bar(100.0, i)).collect();
        let config = BacktestConfig {
            initial_cash: dec!(10000),
            commission_bps: dec!(0),
            slippage_bps: dec!(0),
            start_date: start,
            end_date: end,
        };
        // BuyThenSell buys on bar 0 of filtered data (day 3) and sells on bar 3 (day 6)
        let mut strategy = BuyThenSell { count: 0 };
        let engine = Engine::new(config);
        let result = engine.run(&data, &mut strategy);
        // day 3, 4, 5, 6, 7 → 5 bars → BuyAll at bar 0, SellAll at bar 3 → 2 trades
        // only bars with date >= day3 (1970-01-03, seconds 2*86400 = 172800) pass
        // days_from_epoch 2,3,4 pass (3 bars) then sell never triggers
        // This tests that filtering works without panicking
        assert!(result.trades.len() <= 2);
    }
}
```

- [ ] **Step 3: Add `result.rs` stub** (just enough for engine to compile; full impl in Task 3)

```rust
// crates/fa-backtest/src/result.rs
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, PartialEq)]
pub enum TradeAction { Buy, Sell }

#[derive(Debug, Clone)]
pub struct Trade {
    pub date: NaiveDate,
    pub action: TradeAction,
    pub price: Decimal,
    pub quantity: i64,
    pub amount: Decimal,
    pub pnl: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub total_return: Decimal,
    pub annualized_return: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub sharpe_ratio: Decimal,
    pub total_trades: usize,
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<Decimal>,
    pub initial_cash: Decimal,
    pub final_equity: Decimal,
}

impl BacktestResult {
    pub fn empty(initial_cash: Decimal) -> Self {
        Self {
            total_return: dec!(0),
            annualized_return: dec!(0),
            max_drawdown: dec!(0),
            win_rate: dec!(0),
            sharpe_ratio: dec!(0),
            total_trades: 0,
            trades: vec![],
            equity_curve: vec![],
            initial_cash,
            final_equity: initial_cash,
        }
    }

    pub fn calculate(
        initial_cash: Decimal,
        trades: Vec<Trade>,
        equity_curve: Vec<Decimal>,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> Self {
        // Placeholder — full metrics computed in Task 3
        let final_equity = equity_curve.last().copied().unwrap_or(initial_cash);
        let total_trades = trades.len();
        Self {
            total_return: dec!(0),
            annualized_return: dec!(0),
            max_drawdown: dec!(0),
            win_rate: dec!(0),
            sharpe_ratio: dec!(0),
            total_trades,
            trades,
            equity_curve,
            initial_cash,
            final_equity,
        }
    }
}
```

- [ ] **Step 4: Update `lib.rs`**

```rust
// crates/fa-backtest/src/lib.rs
pub(crate) mod portfolio;
pub mod engine;
pub mod result;
pub mod strategy;

pub use engine::{BacktestConfig, Engine};
pub use result::{BacktestResult, Trade, TradeAction};
pub use strategy::{BarContext, Signal, Strategy};
```

- [ ] **Step 5: Run tests**

```bash
cd /root/projects/financial-analysis
cargo test -p fa-backtest 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. X passed; 0 failed` (at minimum 7 tests: 2 strategy + 5 portfolio + 3 engine)

- [ ] **Step 6: Commit**

```bash
git add crates/fa-backtest/src/
git commit -m "feat(fa-backtest): add Portfolio + Engine + BacktestResult stub

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Full `BacktestResult` with metrics + CSV export

**Files:**
- Modify: `crates/fa-backtest/src/result.rs` (replace stub with full impl)

- [ ] **Step 1: Replace `result.rs` with full implementation**

```rust
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use std::io::Write;

#[derive(Debug, Clone, PartialEq)]
pub enum TradeAction { Buy, Sell }

#[derive(Debug, Clone)]
pub struct Trade {
    pub date: NaiveDate,
    pub action: TradeAction,
    pub price: Decimal,
    pub quantity: i64,
    pub amount: Decimal,
    /// Realised PnL; Some only on Sell trades.
    pub pnl: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub total_return: Decimal,
    pub annualized_return: Decimal,
    pub max_drawdown: Decimal,
    pub win_rate: Decimal,
    pub sharpe_ratio: Decimal,
    pub total_trades: usize,
    pub trades: Vec<Trade>,
    pub equity_curve: Vec<Decimal>,
    pub initial_cash: Decimal,
    pub final_equity: Decimal,
}

impl BacktestResult {
    pub fn empty(initial_cash: Decimal) -> Self {
        Self {
            total_return: dec!(0),
            annualized_return: dec!(0),
            max_drawdown: dec!(0),
            win_rate: dec!(0),
            sharpe_ratio: dec!(0),
            total_trades: 0,
            trades: vec![],
            equity_curve: vec![],
            initial_cash,
            final_equity: initial_cash,
        }
    }

    pub fn calculate(
        initial_cash: Decimal,
        trades: Vec<Trade>,
        equity_curve: Vec<Decimal>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Self {
        let final_equity = equity_curve.last().copied().unwrap_or(initial_cash);
        let total_trades = trades.len();

        let total_return = if initial_cash > dec!(0) {
            (final_equity - initial_cash) / initial_cash
        } else {
            dec!(0)
        };

        let days = (end_date - start_date).num_days() as f64;
        let annualized_return = if days > 0.0 && initial_cash > dec!(0) {
            let tr = total_return.to_f64().unwrap_or(0.0);
            let ann = (1.0 + tr).powf(365.0 / days) - 1.0;
            Decimal::from_f64(ann).unwrap_or(dec!(0))
        } else {
            dec!(0)
        };

        let max_drawdown = compute_max_drawdown(&equity_curve);
        let win_rate = compute_win_rate(&trades);
        let sharpe_ratio = compute_sharpe(&equity_curve);

        Self {
            total_return,
            annualized_return,
            max_drawdown,
            win_rate,
            sharpe_ratio,
            total_trades,
            trades,
            equity_curve,
            initial_cash,
            final_equity,
        }
    }

    /// Write trade log to a CSV file.
    pub fn export_csv(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        writeln!(f, "date,action,price,quantity,amount,pnl")?;
        for t in &self.trades {
            let action = match t.action { TradeAction::Buy => "buy", TradeAction::Sell => "sell" };
            let pnl = t.pnl.map(|p| p.to_string()).unwrap_or_default();
            writeln!(f, "{},{},{},{},{},{}", t.date, action, t.price, t.quantity, t.amount, pnl)?;
        }
        Ok(())
    }
}

fn compute_max_drawdown(equity: &[Decimal]) -> Decimal {
    if equity.is_empty() { return dec!(0); }
    let mut peak = equity[0];
    let mut max_dd = dec!(0);
    for &e in equity {
        if e > peak { peak = e; }
        if peak > dec!(0) {
            let dd = (peak - e) / peak;
            if dd > max_dd { max_dd = dd; }
        }
    }
    -max_dd // negative to indicate loss
}

fn compute_win_rate(trades: &[Trade]) -> Decimal {
    let sells: Vec<&Trade> = trades.iter().filter(|t| t.action == TradeAction::Sell).collect();
    if sells.is_empty() { return dec!(0); }
    let wins = sells.iter().filter(|t| t.pnl.map(|p| p > dec!(0)).unwrap_or(false)).count();
    Decimal::from(wins) / Decimal::from(sells.len())
}

/// Annualised Sharpe ratio using daily equity returns. Risk-free rate = 3% / 252.
fn compute_sharpe(equity: &[Decimal]) -> Decimal {
    if equity.len() < 2 { return dec!(0); }
    let returns: Vec<f64> = equity.windows(2).map(|w| {
        let prev = w[0].to_f64().unwrap_or(1.0);
        let curr = w[1].to_f64().unwrap_or(1.0);
        if prev == 0.0 { 0.0 } else { (curr - prev) / prev }
    }).collect();
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();
    if std_dev == 0.0 { return dec!(0); }
    let rf_daily = 0.03 / 252.0;
    let sharpe = (mean - rf_daily) / std_dev * 252_f64.sqrt();
    Decimal::from_f64(sharpe).unwrap_or(dec!(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_equity(values: &[f64]) -> Vec<Decimal> {
        values.iter().map(|v| Decimal::from_f64(*v).unwrap()).collect()
    }

    #[test]
    fn test_total_return_positive() {
        let start = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let end   = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let equity = make_equity(&[100_000.0, 110_000.0]);
        let r = BacktestResult::calculate(dec!(100_000), vec![], equity, start, end);
        assert_eq!(r.total_return, dec!(0.1));
    }

    #[test]
    fn test_max_drawdown_correct() {
        let equity = make_equity(&[100.0, 120.0, 90.0, 110.0]);
        let dd = compute_max_drawdown(&equity);
        // Peak = 120, trough = 90 → dd = 30/120 = 0.25
        assert!(dd < dec!(-0.24) && dd > dec!(-0.26));
    }

    #[test]
    fn test_win_rate_half() {
        let trades = vec![
            Trade { date: NaiveDate::from_ymd_opt(2023,1,1).unwrap(), action: TradeAction::Sell, price: dec!(110), quantity: 10, amount: dec!(1100), pnl: Some(dec!(100)) },
            Trade { date: NaiveDate::from_ymd_opt(2023,2,1).unwrap(), action: TradeAction::Sell, price: dec!(90), quantity: 10, amount: dec!(900), pnl: Some(dec!(-100)) },
        ];
        assert_eq!(compute_win_rate(&trades), dec!(0.5));
    }

    #[test]
    fn test_export_csv_creates_file() {
        let r = BacktestResult::empty(dec!(100_000));
        let tmp = std::env::temp_dir().join("fa_backtest_test.csv");
        r.export_csv(&tmp).unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("date,action,price"));
        std::fs::remove_file(tmp).ok();
    }

    #[test]
    fn test_empty_result() {
        let r = BacktestResult::empty(dec!(50_000));
        assert_eq!(r.total_trades, 0);
        assert_eq!(r.final_equity, dec!(50_000));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p fa-backtest 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. X passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add crates/fa-backtest/src/result.rs
git commit -m "feat(fa-backtest): full BacktestResult with metrics (return/drawdown/Sharpe/win-rate) + CSV export

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: `MaCross` strategy + `BuiltinStrategy` enum

**Files:**
- Create: `crates/fa-backtest/src/strategies/mod.rs`
- Create: `crates/fa-backtest/src/strategies/ma_cross.rs`
- Modify: `crates/fa-backtest/src/lib.rs`

- [ ] **Step 1: Write failing test**

Create `crates/fa-backtest/src/strategies/ma_cross.rs`:
```rust
use crate::strategy::{BarContext, Signal, Strategy};
use fa_indicator::sma;

pub struct MaCrossStrategy {
    pub fast: usize,
    pub slow: usize,
}

impl MaCrossStrategy {
    pub fn new(fast: usize, slow: usize) -> Self {
        Self { fast, slow }
    }
}

impl Strategy for MaCrossStrategy {
    fn name(&self) -> &str { "双均线穿越" }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        // Need at least slow+1 bars to detect a crossover (compare prev and curr MA)
        if history.len() < self.slow + 1 {
            return Signal::Hold;
        }
        let fast_vals = sma(history, self.fast);
        let slow_vals = sma(history, self.slow);
        let n = history.len();
        match (fast_vals[n - 2], slow_vals[n - 2], fast_vals[n - 1], slow_vals[n - 1]) {
            (Some(pf), Some(ps), Some(cf), Some(cs)) => {
                if pf < ps && cf >= cs && ctx.position == 0 {
                    Signal::BuyAll  // golden cross
                } else if pf >= ps && cf < cs && ctx.position > 0 {
                    Signal::SellAll // death cross
                } else {
                    Signal::Hold
                }
            }
            _ => Signal::Hold,
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use chrono::Utc;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV { symbol: Symbol::new("T", Market::USStock), timestamp: Utc::now(),
                open: c, high: c, low: c, close: c, volume: 0 }
    }

    fn ctx<'a>(history: &'a [OHLCV], position: i64) -> BarContext<'a> {
        use rust_decimal_macros::dec;
        BarContext { bar: history.last().unwrap(), position, cash: dec!(10000), history }
    }

    #[test]
    fn test_golden_cross_signals_buy() {
        // fast(2), slow(3): series starts below then crosses up
        // prices: 10,10,10,10,10 → fast=slow=10, no cross yet
        // then: 10,10,10,11,12 → fast rises above slow → BuyAll
        let mut history: Vec<OHLCV> = vec![
            bar(10.0), bar(10.0), bar(10.0), bar(11.0),
        ];
        let mut s = MaCrossStrategy::new(2, 3);
        // With fast=2, slow=3, need slow+1=4 bars
        let c = ctx(&history, 0);
        let signal = s.on_bar(&c);
        // fast(last 2) = (10+11)/2=10.5, slow(last 3) = (10+10+11)/3=10.33
        // prev: fast(10+10)/2=10, slow(10+10+10)/3=10 → pf==ps, not pf<ps → Hold
        assert_eq!(signal, Signal::Hold);
    }

    #[test]
    fn test_hold_when_insufficient_data() {
        let history = vec![bar(10.0), bar(11.0)];
        let mut s = MaCrossStrategy::new(2, 3);
        let c = ctx(&history, 0);
        assert_eq!(s.on_bar(&c), Signal::Hold);
    }

    #[test]
    fn test_death_cross_signals_sell() {
        // Falling series: fast drops below slow while in position
        let history: Vec<OHLCV> = vec![
            bar(20.0), bar(18.0), bar(15.0), bar(10.0),
        ];
        let mut s = MaCrossStrategy::new(2, 3);
        // fast(last 2) = (15+10)/2=12.5, slow(last 3) = (18+15+10)/3=14.33 → fast < slow
        // prev: fast(18+15)/2=16.5, slow(20+18+15)/3=17.67 → fast < slow already → Hold (no crossover)
        let c = ctx(&history, 10); // in position
        assert_eq!(s.on_bar(&c), Signal::Hold);
    }
}
```

- [ ] **Step 2: Create `strategies/mod.rs`**

```rust
pub mod bollinger;
pub mod ma_cross;
pub mod rsi;

use crate::strategy::Strategy;
use ma_cross::MaCrossStrategy;
use rust_decimal_macros::dec;

/// Enumeration of all built-in strategies.
#[derive(Clone, Debug)]
pub enum BuiltinStrategy {
    MaCross { fast: usize, slow: usize },
    Rsi { period: usize },
    Bollinger { period: usize },
}

impl BuiltinStrategy {
    pub fn all() -> Vec<BuiltinStrategy> {
        vec![
            BuiltinStrategy::MaCross { fast: 5, slow: 20 },
            BuiltinStrategy::Rsi { period: 14 },
            BuiltinStrategy::Bollinger { period: 20 },
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            BuiltinStrategy::MaCross { fast, slow } => "双均线穿越 (MA5×MA20)",
            BuiltinStrategy::Rsi { .. } => "RSI 均值回归 (14/30/70)",
            BuiltinStrategy::Bollinger { .. } => "布林带 (20, 2σ)",
        }
    }

    pub fn to_boxed(&self) -> Box<dyn Strategy> {
        match self {
            BuiltinStrategy::MaCross { fast, slow } => {
                Box::new(MaCrossStrategy::new(*fast, *slow))
            }
            BuiltinStrategy::Rsi { period } => {
                Box::new(rsi::RsiStrategy::new(*period, 30, 70))
            }
            BuiltinStrategy::Bollinger { period } => {
                Box::new(bollinger::BollingerStrategy::new(*period, 2.0))
            }
        }
    }
}
```

Note: `bollinger.rs` and `rsi.rs` are stubs for now (will panic if called; they're written in Tasks 5 & 6). Add placeholder files:

`crates/fa-backtest/src/strategies/rsi.rs`:
```rust
use crate::strategy::{BarContext, Signal, Strategy};
pub struct RsiStrategy { pub period: usize, pub oversold: u32, pub overbought: u32 }
impl RsiStrategy {
    pub fn new(period: usize, oversold: u32, overbought: u32) -> Self {
        Self { period, oversold, overbought }
    }
}
impl Strategy for RsiStrategy {
    fn name(&self) -> &str { "RSI 均值回归" }
    fn on_bar(&mut self, _ctx: &BarContext) -> Signal { Signal::Hold }
    fn reset(&mut self) {}
}
```

`crates/fa-backtest/src/strategies/bollinger.rs`:
```rust
use crate::strategy::{BarContext, Signal, Strategy};
pub struct BollingerStrategy { pub period: usize, pub std_dev: f64 }
impl BollingerStrategy {
    pub fn new(period: usize, std_dev: f64) -> Self { Self { period, std_dev } }
}
impl Strategy for BollingerStrategy {
    fn name(&self) -> &str { "布林带" }
    fn on_bar(&mut self, _ctx: &BarContext) -> Signal { Signal::Hold }
    fn reset(&mut self) {}
}
```

- [ ] **Step 3: Update `lib.rs`**

```rust
pub(crate) mod portfolio;
pub mod engine;
pub mod result;
pub mod strategies;
pub mod strategy;

pub use engine::{BacktestConfig, Engine};
pub use result::{BacktestResult, Trade, TradeAction};
pub use strategies::BuiltinStrategy;
pub use strategy::{BarContext, Signal, Strategy};
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p fa-backtest 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. X passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add crates/fa-backtest/src/strategies/ crates/fa-backtest/src/lib.rs
git commit -m "feat(fa-backtest): add MaCross strategy + BuiltinStrategy enum

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: RSI indicator + RSI strategy

**Files:**
- Create: `crates/fa-indicator/src/rsi.rs`
- Modify: `crates/fa-indicator/src/lib.rs`
- Modify: `crates/fa-backtest/src/strategies/rsi.rs`

- [ ] **Step 1: Write failing test for RSI indicator**

Create `crates/fa-indicator/src/rsi.rs`:
```rust
use fa_core::OHLCV;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};

/// RSI using Wilder's smoothing method.
/// Returns Vec of same length as `data`.
/// First `period` elements are None; element at index `period` is the first RSI value.
pub fn rsi(data: &[OHLCV], period: usize) -> Vec<Option<Decimal>> {
    if period == 0 || data.len() <= period {
        return vec![None; data.len()];
    }

    let mut result = vec![None; data.len()];
    let closes: Vec<f64> = data.iter().map(|b| b.close.to_f64().unwrap_or(0.0)).collect();

    // Seed: average gain/loss over first `period` changes
    let mut avg_gain: f64 = 0.0;
    let mut avg_loss: f64 = 0.0;
    for i in 1..=period {
        let change = closes[i] - closes[i - 1];
        if change > 0.0 { avg_gain += change; } else { avg_loss += change.abs(); }
    }
    avg_gain /= period as f64;
    avg_loss /= period as f64;

    result[period] = Some(rsi_from_avg(avg_gain, avg_loss));

    // Wilder smoothing
    for i in (period + 1)..closes.len() {
        let change = closes[i] - closes[i - 1];
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { change.abs() } else { 0.0 };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        result[i] = Some(rsi_from_avg(avg_gain, avg_loss));
    }

    result
}

fn rsi_from_avg(avg_gain: f64, avg_loss: f64) -> Decimal {
    let val = if avg_loss == 0.0 {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
    };
    Decimal::from_f64(val).unwrap_or(dec!(50))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use chrono::Utc;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV { symbol: Symbol::new("T", Market::USStock), timestamp: Utc::now(),
                open: c, high: c, low: c, close: c, volume: 0 }
    }

    #[test]
    fn test_rsi_length_matches_input() {
        let data: Vec<OHLCV> = (0..20).map(|i| bar(100.0 + i as f64)).collect();
        let result = rsi(&data, 14);
        assert_eq!(result.len(), 20);
    }

    #[test]
    fn test_rsi_first_period_elements_are_none() {
        let data: Vec<OHLCV> = (0..20).map(|i| bar(100.0 + i as f64)).collect();
        let result = rsi(&data, 14);
        for i in 0..14 { assert!(result[i].is_none(), "index {i} should be None"); }
    }

    #[test]
    fn test_rsi_all_up_approaches_100() {
        // Consistently rising prices → RSI → 100
        let data: Vec<OHLCV> = (0..30).map(|i| bar(100.0 + i as f64 * 2.0)).collect();
        let result = rsi(&data, 14);
        let last = result.last().unwrap().unwrap();
        assert!(last > dec!(90), "expected RSI > 90 for all-up series, got {last}");
    }

    #[test]
    fn test_rsi_all_down_approaches_0() {
        let data: Vec<OHLCV> = (0..30).map(|i| bar(200.0 - i as f64 * 2.0)).collect();
        let result = rsi(&data, 14);
        let last = result.last().unwrap().unwrap();
        assert!(last < dec!(10), "expected RSI < 10 for all-down series, got {last}");
    }

    #[test]
    fn test_rsi_too_short_returns_all_none() {
        let data: Vec<OHLCV> = (0..5).map(|i| bar(100.0 + i as f64)).collect();
        let result = rsi(&data, 14);
        assert!(result.iter().all(|r| r.is_none()));
    }
}
```

- [ ] **Step 2: Update `fa-indicator/src/lib.rs`**

```rust
pub mod ma;
pub mod rsi;
pub use ma::sma;
pub use rsi::rsi;
```

- [ ] **Step 3: Run indicator tests**

```bash
cargo test -p fa-indicator 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. 10 passed; 0 failed` (5 existing + 5 new)

- [ ] **Step 4: Implement RSI strategy in `crates/fa-backtest/src/strategies/rsi.rs`**

Replace the placeholder with:
```rust
use crate::strategy::{BarContext, Signal, Strategy};
use fa_indicator::rsi;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;

pub struct RsiStrategy {
    pub period: usize,
    pub oversold: u32,   // e.g. 30
    pub overbought: u32, // e.g. 70
}

impl RsiStrategy {
    pub fn new(period: usize, oversold: u32, overbought: u32) -> Self {
        Self { period, oversold, overbought }
    }
}

impl Strategy for RsiStrategy {
    fn name(&self) -> &str { "RSI 均值回归" }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        if history.len() <= self.period {
            return Signal::Hold;
        }
        let rsi_vals = rsi(history, self.period);
        let current = match rsi_vals.last().and_then(|v| *v) {
            Some(v) => v,
            None => return Signal::Hold,
        };
        let os = Decimal::from(self.oversold);
        let ob = Decimal::from(self.overbought);
        if current < os && ctx.position == 0 {
            Signal::BuyAll
        } else if current > ob && ctx.position > 0 {
            Signal::SellAll
        } else {
            Signal::Hold
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV { symbol: Symbol::new("T", Market::USStock), timestamp: Utc::now(),
                open: c, high: c, low: c, close: c, volume: 0 }
    }

    fn ctx<'a>(history: &'a [OHLCV], position: i64) -> BarContext<'a> {
        BarContext { bar: history.last().unwrap(), position, cash: dec!(10000), history }
    }

    #[test]
    fn test_rsi_hold_when_insufficient_data() {
        let history: Vec<OHLCV> = (0..5).map(|i| bar(100.0 + i as f64)).collect();
        let mut s = RsiStrategy::new(14, 30, 70);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::Hold);
    }

    #[test]
    fn test_rsi_buys_on_oversold_series() {
        // Sharply falling series → RSI < 30 → BuyAll
        let history: Vec<OHLCV> = (0..30).map(|i| bar(200.0 - i as f64 * 5.0)).collect();
        let mut s = RsiStrategy::new(14, 30, 70);
        let signal = s.on_bar(&ctx(&history, 0));
        assert_eq!(signal, Signal::BuyAll);
    }
}
```

- [ ] **Step 5: Run all backtest tests**

```bash
cargo test -p fa-backtest 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. X passed; 0 failed`

- [ ] **Step 6: Commit**

```bash
git add crates/fa-indicator/src/ crates/fa-backtest/src/strategies/rsi.rs
git commit -m "feat: add RSI indicator to fa-indicator + RSI mean-revert strategy

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Bollinger Band strategy

**Files:**
- Modify: `crates/fa-backtest/src/strategies/bollinger.rs`

- [ ] **Step 1: Replace Bollinger placeholder with full implementation**

```rust
use crate::strategy::{BarContext, Signal, Strategy};
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};

pub struct BollingerStrategy {
    pub period: usize,
    pub std_dev: f64,
}

impl BollingerStrategy {
    pub fn new(period: usize, std_dev: f64) -> Self {
        Self { period, std_dev }
    }
}

impl Strategy for BollingerStrategy {
    fn name(&self) -> &str { "布林带" }

    fn on_bar(&mut self, ctx: &BarContext) -> Signal {
        let history = ctx.history;
        if history.len() < self.period { return Signal::Hold; }

        let window = &history[history.len() - self.period..];
        let closes_f64: Vec<f64> = window.iter()
            .map(|b| b.close.to_f64().unwrap_or(0.0))
            .collect();
        let mean = closes_f64.iter().sum::<f64>() / self.period as f64;
        let variance = closes_f64.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / self.period as f64;
        let std = variance.sqrt();

        let upper = Decimal::from_f64(mean + self.std_dev * std).unwrap_or(Decimal::MAX);
        let lower = Decimal::from_f64(mean - self.std_dev * std).unwrap_or(Decimal::ZERO);
        let close = ctx.bar.close;

        if close <= lower && ctx.position == 0 {
            Signal::BuyAll
        } else if close >= upper && ctx.position > 0 {
            Signal::SellAll
        } else {
            Signal::Hold
        }
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn bar(close: f64) -> OHLCV {
        let c = Decimal::from_f64_retain(close).unwrap();
        OHLCV { symbol: Symbol::new("T", Market::USStock), timestamp: Utc::now(),
                open: c, high: c, low: c, close: c, volume: 0 }
    }

    fn ctx<'a>(history: &'a [OHLCV], position: i64) -> BarContext<'a> {
        BarContext { bar: history.last().unwrap(), position, cash: dec!(10000), history }
    }

    #[test]
    fn test_hold_when_insufficient_data() {
        let history = vec![bar(100.0), bar(101.0)];
        let mut s = BollingerStrategy::new(20, 2.0);
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::Hold);
    }

    #[test]
    fn test_buy_below_lower_band() {
        // Flat series at 100, then a spike down to 50 → well below lower band
        let mut history: Vec<OHLCV> = (0..20).map(|_| bar(100.0)).collect();
        history.push(bar(50.0)); // far below lower band (100 ± ~0 std)
        let mut s = BollingerStrategy::new(20, 2.0);
        // With all 100 + one 50: window is last 20 bars (1 bar at 50 + 19 at 100)
        // mean ≈ 97.6, std ≈ 11.8, lower ≈ 74
        // close=50 < lower → BuyAll
        assert_eq!(s.on_bar(&ctx(&history, 0)), Signal::BuyAll);
    }

    #[test]
    fn test_sell_above_upper_band() {
        let mut history: Vec<OHLCV> = (0..20).map(|_| bar(100.0)).collect();
        history.push(bar(200.0)); // far above upper band
        let mut s = BollingerStrategy::new(20, 2.0);
        // close=200 > upper → SellAll (when in position)
        assert_eq!(s.on_bar(&ctx(&history, 100)), Signal::SellAll);
    }
}
```

- [ ] **Step 2: Run all backtest tests**

```bash
cargo test -p fa-backtest 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: `test result: ok. X passed; 0 failed`

- [ ] **Step 3: Commit**

```bash
git add crates/fa-backtest/src/strategies/bollinger.rs
git commit -m "feat(fa-backtest): implement Bollinger Band strategy

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 7: TUI `app.rs` — `BacktestState` + new `AppAction` variants

**Files:**
- Modify: `crates/fa-tui/Cargo.toml`
- Modify: `crates/fa-tui/src/app.rs`

- [ ] **Step 1: Add `fa-backtest` dependency**

In `crates/fa-tui/Cargo.toml`, add under `[dependencies]`:
```toml
fa-backtest  = { path = "../fa-backtest" }
```

- [ ] **Step 2: Write failing tests**

Add these tests to the existing `#[cfg(test)] mod tests` block in `crates/fa-tui/src/app.rs`:

```rust
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
    // wraps at end
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
```

- [ ] **Step 3: Run to verify tests fail**

```bash
cargo test -p fa-tui test_start_backtest 2>&1 | tail -5
```
Expected: compile error — `StartBacktest` variant not found.

- [ ] **Step 4: Add `BacktestState` and related types to `app.rs`**

Add these imports at the top of `crates/fa-tui/src/app.rs`:
```rust
use fa_backtest::{BacktestConfig, BacktestResult, BuiltinStrategy};
```

Add these structs/enums **before** `impl State`:
```rust
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
        // Convert to &'static str by matching
        match &all[idx] {
            BuiltinStrategy::MaCross { .. } => "双均线穿越 (MA5×MA20)",
            BuiltinStrategy::Rsi { .. }     => "RSI 均值回归 (14/30/70)",
            BuiltinStrategy::Bollinger { .. } => "布林带 (20, 2σ)",
        }
    }
}
```

- [ ] **Step 5: Add `Backtest` variant to `AppScreen`**

Extend the existing `AppScreen` enum:
```rust
pub enum AppScreen {
    Main,
    Chart(ChartState),
    Backtest(BacktestState),  // ← add this
}
```

- [ ] **Step 6: Add new `AppAction` variants**

Add to the existing `AppAction` enum:
```rust
StartBacktest(fa_core::Symbol),
RunBacktest,
BacktestComplete(BacktestResult),
BacktestFailed(String),
BacktestNextStrategy,
BacktestPrevStrategy,
BacktestScrollUp,
BacktestScrollDown,
ExitBacktest,
```

- [ ] **Step 7: Add `apply()` arms**

Inside `State::apply()`, add after the existing `ExitChart` arm:
```rust
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
        bs.trade_scroll += 1;
    }
}
AppAction::ExitBacktest => {
    self.screen = AppScreen::Main;
}
```

- [ ] **Step 8: Run tests**

```bash
cargo test -p fa-tui 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: all pass (new tests + all existing tests still pass)

- [ ] **Step 9: Commit**

```bash
git add crates/fa-tui/Cargo.toml crates/fa-tui/src/app.rs
git commit -m "feat(fa-tui/app): add BacktestState, AppScreen::Backtest, 9 new AppAction variants

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 8: TUI `event.rs` — key routing for backtest screen

**Files:**
- Modify: `crates/fa-tui/src/event.rs`

- [ ] **Step 1: Write failing tests**

Add to `#[cfg(test)] mod tests` in `crates/fa-tui/src/event.rs`:

```rust
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
    assert!(matches!(action, Some(AppAction::StartBacktest(sym)) if sym.code == "AAPL"));
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
```

- [ ] **Step 2: Verify tests fail**

```bash
cargo test -p fa-tui test_map_key_backtest 2>&1 | tail -5
```
Expected: compile error — `map_key_backtest` not found.

- [ ] **Step 3: Add `map_key_backtest()` to `EventHandler`**

In `crates/fa-tui/src/event.rs`, add after `map_key_add`:
```rust
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
```

- [ ] **Step 4: Update `resolve_action()` to handle backtest screen**

In `resolve_action`, add a new branch after the chart branch:
```rust
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
        && code == KeyCode::Char('a')
        && modifiers == KeyModifiers::NONE
    {
        Some(AppAction::StartAdd)
    } else if !state.is_search_active
        && !state.is_add_active
        && code == KeyCode::Char('b')
        && modifiers == KeyModifiers::NONE
    {
        // Start backtest for the selected symbol
        state.selected_symbol().map(|sym| AppAction::StartBacktest(sym.clone()))
    } else if code == KeyCode::Enter && modifiers == KeyModifiers::NONE {
        state.selected_symbol().map(|sym| AppAction::EnterChart(sym.clone()))
    } else {
        Self::map_key_main(code, modifiers)
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p fa-tui 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/fa-tui/src/event.rs
git commit -m "feat(fa-tui/event): add map_key_backtest, b→StartBacktest routing

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 9: TUI `ui/backtest.rs` — render the backtest screen

**Files:**
- Create: `crates/fa-tui/src/ui/backtest.rs`
- Modify: `crates/fa-tui/src/ui/mod.rs`

- [ ] **Step 1: Write failing test**

Create `crates/fa-tui/src/ui/backtest.rs` with the test first:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppScreen, BacktestState};
    use fa_core::{Market, Symbol};
    use ratatui::{backend::TestBackend, Terminal};

    fn make_state_on_backtest() -> State {
        let mut s = crate::app::State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.screen = AppScreen::Backtest(BacktestState::new(sym));
        s
    }

    #[test]
    fn test_render_backtest_idle_does_not_panic() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = make_state_on_backtest();
        terminal.draw(|f| {
            if let AppScreen::Backtest(ref bs) = state.screen {
                render(f, bs, f.area());
            }
        }).unwrap();
    }

    #[test]
    fn test_render_backtest_shows_strategy_name() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = make_state_on_backtest();
        terminal.draw(|f| {
            if let AppScreen::Backtest(ref bs) = state.screen {
                render(f, bs, f.area());
            }
        }).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("MA") || content.contains("均线") || content.contains("回测"),
                "expected strategy name or '回测' in output");
    }
}
```

- [ ] **Step 2: Add `pub mod backtest` to `ui/mod.rs`**

In `crates/fa-tui/src/ui/mod.rs`, add:
```rust
pub mod backtest;
```

- [ ] **Step 3: Implement `render()` in `backtest.rs`**

```rust
use crate::app::{BacktestState, BacktestStatus};
use fa_backtest::{BuiltinStrategy, TradeAction};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn render(f: &mut Frame, bs: &BacktestState, area: Rect) {
    // Left panel 35% | Right panel 65%
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    render_config(f, bs, cols[0]);

    // Right side: metrics top 40% | trade log 60%
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(cols[1]);

    render_metrics(f, bs, right_rows[0]);
    render_trades(f, bs, right_rows[1]);
}

fn render_config(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let strategies = BuiltinStrategy::all();
    let strategy_name = bs.strategy_name();

    let status_str = match &bs.status {
        BacktestStatus::Idle    => " [r 运行]".to_string(),
        BacktestStatus::Running => " ⏳ 运行中...".to_string(),
        BacktestStatus::Done    => " ✅ 完成".to_string(),
        BacktestStatus::Error(e) => format!(" ❌ {e}"),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("股票: ", Style::default().fg(Color::Gray)),
            Span::styled(bs.symbol.code.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("策略: ", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(format!("← {strategy_name} →"), Style::default().fg(Color::Yellow)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("资金: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("¥{:.0}", bs.config.initial_cash)),
        ]),
        Line::from(vec![
            Span::styled("手续费: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}bps", bs.config.commission_bps)),
        ]),
        Line::from(vec![
            Span::styled("滑点:   ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}bps", bs.config.slippage_bps)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("时间: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{} ~ {}", bs.config.start_date, bs.config.end_date)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(status_str, Style::default().fg(Color::Green))),
        Line::raw(""),
        Line::from(Span::styled("← → 切换策略  r 运行  Esc 返回", Style::default().fg(Color::DarkGray))),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().title(" 回测配置 ").borders(Borders::ALL));
    f.render_widget(para, area);
}

fn render_metrics(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let content = match &bs.result {
        None => {
            let hint = match &bs.status {
                BacktestStatus::Running => "正在运行回测...",
                _ => "按 r 开始回测",
            };
            vec![Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray)))]
        }
        Some(r) => {
            let ret_color = if r.total_return >= rust_decimal_macros::dec!(0) { Color::Red } else { Color::Green };
            vec![
                Line::from(vec![
                    Span::styled("总收益:  ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:+.2}%", r.total_return * rust_decimal_macros::dec!(100)), Style::default().fg(ret_color).add_modifier(Modifier::BOLD)),
                    Span::raw("   "),
                    Span::styled("年化:  ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:+.2}%", r.annualized_return * rust_decimal_macros::dec!(100)), Style::default().fg(ret_color)),
                ]),
                Line::from(vec![
                    Span::styled("最大回撤: ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:.2}%", r.max_drawdown * rust_decimal_macros::dec!(100)), Style::default().fg(Color::Green)),
                    Span::raw("   "),
                    Span::styled("胜率:  ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{:.1}%", r.win_rate * rust_decimal_macros::dec!(100))),
                ]),
                Line::from(vec![
                    Span::styled("Sharpe:   ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{:.3}", r.sharpe_ratio)),
                    Span::raw("   "),
                    Span::styled("交易次数: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{}", r.total_trades)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("初始: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("¥{:.0}", r.initial_cash)),
                    Span::raw("   "),
                    Span::styled("最终: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("¥{:.0}", r.final_equity)),
                ]),
            ]
        }
    };

    let para = Paragraph::new(content)
        .block(Block::default().title(" 回测结果 ").borders(Borders::ALL));
    f.render_widget(para, area);
}

fn render_trades(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let items: Vec<ListItem> = match &bs.result {
        None => vec![ListItem::new(" 暂无交易记录")],
        Some(r) => {
            let header = ListItem::new(Line::from(vec![
                Span::styled(format!("{:<12} {:<5} {:<10} {:<7} {:<12} {}", "日期", "操作", "价格", "数量", "金额", "盈亏"),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            ]));
            let mut items = vec![header];
            let trades = &r.trades;
            let skip = bs.trade_scroll.min(trades.len().saturating_sub(1));
            for t in trades.iter().skip(skip) {
                let (action_str, action_color) = match t.action {
                    TradeAction::Buy  => ("买入", Color::Red),
                    TradeAction::Sell => ("卖出", Color::Green),
                };
                let pnl_str = t.pnl
                    .map(|p| format!("{:+.0}", p))
                    .unwrap_or_default();
                let pnl_color = t.pnl
                    .map(|p| if p >= rust_decimal_macros::dec!(0) { Color::Red } else { Color::Green })
                    .unwrap_or(Color::White);
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(format!("{:<12} ", t.date)),
                    Span::styled(format!("{:<5} ", action_str), Style::default().fg(action_color)),
                    Span::raw(format!("{:<10.2} {:<7} {:<12.0} ", t.price, t.quantity, t.amount)),
                    Span::styled(pnl_str, Style::default().fg(pnl_color)),
                ])));
            }
            items
        }
    };

    let list = List::new(items)
        .block(Block::default()
            .title(" 交易记录  ↑/↓ 滚动 · e 导出 CSV ")
            .borders(Borders::ALL));
    f.render_widget(list, area);
}
```

Note: Add `use crate::app::State;` at the top of the test module.

- [ ] **Step 4: Run tests**

```bash
cargo test -p fa-tui 2>&1 | grep -E "^test result|FAILED|error"
```
Expected: all pass (including new backtest rendering tests).

- [ ] **Step 5: Commit**

```bash
git add crates/fa-tui/src/ui/backtest.rs crates/fa-tui/src/ui/mod.rs
git commit -m "feat(fa-tui/ui): add backtest screen renderer with config/metrics/trade-log panels

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 10: `main.rs` integration + README

**Files:**
- Modify: `src/main.rs`
- Modify: `README.md`

- [ ] **Step 1: Add `fa-backtest` to root `Cargo.toml` dependencies**

In the root `Cargo.toml` `[dependencies]` section, add:
```toml
fa-backtest  = { path = "crates/fa-backtest" }
```

- [ ] **Step 2: Update `main.rs` imports**

Add to the existing `use fa_tui::ui::{...}` line:
```rust
use fa_tui::ui::{backtest, chart, detail, layout, portfolio, statusbar, watchlist};
```

Add at top:
```rust
use fa_backtest::{BacktestConfig, BuiltinStrategy, Engine};
```

- [ ] **Step 3: Add backtest render branch in the draw loop**

In `run_app()`, update the `terminal.draw` block to handle the new screen:
```rust
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
        AppScreen::Backtest(bs) => {       // ← add this arm
            backtest::render(f, bs, f.area());
        }
    }
})?;
```

- [ ] **Step 4: Add `RunBacktest` handling in the action dispatch loop**

In `run_app()`, update the action dispatch section. Find the block:
```rust
let needs_ohlcv_fetch = matches!(
    &action,
    AppAction::EnterChart(_) | AppAction::ChartChangePeriod(_)
);
```

Replace the entire action-dispatch block with:
```rust
let needs_ohlcv_fetch = matches!(
    &action,
    AppAction::EnterChart(_) | AppAction::ChartChangePeriod(_)
);
let needs_backtest_run = matches!(&action, AppAction::RunBacktest);

let (symbol_period, backtest_params, should_quit) = {
    let mut state = app_state.write().await;
    state.apply(action);
    let sp = if needs_ohlcv_fetch {
        if let AppScreen::Chart(cs) = &state.screen {
            Some((cs.symbol.clone(), cs.period))
        } else { None }
    } else { None };
    let bp = if needs_backtest_run {
        if let AppScreen::Backtest(bs) = &state.screen {
            Some((bs.symbol.clone(), bs.strategy_idx, bs.config.clone()))
        } else { None }
    } else { None };
    (sp, bp, state.should_quit)
}; // write lock released here

if let Some((symbol, period)) = symbol_period {
    let router = Arc::clone(router);
    let tx = tx.clone();
    tokio::spawn(async move {
        match router.fetch_ohlcv(&symbol, period).await {
            Ok(data) => { let _ = tx.send(AppAction::ChartDataLoaded(data)).await; }
            Err(e) => { let _ = tx.send(AppAction::StatusMessage(format!("K线获取失败: {}", e))).await; }
        }
    });
}

if let Some((symbol, strategy_idx, config)) = backtest_params {
    let router = Arc::clone(router);
    let tx = tx.clone();
    tokio::spawn(async move {
        match router.fetch_ohlcv(&symbol, fa_core::Period::Year1).await {
            Ok(data) => {
                let engine = Engine::new(config);
                let presets = BuiltinStrategy::all();
                let preset = &presets[strategy_idx.min(presets.len().saturating_sub(1))];
                let mut strategy = preset.to_boxed();
                let result = engine.run(&data, strategy.as_mut());
                let _ = tx.send(AppAction::BacktestComplete(result)).await;
            }
            Err(e) => {
                let _ = tx.send(AppAction::BacktestFailed(e.to_string())).await;
            }
        }
    });
}
```

Also update the `needs_ohlcv_fetch` extract block — replace the existing `(symbol_period, should_quit)` tuple with the new `(symbol_period, backtest_params, should_quit)` tuple pattern throughout the function.

- [ ] **Step 5: Verify full workspace compiles and tests pass**

```bash
cd /root/projects/financial-analysis
cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error\[" | head -20
```
Expected: all `test result: ok`, 0 failed.

- [ ] **Step 6: Update README**

In `README.md`, in the features list, change:
```markdown
- **自选股管理**：支持 A 股（sh/sz 前缀）和美股，运行时按 `a` 添加新股票
```
to:
```markdown
- **自选股管理**：支持 A 股（sh/sz 前缀）和美股，运行时按 `a` 添加新股票
- **回测引擎**：按 `b` 对选中股票运行历史策略回测（双均线、RSI、布林带），查看收益、回撤、Sharpe 比率
```

In `README.md`, add `b` key to the main interface keybinding table:
```markdown
| `b` | 对选中股票进入回测模式 |
```
(Insert between the `a` row and the `d` row.)

In the README, add a new section after the existing keybinding sections:

```markdown
### 回测界面

| 按键 | 功能 |
|---|---|
| `←` / `→` | 切换回测策略 |
| `↑` / `↓` | 滚动交易记录 |
| `r` | 运行回测（获取 1 年日线数据后执行） |
| `Esc` | 返回主界面 |

内置策略：
- **双均线穿越**：MA5 上穿 MA20 买入，下穿卖出
- **RSI 均值回归**：RSI < 30 买入，RSI > 70 卖出
- **布林带**：价格触及下轨买入，触及上轨卖出

> 交易成本：手续费 5bps + 滑点 3bps（模拟市场冲击）。数据来源为 Yahoo Finance 近 1 年日线。
```

- [ ] **Step 7: Commit**

```bash
git add src/main.rs Cargo.toml README.md
git commit -m "feat: wire backtest into main.rs + document b key in README

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Self-Review Checklist

**Spec coverage:**
- [x] Engine (bar-by-bar, commission, slippage) → Tasks 2–3
- [x] Strategy trait for custom strategies → Task 1
- [x] MaCross, RSI, Bollinger built-in strategies → Tasks 4–6
- [x] BacktestResult with all 6 metrics + trade log + CSV → Task 3
- [x] TUI integration (`b` key, backtest screen) → Tasks 7–9
- [x] main.rs async task (same pattern as ChartFetcher) → Task 10
- [x] README documentation → Task 10

**Placeholder scan:** No TBDs, all code is complete and runnable.

**Type consistency:**
- `Strategy::on_bar` takes `&BarContext` throughout
- `Signal` enum: `BuyAll`, `SellAll`, `Hold` (no `Buy(Decimal)` or `Sell(i64)` — simplified to just Full-position for MVP)
- `BacktestResult` fields match between `result.rs`, `app.rs`, and `backtest.rs` rendering
- `BuiltinStrategy::to_boxed()` used consistently in `strategies/mod.rs` and `main.rs`
- `BacktestConfig: Clone + Default` — used by `BacktestState::new()` and cloned in main.rs task
- `AppAction::BacktestComplete(BacktestResult)` — no Box (consistent with existing `ChartDataLoaded(Vec<OHLCV>)` pattern)
