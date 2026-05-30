# Financial Analysis TUI — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 构建 Rust + Ratatui 股票金融分析系统 Phase 1：实时行情监控 + 持仓管理 TUI，支持美股、A 股、加密货币多市场。

**Architecture:** Cargo Workspace 组织代码（fa-core / fa-data / fa-tui），Tokio Actor 模型驱动数据管道。DataFetcher Task 每 30s 轮询数据源，写入 `Arc<RwLock<State>>`，TUI Renderer 以 16ms tick 读取并刷新，EventHandler Task 将键盘事件转换为 AppAction 驱动状态变更。

**Tech Stack:** Rust 1.75+, Ratatui 0.28, Crossterm 0.28, Tokio 1.x, Reqwest 0.12, rust_decimal 1.x, Chrono 0.4, Thiserror 1.x, Serde/serde_json, Mockito 1.x

---

## File Map

```
financial-analysis/
├── Cargo.toml                                  [CREATE] workspace 根配置
├── src/
│   ├── main.rs                                 [CREATE] 入口，组装 Actor 和 TUI
│   └── config.rs                               [CREATE] 配置文件加载
├── crates/
│   ├── fa-core/
│   │   ├── Cargo.toml                          [CREATE]
│   │   └── src/
│   │       ├── lib.rs                          [CREATE]
│   │       ├── error.rs                        [CREATE] DataError
│   │       ├── market.rs                       [CREATE] Market enum
│   │       ├── symbol.rs                       [CREATE] Symbol struct
│   │       ├── quote.rs                        [CREATE] Quote struct
│   │       ├── ohlcv.rs                        [CREATE] OHLCV, Period
│   │       ├── portfolio.rs                    [CREATE] Portfolio, Position, PnL
│   │       └── provider.rs                    [CREATE] DataProvider trait
│   ├── fa-data/
│   │   ├── Cargo.toml                          [CREATE]
│   │   └── src/
│   │       ├── lib.rs                          [CREATE]
│   │       ├── cache.rs                        [CREATE] InMemoryCache with TTL
│   │       ├── yahoo.rs                        [CREATE] Yahoo Finance provider
│   │       ├── sina.rs                         [CREATE] Sina Finance provider (A股)
│   │       ├── csv.rs                          [CREATE] CSV/JSON 本地导入
│   │       └── router.rs                       [CREATE] ProviderRouter (auto-select + fallback)
│   └── fa-tui/
│       ├── Cargo.toml                          [CREATE]
│       └── src/
│           ├── lib.rs                          [CREATE]
│           ├── app.rs                          [CREATE] AppState, State, AppAction
│           ├── event.rs                        [CREATE] EventHandler (keyboard → AppAction)
│           └── ui/
│               ├── mod.rs                      [CREATE]
│               ├── layout.rs                   [CREATE] 根布局，组装所有面板
│               ├── watchlist.rs                [CREATE] 自选股面板
│               ├── portfolio.rs                [CREATE] 持仓面板
│               ├── detail.rs                   [CREATE] 个股详情面板
│               └── statusbar.rs                [CREATE] 状态栏
└── fixtures/                                   [CREATE] 测试数据
    ├── yahoo_quote_aapl.json
    ├── sina_quote_sh600519.txt
    └── sample_portfolio.csv
```

---

## Task 1: Cargo Workspace 初始化

**Files:**
- Create: `Cargo.toml`
- Create: `crates/fa-core/Cargo.toml`
- Create: `crates/fa-data/Cargo.toml`
- Create: `crates/fa-tui/Cargo.toml`
- Create: `src/main.rs` (stub)
- Create: `crates/fa-core/src/lib.rs` (stub)
- Create: `crates/fa-data/src/lib.rs` (stub)
- Create: `crates/fa-tui/src/lib.rs` (stub)

- [ ] **Step 1: 创建 workspace 根 Cargo.toml**

```toml
# Cargo.toml
[workspace]
members = [".", "crates/fa-core", "crates/fa-data", "crates/fa-tui"]
resolver = "2"

[workspace.dependencies]
# TUI
ratatui = "0.28"
crossterm = "0.28"

# Async
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "1"
anyhow = "1"

# Time & Finance
chrono = { version = "0.4", features = ["serde"] }
rust_decimal = { version = "1", features = ["serde-with-str"] }
rust_decimal_macros = "1"

# Config
toml = "0.8"

# Testing
mockito = "1"

[package]
name = "financial-analysis"
version = "0.1.0"
edition = "2021"

[dependencies]
fa-core = { path = "crates/fa-core" }
fa-data = { path = "crates/fa-data" }
fa-tui  = { path = "crates/fa-tui" }
tokio   = { workspace = true }
anyhow  = { workspace = true }
toml    = { workspace = true }
serde   = { workspace = true }
```

- [ ] **Step 2: 创建 fa-core Cargo.toml**

```toml
# crates/fa-core/Cargo.toml
[package]
name = "fa-core"
version = "0.1.0"
edition = "2021"

[dependencies]
thiserror       = { workspace = true }
async-trait     = { workspace = true }
serde           = { workspace = true }
chrono          = { workspace = true }
rust_decimal    = { workspace = true }
rust_decimal_macros = { workspace = true }
```

- [ ] **Step 3: 创建 fa-data Cargo.toml**

```toml
# crates/fa-data/Cargo.toml
[package]
name = "fa-data"
version = "0.1.0"
edition = "2021"

[dependencies]
fa-core      = { path = "../fa-core" }
reqwest      = { workspace = true }
serde        = { workspace = true }
serde_json   = { workspace = true }
tokio        = { workspace = true }
async-trait  = { workspace = true }
thiserror    = { workspace = true }
chrono       = { workspace = true }
rust_decimal = { workspace = true }
rust_decimal_macros = { workspace = true }

[dev-dependencies]
mockito = { workspace = true }
tokio   = { workspace = true }
```

- [ ] **Step 4: 创建 fa-tui Cargo.toml**

```toml
# crates/fa-tui/Cargo.toml
[package]
name = "fa-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
fa-core    = { path = "../fa-core" }
ratatui    = { workspace = true }
crossterm  = { workspace = true }
tokio      = { workspace = true }
async-trait = { workspace = true }
chrono     = { workspace = true }
rust_decimal = { workspace = true }
```

- [ ] **Step 5: 创建各 crate stub lib.rs 和 main.rs stub**

```rust
// crates/fa-core/src/lib.rs
pub mod error;
pub mod market;
pub mod symbol;
pub mod quote;
pub mod ohlcv;
pub mod portfolio;
pub mod provider;

pub use error::DataError;
pub use market::Market;
pub use symbol::Symbol;
pub use quote::Quote;
pub use ohlcv::{OHLCV, Period};
pub use portfolio::{Portfolio, Position};
pub use provider::DataProvider;
```

```rust
// crates/fa-data/src/lib.rs
pub mod cache;
pub mod yahoo;
pub mod sina;
pub mod csv;
pub mod router;
```

```rust
// crates/fa-tui/src/lib.rs
pub mod app;
pub mod event;
pub mod ui;
```

```rust
// src/main.rs
fn main() {
    println!("Financial Analysis TUI - stub");
}
```

- [ ] **Step 6: 验证 workspace 编译**

```bash
cargo check --workspace
```

Expected: 各 crate 成功通过类型检查（可有 warning，不应有 error）

- [ ] **Step 7: 提交**

```bash
git add -A
git commit -m "feat: initialize cargo workspace with fa-core, fa-data, fa-tui crates"
```

---

## Task 2: fa-core — DataError & Market

**Files:**
- Create: `crates/fa-core/src/error.rs`
- Create: `crates/fa-core/src/market.rs`

- [ ] **Step 1: 写失败测试**

在 `crates/fa-core/src/market.rs` 底部：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_display() {
        assert_eq!(Market::USStock.to_string(), "US");
        assert_eq!(Market::AShare.to_string(), "A股");
        assert_eq!(Market::HKStock.to_string(), "HK");
        assert_eq!(Market::Crypto.to_string(), "Crypto");
    }

    #[test]
    fn test_market_from_str() {
        assert_eq!("us".parse::<Market>().unwrap(), Market::USStock);
        assert_eq!("a_share".parse::<Market>().unwrap(), Market::AShare);
        assert!("invalid".parse::<Market>().is_err());
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p fa-core market 2>&1 | head -20
```

Expected: `FAILED` — market module/types not yet defined

- [ ] **Step 3: 实现 error.rs**

```rust
// crates/fa-core/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    #[error("Symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },

    #[error("Market not supported: {market}")]
    MarketNotSupported { market: String },

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),
}
```

- [ ] **Step 4: 实现 market.rs**

```rust
// crates/fa-core/src/market.rs
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use crate::DataError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    USStock,
    AShare,
    HKStock,
    Crypto,
    Forex,
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Market::USStock => write!(f, "US"),
            Market::AShare  => write!(f, "A股"),
            Market::HKStock => write!(f, "HK"),
            Market::Crypto  => write!(f, "Crypto"),
            Market::Forex   => write!(f, "Forex"),
        }
    }
}

impl FromStr for Market {
    type Err = DataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "us" | "us_stock" | "nasdaq" | "nyse" => Ok(Market::USStock),
            "a_share" | "ashare" | "cn" | "china" => Ok(Market::AShare),
            "hk" | "hk_stock" | "hkex"            => Ok(Market::HKStock),
            "crypto" | "btc" | "eth"               => Ok(Market::Crypto),
            "forex" | "fx"                         => Ok(Market::Forex),
            _ => Err(DataError::Parse(format!("Unknown market: {s}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_display() {
        assert_eq!(Market::USStock.to_string(), "US");
        assert_eq!(Market::AShare.to_string(), "A股");
        assert_eq!(Market::HKStock.to_string(), "HK");
        assert_eq!(Market::Crypto.to_string(), "Crypto");
    }

    #[test]
    fn test_market_from_str() {
        assert_eq!("us".parse::<Market>().unwrap(), Market::USStock);
        assert_eq!("a_share".parse::<Market>().unwrap(), Market::AShare);
        assert_eq!("hk".parse::<Market>().unwrap(), Market::HKStock);
        assert!("invalid".parse::<Market>().is_err());
    }
}
```

- [ ] **Step 5: 运行测试，确认通过**

```bash
cargo test -p fa-core market -- --nocapture
```

Expected: `test tests::test_market_display ... ok` / `test tests::test_market_from_str ... ok`

- [ ] **Step 6: 提交**

```bash
git add crates/fa-core/src/error.rs crates/fa-core/src/market.rs
git commit -m "feat(fa-core): add DataError and Market enum"
```

---

## Task 3: fa-core — Symbol & Quote

**Files:**
- Create: `crates/fa-core/src/symbol.rs`
- Create: `crates/fa-core/src/quote.rs`

- [ ] **Step 1: 写失败测试（symbol.rs 和 quote.rs 尾部）**

对 symbol：
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Market;

    #[test]
    fn test_yahoo_ticker_a_share() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.yahoo_ticker(), "600519.SS");
        let s2 = Symbol::new("000001", Market::AShare);
        assert_eq!(s2.yahoo_ticker(), "000001.SZ");
    }

    #[test]
    fn test_yahoo_ticker_us() {
        let s = Symbol::new("AAPL", Market::USStock);
        assert_eq!(s.yahoo_ticker(), "AAPL");
    }

    #[test]
    fn test_sina_ticker() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.sina_ticker(), "sh600519");
        let s2 = Symbol::new("000001", Market::AShare);
        assert_eq!(s2.sina_ticker(), "sz000001");
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p fa-core symbol 2>&1 | head -10
```

Expected: FAIL — Symbol not defined

- [ ] **Step 3: 实现 symbol.rs**

```rust
// crates/fa-core/src/symbol.rs
use serde::{Deserialize, Serialize};
use crate::Market;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    pub code: String,
    pub market: Market,
    pub name: Option<String>,
}

impl Symbol {
    pub fn new(code: impl Into<String>, market: Market) -> Self {
        Self { code: code.into(), market, name: None }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Ticker format for Yahoo Finance API
    pub fn yahoo_ticker(&self) -> String {
        match &self.market {
            Market::AShare => {
                if self.code.starts_with('6') {
                    format!("{}.SS", self.code) // Shanghai
                } else {
                    format!("{}.SZ", self.code) // Shenzhen
                }
            }
            Market::HKStock => format!("{:0>4}.HK", self.code),
            _ => self.code.clone(),
        }
    }

    /// Ticker format for Sina Finance API (A股)
    pub fn sina_ticker(&self) -> String {
        match &self.market {
            Market::AShare => {
                if self.code.starts_with('6') {
                    format!("sh{}", self.code)
                } else {
                    format!("sz{}", self.code)
                }
            }
            _ => self.code.clone(),
        }
    }

    pub fn display_code(&self) -> String {
        self.code.clone()
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{} ({})", self.code, name)
        } else {
            write!(f, "{}", self.code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Market;

    #[test]
    fn test_yahoo_ticker_a_share() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.yahoo_ticker(), "600519.SS");
        let s2 = Symbol::new("000001", Market::AShare);
        assert_eq!(s2.yahoo_ticker(), "000001.SZ");
    }

    #[test]
    fn test_yahoo_ticker_us() {
        let s = Symbol::new("AAPL", Market::USStock);
        assert_eq!(s.yahoo_ticker(), "AAPL");
    }

    #[test]
    fn test_sina_ticker() {
        let s = Symbol::new("600519", Market::AShare);
        assert_eq!(s.sina_ticker(), "sh600519");
        let s2 = Symbol::new("000001", Market::AShare);
        assert_eq!(s2.sina_ticker(), "sz000001");
    }

    #[test]
    fn test_with_name() {
        let s = Symbol::new("AAPL", Market::USStock).with_name("Apple Inc.");
        assert_eq!(s.name.as_deref(), Some("Apple Inc."));
        assert_eq!(s.to_string(), "AAPL (Apple Inc.)");
    }
}
```

- [ ] **Step 4: 实现 quote.rs**

```rust
// crates/fa-core/src/quote.rs
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::Symbol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub symbol: Symbol,
    pub price: Decimal,
    pub change: Decimal,
    pub change_pct: Decimal,
    pub open: Option<Decimal>,
    pub high: Option<Decimal>,
    pub low: Option<Decimal>,
    pub volume: Option<u64>,
    pub market_cap: Option<Decimal>,
    pub pe_ratio: Option<Decimal>,
    pub week_52_high: Option<Decimal>,
    pub week_52_low: Option<Decimal>,
    pub name: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl Quote {
    pub fn is_positive(&self) -> bool {
        self.change >= Decimal::ZERO
    }

    pub fn change_sign(&self) -> &'static str {
        if self.change >= Decimal::ZERO { "+" } else { "" }
    }

    /// Format: "+1.25 (+0.68%)"
    pub fn change_display(&self) -> String {
        format!(
            "{}{:.2} ({}{:.2}%)",
            self.change_sign(), self.change,
            self.change_sign(), self.change_pct
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Market, Symbol};
    use rust_decimal_macros::dec;

    fn make_quote(price: Decimal, change: Decimal, change_pct: Decimal) -> Quote {
        Quote {
            symbol: Symbol::new("AAPL", Market::USStock),
            price,
            change,
            change_pct,
            open: None,
            high: None,
            low: None,
            volume: None,
            market_cap: None,
            pe_ratio: None,
            week_52_high: None,
            week_52_low: None,
            name: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_positive_change() {
        let q = make_quote(dec!(185.20), dec!(2.20), dec!(1.20));
        assert!(q.is_positive());
        assert_eq!(q.change_sign(), "+");
        assert_eq!(q.change_display(), "+2.20 (+1.20%)");
    }

    #[test]
    fn test_negative_change() {
        let q = make_quote(dec!(245.80), dec!(-2.10), dec!(-0.85));
        assert!(!q.is_positive());
        assert_eq!(q.change_sign(), "");
        assert_eq!(q.change_display(), "-2.10 (-0.85%)");
    }
}
```

- [ ] **Step 5: 运行测试**

```bash
cargo test -p fa-core symbol quote -- --nocapture
```

Expected: 所有测试通过

- [ ] **Step 6: 提交**

```bash
git add crates/fa-core/src/symbol.rs crates/fa-core/src/quote.rs
git commit -m "feat(fa-core): add Symbol and Quote types"
```

---

## Task 4: fa-core — OHLCV, Period & Portfolio

**Files:**
- Create: `crates/fa-core/src/ohlcv.rs`
- Create: `crates/fa-core/src/portfolio.rs`

- [ ] **Step 1: 写失败测试（portfolio.rs 尾部，最核心的 PnL 计算）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Market, Symbol};
    use rust_decimal_macros::dec;

    fn aapl_position() -> Position {
        Position {
            symbol: Symbol::new("AAPL", Market::USStock),
            quantity: dec!(10),
            cost_basis: dec!(1800.00),
        }
    }

    #[test]
    fn test_market_value() {
        let pos = aapl_position();
        assert_eq!(pos.market_value(dec!(185.20)), dec!(1852.00));
    }

    #[test]
    fn test_pnl_positive() {
        let pos = aapl_position();
        assert_eq!(pos.pnl(dec!(185.20)), dec!(52.00));
    }

    #[test]
    fn test_pnl_negative() {
        let pos = aapl_position();
        assert_eq!(pos.pnl(dec!(170.00)), dec!(-100.00));
    }

    #[test]
    fn test_pnl_pct() {
        let pos = aapl_position();
        // 52/1800 * 100 = 2.888...%
        let pct = pos.pnl_pct(dec!(185.20));
        assert!(pct > dec!(2.88) && pct < dec!(2.90));
    }

    #[test]
    fn test_cost_per_share() {
        let pos = aapl_position();
        assert_eq!(pos.cost_per_share(), dec!(180.00));
    }

    #[test]
    fn test_portfolio_total_cost() {
        let p = Portfolio {
            positions: vec![
                aapl_position(),
                Position {
                    symbol: Symbol::new("TSLA", Market::USStock),
                    quantity: dec!(5),
                    cost_basis: dec!(1100.00),
                },
            ],
        };
        assert_eq!(p.total_cost(), dec!(2900.00));
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p fa-core portfolio 2>&1 | head -10
```

- [ ] **Step 3: 实现 ohlcv.rs**

```rust
// crates/fa-core/src/ohlcv.rs
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::Symbol;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Period {
    Day1,
    Week1,
    Month1,
    Month3,
    Month6,
    Year1,
    Year5,
}

impl Period {
    /// Yahoo Finance range parameter
    pub fn yahoo_range(&self) -> &'static str {
        match self {
            Period::Day1   => "1d",
            Period::Week1  => "5d",
            Period::Month1 => "1mo",
            Period::Month3 => "3mo",
            Period::Month6 => "6mo",
            Period::Year1  => "1y",
            Period::Year5  => "5y",
        }
    }

    /// Yahoo Finance interval parameter
    pub fn yahoo_interval(&self) -> &'static str {
        match self {
            Period::Day1  => "5m",
            Period::Week1 => "1h",
            _             => "1d",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OHLCV {
    pub symbol: Symbol,
    pub timestamp: DateTime<Utc>,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: u64,
}
```

- [ ] **Step 4: 实现 portfolio.rs**

```rust
// crates/fa-core/src/portfolio.rs
use std::collections::HashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::Symbol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    pub quantity: Decimal,
    pub cost_basis: Decimal, // total cost (quantity * avg_price)
}

impl Position {
    pub fn market_value(&self, current_price: Decimal) -> Decimal {
        self.quantity * current_price
    }

    pub fn pnl(&self, current_price: Decimal) -> Decimal {
        self.market_value(current_price) - self.cost_basis
    }

    pub fn pnl_pct(&self, current_price: Decimal) -> Decimal {
        if self.cost_basis.is_zero() {
            Decimal::ZERO
        } else {
            (self.pnl(current_price) / self.cost_basis) * Decimal::from(100)
        }
    }

    pub fn cost_per_share(&self) -> Decimal {
        if self.quantity.is_zero() {
            Decimal::ZERO
        } else {
            self.cost_basis / self.quantity
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Portfolio {
    pub positions: Vec<Position>,
}

impl Portfolio {
    pub fn total_cost(&self) -> Decimal {
        self.positions.iter().map(|p| p.cost_basis).sum()
    }

    /// prices: symbol.code → current price
    pub fn total_market_value(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        self.positions.iter().map(|p| {
            prices.get(&p.symbol.code)
                .copied()
                .map(|price| p.market_value(price))
                .unwrap_or(p.cost_basis)
        }).sum()
    }

    pub fn total_pnl(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        self.total_market_value(prices) - self.total_cost()
    }

    pub fn total_pnl_pct(&self, prices: &HashMap<String, Decimal>) -> Decimal {
        let cost = self.total_cost();
        if cost.is_zero() {
            Decimal::ZERO
        } else {
            (self.total_pnl(prices) / cost) * Decimal::from(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Market, Symbol};
    use rust_decimal_macros::dec;

    fn aapl_position() -> Position {
        Position {
            symbol: Symbol::new("AAPL", Market::USStock),
            quantity: dec!(10),
            cost_basis: dec!(1800.00),
        }
    }

    #[test]
    fn test_market_value() {
        let pos = aapl_position();
        assert_eq!(pos.market_value(dec!(185.20)), dec!(1852.00));
    }

    #[test]
    fn test_pnl_positive() {
        let pos = aapl_position();
        assert_eq!(pos.pnl(dec!(185.20)), dec!(52.00));
    }

    #[test]
    fn test_pnl_negative() {
        let pos = aapl_position();
        assert_eq!(pos.pnl(dec!(170.00)), dec!(-100.00));
    }

    #[test]
    fn test_pnl_pct() {
        let pos = aapl_position();
        let pct = pos.pnl_pct(dec!(185.20));
        assert!(pct > dec!(2.88) && pct < dec!(2.90));
    }

    #[test]
    fn test_cost_per_share() {
        let pos = aapl_position();
        assert_eq!(pos.cost_per_share(), dec!(180.00));
    }

    #[test]
    fn test_portfolio_total_cost() {
        let p = Portfolio {
            positions: vec![
                aapl_position(),
                Position {
                    symbol: Symbol::new("TSLA", Market::USStock),
                    quantity: dec!(5),
                    cost_basis: dec!(1100.00),
                },
            ],
        };
        assert_eq!(p.total_cost(), dec!(2900.00));
    }

    #[test]
    fn test_portfolio_total_pnl() {
        let p = Portfolio {
            positions: vec![aapl_position()],
        };
        let mut prices = HashMap::new();
        prices.insert("AAPL".to_string(), dec!(185.20));
        assert_eq!(p.total_pnl(&prices), dec!(52.00));
    }
}
```

- [ ] **Step 5: 实现 provider.rs**

```rust
// crates/fa-core/src/provider.rs
use async_trait::async_trait;
use crate::{DataError, Market, OHLCV, Period, Quote, Symbol};

#[async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError>;
    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError>;
    fn name(&self) -> &'static str;
    fn supports(&self, market: &Market) -> bool;
}
```

- [ ] **Step 6: 运行所有 fa-core 测试**

```bash
cargo test -p fa-core -- --nocapture
```

Expected: 全部通过（market, symbol, quote, portfolio 共 ~12 个测试）

- [ ] **Step 7: 提交**

```bash
git add crates/fa-core/src/
git commit -m "feat(fa-core): add OHLCV, Period, Portfolio, Position, DataProvider trait"
```

---

## Task 5: fa-data — InMemoryCache

**Files:**
- Create: `crates/fa-data/src/cache.rs`
- Modify: `crates/fa-data/src/lib.rs`

- [ ] **Step 1: 写失败测试**

```rust
// 在 cache.rs 底部
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread::sleep;

    #[test]
    fn test_cache_hit() {
        let mut cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("AAPL", "price_data".to_string());
        assert_eq!(cache.get("AAPL"), Some("price_data".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = InMemoryCache::<String>::new(Duration::from_secs(60));
        assert_eq!(cache.get("TSLA"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let mut cache = InMemoryCache::new(Duration::from_millis(50));
        cache.set("BTC", "value".to_string());
        sleep(Duration::from_millis(100));
        assert_eq!(cache.get("BTC"), None);
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("AAPL", "v1".to_string());
        cache.set("AAPL", "v2".to_string());
        assert_eq!(cache.get("AAPL"), Some("v2".to_string()));
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p fa-data cache 2>&1 | head -10
```

- [ ] **Step 3: 实现 cache.rs**

```rust
// crates/fa-data/src/cache.rs
use std::collections::HashMap;
use std::time::{Duration, Instant};

struct CacheEntry<T> {
    value: T,
    expires_at: Instant,
}

pub struct InMemoryCache<T: Clone> {
    entries: HashMap<String, CacheEntry<T>>,
    ttl: Duration,
}

impl<T: Clone> InMemoryCache<T> {
    pub fn new(ttl: Duration) -> Self {
        Self { entries: HashMap::new(), ttl }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        self.entries.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: impl Into<String>, value: T) {
        self.entries.insert(key.into(), CacheEntry {
            value,
            expires_at: Instant::now() + self.ttl,
        });
    }

    pub fn invalidate(&mut self, key: &str) {
        self.entries.remove(key);
    }

    pub fn clear_expired(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_cache_hit() {
        let mut cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("AAPL", "price_data".to_string());
        assert_eq!(cache.get("AAPL"), Some("price_data".to_string()));
    }

    #[test]
    fn test_cache_miss() {
        let cache = InMemoryCache::<String>::new(Duration::from_secs(60));
        assert_eq!(cache.get("TSLA"), None);
    }

    #[test]
    fn test_cache_expiry() {
        let mut cache = InMemoryCache::new(Duration::from_millis(50));
        cache.set("BTC", "value".to_string());
        sleep(Duration::from_millis(100));
        assert_eq!(cache.get("BTC"), None);
    }

    #[test]
    fn test_cache_overwrite() {
        let mut cache = InMemoryCache::new(Duration::from_secs(60));
        cache.set("AAPL", "v1".to_string());
        cache.set("AAPL", "v2".to_string());
        assert_eq!(cache.get("AAPL"), Some("v2".to_string()));
    }
}
```

- [ ] **Step 4: 运行测试**

```bash
cargo test -p fa-data cache -- --nocapture
```

Expected: 4 个测试全部通过

- [ ] **Step 5: 提交**

```bash
git add crates/fa-data/src/cache.rs crates/fa-data/src/lib.rs
git commit -m "feat(fa-data): add InMemoryCache with TTL"
```

---

## Task 6: fa-data — Yahoo Finance Provider

**Files:**
- Create: `crates/fa-data/src/yahoo.rs`
- Create: `fixtures/yahoo_quote_aapl.json`

- [ ] **Step 1: 创建 fixture 文件**

```json
// fixtures/yahoo_quote_aapl.json
{
  "quoteResponse": {
    "result": [{
      "symbol": "AAPL",
      "longName": "Apple Inc.",
      "regularMarketPrice": 185.20,
      "regularMarketChange": 2.20,
      "regularMarketChangePercent": 1.20,
      "regularMarketOpen": 183.50,
      "regularMarketDayHigh": 186.10,
      "regularMarketDayLow": 182.90,
      "regularMarketVolume": 52300000,
      "marketCap": 2870000000000.0,
      "trailingPE": 28.5,
      "fiftyTwoWeekHigh": 198.23,
      "fiftyTwoWeekLow": 124.17,
      "regularMarketTime": 1706000000
    }],
    "error": null
  }
}
```

- [ ] **Step 2: 写失败测试（yahoo.rs 尾部）**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use mockito::Server;

    #[tokio::test]
    async fn test_fetch_quote_success() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/yahoo_quote_aapl.json");

        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(fixture)
            .create_async()
            .await;

        let provider = YahooFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("AAPL", Market::USStock);
        let quote = provider.fetch_quote(&symbol).await.unwrap();

        assert_eq!(quote.symbol.code, "AAPL");
        assert_eq!(quote.price, rust_decimal_macros::dec!(185.20));
        assert_eq!(quote.name.as_deref(), Some("Apple Inc."));
        assert!(quote.volume.unwrap() > 0);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_supports_markets() {
        let p = YahooFinanceProvider::new();
        assert!(p.supports(&Market::USStock));
        assert!(p.supports(&Market::HKStock));
        assert!(p.supports(&Market::AShare));
        assert!(p.supports(&Market::Crypto));
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

```bash
cargo test -p fa-data yahoo 2>&1 | head -10
```

- [ ] **Step 4: 实现 yahoo.rs**

```rust
// crates/fa-data/src/yahoo.rs
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fa_core::{DataError, DataProvider, Market, Quote, Symbol, OHLCV, Period};
use rust_decimal::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::cache::InMemoryCache;

const DEFAULT_BASE_URL: &str = "https://query1.finance.yahoo.com";

pub struct YahooFinanceProvider {
    client: reqwest::Client,
    base_url: String,
    cache: Arc<Mutex<InMemoryCache<Quote>>>,
}

impl YahooFinanceProvider {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            cache: Arc::new(Mutex::new(InMemoryCache::new(
                std::time::Duration::from_secs(60),
            ))),
        }
    }
}

// --- Deserialization structs ---

#[derive(Deserialize)]
struct YahooResponse {
    #[serde(rename = "quoteResponse")]
    quote_response: QuoteResponse,
}

#[derive(Deserialize)]
struct QuoteResponse {
    result: Vec<YahooQuoteResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct YahooQuoteResult {
    symbol: String,
    long_name: Option<String>,
    regular_market_price: f64,
    regular_market_change: f64,
    regular_market_change_percent: f64,
    regular_market_open: Option<f64>,
    regular_market_day_high: Option<f64>,
    regular_market_day_low: Option<f64>,
    regular_market_volume: Option<u64>,
    market_cap: Option<f64>,
    trailing_pe: Option<f64>,
    fifty_two_week_high: Option<f64>,
    fifty_two_week_low: Option<f64>,
    regular_market_time: Option<i64>,
}

fn f64_to_dec(v: f64) -> Decimal {
    Decimal::from_f64(v).unwrap_or(Decimal::ZERO)
}

impl TryFrom<(YahooQuoteResult, &Symbol)> for Quote {
    type Error = DataError;

    fn try_from((r, symbol): (YahooQuoteResult, &Symbol)) -> Result<Self, Self::Error> {
        let ts = r.regular_market_time
            .and_then(|t| DateTime::from_timestamp(t, 0))
            .unwrap_or_else(Utc::now);

        Ok(Quote {
            symbol: symbol.clone(),
            price: f64_to_dec(r.regular_market_price),
            change: f64_to_dec(r.regular_market_change),
            change_pct: f64_to_dec(r.regular_market_change_percent),
            open: r.regular_market_open.map(f64_to_dec),
            high: r.regular_market_day_high.map(f64_to_dec),
            low: r.regular_market_day_low.map(f64_to_dec),
            volume: r.regular_market_volume,
            market_cap: r.market_cap.map(f64_to_dec),
            pe_ratio: r.trailing_pe.map(f64_to_dec),
            week_52_high: r.fifty_two_week_high.map(f64_to_dec),
            week_52_low: r.fifty_two_week_low.map(f64_to_dec),
            name: r.long_name,
            timestamp: ts,
        })
    }
}

#[async_trait]
impl DataProvider for YahooFinanceProvider {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        let ticker = symbol.yahoo_ticker();

        {
            let cache = self.cache.lock().await;
            if let Some(cached) = cache.get(&ticker) {
                return Ok(cached);
            }
        }

        let url = format!(
            "{}/v7/finance/quote?symbols={}&fields=regularMarketPrice,regularMarketChange,\
             regularMarketChangePercent,regularMarketOpen,regularMarketDayHigh,\
             regularMarketDayLow,regularMarketVolume,marketCap,trailingPE,\
             fiftyTwoWeekHigh,fiftyTwoWeekLow,longName,regularMarketTime",
            self.base_url, ticker
        );

        let resp = self.client.get(&url).send().await
            .map_err(|e| DataError::Network(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DataError::RateLimited { retry_after: 60 });
        }

        let yahoo: YahooResponse = resp.json().await
            .map_err(|e| DataError::Parse(e.to_string()))?;

        let result = yahoo.quote_response.result.into_iter().next()
            .ok_or_else(|| DataError::SymbolNotFound { symbol: symbol.code.clone() })?;

        let quote = Quote::try_from((result, symbol))?;

        {
            let mut cache = self.cache.lock().await;
            cache.set(ticker, quote.clone());
        }

        Ok(quote)
    }

    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError> {
        let ticker = symbol.yahoo_ticker();
        let url = format!(
            "{}/v8/finance/chart/{}?interval={}&range={}",
            self.base_url, ticker,
            period.yahoo_interval(),
            period.yahoo_range()
        );

        let resp = self.client.get(&url).send().await
            .map_err(|e| DataError::Network(e.to_string()))?;

        let json: serde_json::Value = resp.json().await
            .map_err(|e| DataError::Parse(e.to_string()))?;

        parse_ohlcv_response(&json, symbol)
    }

    fn name(&self) -> &'static str { "Yahoo Finance" }

    fn supports(&self, _market: &Market) -> bool {
        true // Yahoo supports all markets (quality varies)
    }
}

fn parse_ohlcv_response(json: &serde_json::Value, symbol: &Symbol) -> Result<Vec<OHLCV>, DataError> {
    let result = &json["chart"]["result"][0];
    let timestamps = result["timestamp"].as_array()
        .ok_or_else(|| DataError::Parse("missing timestamps".into()))?;
    let quote = &result["indicators"]["quote"][0];

    let opens  = quote["open"].as_array().ok_or_else(|| DataError::Parse("missing open".into()))?;
    let highs  = quote["high"].as_array().ok_or_else(|| DataError::Parse("missing high".into()))?;
    let lows   = quote["low"].as_array().ok_or_else(|| DataError::Parse("missing low".into()))?;
    let closes = quote["close"].as_array().ok_or_else(|| DataError::Parse("missing close".into()))?;
    let vols   = quote["volume"].as_array().ok_or_else(|| DataError::Parse("missing volume".into()))?;

    let bars = timestamps.iter().enumerate()
        .filter_map(|(i, ts)| {
            let ts = ts.as_i64()?;
            let dt = DateTime::from_timestamp(ts, 0)?;
            Some(OHLCV {
                symbol: symbol.clone(),
                timestamp: dt,
                open:   f64_to_dec(opens[i].as_f64()?),
                high:   f64_to_dec(highs[i].as_f64()?),
                low:    f64_to_dec(lows[i].as_f64()?),
                close:  f64_to_dec(closes[i].as_f64()?),
                volume: vols[i].as_u64().unwrap_or(0),
            })
        })
        .collect();

    Ok(bars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use mockito::Server;

    #[tokio::test]
    async fn test_fetch_quote_success() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/yahoo_quote_aapl.json");

        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(fixture)
            .create_async()
            .await;

        let provider = YahooFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("AAPL", Market::USStock);
        let quote = provider.fetch_quote(&symbol).await.unwrap();

        assert_eq!(quote.symbol.code, "AAPL");
        assert_eq!(quote.price, rust_decimal_macros::dec!(185.20));
        assert_eq!(quote.name.as_deref(), Some("Apple Inc."));
        assert!(quote.volume.unwrap() > 0);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_fetch_quote_cache_hit() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/yahoo_quote_aapl.json");
        // Only create ONE mock — second call should use cache, not hit server
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(fixture)
            .expect(1)
            .create_async()
            .await;

        let provider = YahooFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("AAPL", Market::USStock);
        provider.fetch_quote(&symbol).await.unwrap();
        provider.fetch_quote(&symbol).await.unwrap(); // should hit cache
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_supports_markets() {
        let p = YahooFinanceProvider::new();
        assert!(p.supports(&Market::USStock));
        assert!(p.supports(&Market::HKStock));
        assert!(p.supports(&Market::AShare));
        assert!(p.supports(&Market::Crypto));
    }
}
```

- [ ] **Step 5: 运行测试**

```bash
cargo test -p fa-data yahoo -- --nocapture
```

Expected: 3 个测试全部通过

- [ ] **Step 6: 提交**

```bash
git add crates/fa-data/src/yahoo.rs fixtures/yahoo_quote_aapl.json
git commit -m "feat(fa-data): add Yahoo Finance provider with cache and mockito tests"
```

---

## Task 7: fa-data — Sina Finance Provider (A 股)

**Files:**
- Create: `crates/fa-data/src/sina.rs`
- Create: `fixtures/sina_quote_sh600519.txt`

- [ ] **Step 1: 创建 Sina fixture**

```
// fixtures/sina_quote_sh600519.txt
var hq_str_sh600519="贵州茅台,1832.00,1830.00,1845.00,1860.00,1820.00,1844.90,1845.00,12345678,22597345678.00,100,1844.90,200,1844.80,300,1845.00,100,1845.10,200,1845.20,2026-05-30,14:30:00,00,";
```

Sina 字段含义（逗号分隔）：
- 0: 股票名称 = 贵州茅台
- 1: 今日开盘价 = 1832.00
- 2: 昨日收盘价 = 1830.00
- 3: 当前价格 = 1845.00
- 4: 今日最高价 = 1860.00
- 5: 今日最低价 = 1820.00
- 8: 成交量(手) = 12345678
- 31: 日期 = 2026-05-30
- 32: 时间 = 14:30:00

- [ ] **Step 2: 写失败测试**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use mockito::Server;

    #[tokio::test]
    async fn test_parse_sina_response() {
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = parse_sina_response(fixture, &symbol).unwrap();

        assert_eq!(quote.symbol.code, "600519");
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        assert_eq!(quote.open.unwrap(), rust_decimal_macros::dec!(1832.00));
        assert_eq!(quote.high.unwrap(), rust_decimal_macros::dec!(1860.00));
        assert_eq!(quote.low.unwrap(), rust_decimal_macros::dec!(1820.00));
        assert_eq!(quote.name.as_deref(), Some("贵州茅台"));
    }

    #[tokio::test]
    async fn test_fetch_quote_via_http() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(fixture)
            .create_async()
            .await;

        let provider = SinaFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = provider.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        mock.assert_async().await;
    }

    #[test]
    fn test_supports_only_a_share() {
        let p = SinaFinanceProvider::new();
        assert!(p.supports(&Market::AShare));
        assert!(!p.supports(&Market::USStock));
        assert!(!p.supports(&Market::Crypto));
    }
}
```

- [ ] **Step 3: 运行测试，确认失败**

```bash
cargo test -p fa-data sina 2>&1 | head -10
```

- [ ] **Step 4: 实现 sina.rs**

```rust
// crates/fa-data/src/sina.rs
use async_trait::async_trait;
use chrono::Utc;
use fa_core::{DataError, DataProvider, Market, Quote, Symbol, OHLCV, Period};
use rust_decimal::prelude::*;

const DEFAULT_BASE_URL: &str = "https://hq.sinajs.cn";

pub struct SinaFinanceProvider {
    client: reqwest::Client,
    base_url: String,
}

impl SinaFinanceProvider {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self { client: reqwest::Client::new(), base_url: base_url.into() }
    }
}

/// Parse Sina Finance response text into a Quote.
/// Format: var hq_str_sh600519="name,open,prev_close,price,high,low,...";
pub fn parse_sina_response(text: &str, symbol: &Symbol) -> Result<Quote, DataError> {
    // Extract the part inside quotes
    let start = text.find('"').ok_or_else(|| DataError::Parse("no opening quote".into()))? + 1;
    let end   = text.rfind('"').ok_or_else(|| DataError::Parse("no closing quote".into()))?;
    let data  = &text[start..end];

    let fields: Vec<&str> = data.split(',').collect();
    if fields.len() < 10 {
        return Err(DataError::Parse(format!("too few fields: {}", fields.len())));
    }

    let parse = |s: &str| -> Result<Decimal, DataError> {
        Decimal::from_str(s).map_err(|_| DataError::Parse(format!("cannot parse '{s}' as decimal")))
    };

    let name       = fields[0].to_string();
    let open       = parse(fields[1])?;
    let prev_close = parse(fields[2])?;
    let price      = parse(fields[3])?;
    let high       = parse(fields[4])?;
    let low        = parse(fields[5])?;
    let volume     = fields[8].parse::<u64>().unwrap_or(0);
    let change     = price - prev_close;
    let change_pct = if prev_close.is_zero() {
        Decimal::ZERO
    } else {
        (change / prev_close) * Decimal::from(100)
    };

    Ok(Quote {
        symbol: symbol.clone(),
        price,
        change,
        change_pct,
        open: Some(open),
        high: Some(high),
        low: Some(low),
        volume: Some(volume),
        market_cap: None,
        pe_ratio: None,
        week_52_high: None,
        week_52_low: None,
        name: Some(name),
        timestamp: Utc::now(),
    })
}

#[async_trait]
impl DataProvider for SinaFinanceProvider {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        if !self.supports(&symbol.market) {
            return Err(DataError::MarketNotSupported { market: symbol.market.to_string() });
        }

        let ticker = symbol.sina_ticker();
        let url = format!("{}/list={}", self.base_url, ticker);

        let resp = self.client.get(&url).send().await
            .map_err(|e| DataError::Network(e.to_string()))?;
        let text = resp.text().await
            .map_err(|e| DataError::Network(e.to_string()))?;

        parse_sina_response(&text, symbol)
    }

    async fn fetch_ohlcv(&self, _symbol: &Symbol, _period: Period) -> Result<Vec<OHLCV>, DataError> {
        // Sina does not provide OHLCV history; use Yahoo for history
        Err(DataError::MarketNotSupported {
            market: "OHLCV not supported by Sina provider".into(),
        })
    }

    fn name(&self) -> &'static str { "Sina Finance" }

    fn supports(&self, market: &Market) -> bool {
        matches!(market, Market::AShare)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol};
    use mockito::Server;

    #[tokio::test]
    async fn test_parse_sina_response() {
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = parse_sina_response(fixture, &symbol).unwrap();

        assert_eq!(quote.symbol.code, "600519");
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        assert_eq!(quote.open.unwrap(), rust_decimal_macros::dec!(1832.00));
        assert_eq!(quote.high.unwrap(), rust_decimal_macros::dec!(1860.00));
        assert_eq!(quote.low.unwrap(), rust_decimal_macros::dec!(1820.00));
        assert_eq!(quote.name.as_deref(), Some("贵州茅台"));
    }

    #[tokio::test]
    async fn test_fetch_quote_via_http() {
        let mut server = Server::new_async().await;
        let fixture = include_str!("../../../fixtures/sina_quote_sh600519.txt");
        let mock = server
            .mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_body(fixture)
            .create_async()
            .await;

        let provider = SinaFinanceProvider::with_base_url(server.url());
        let symbol = Symbol::new("600519", Market::AShare);
        let quote = provider.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, rust_decimal_macros::dec!(1845.00));
        mock.assert_async().await;
    }

    #[test]
    fn test_supports_only_a_share() {
        let p = SinaFinanceProvider::new();
        assert!(p.supports(&Market::AShare));
        assert!(!p.supports(&Market::USStock));
        assert!(!p.supports(&Market::Crypto));
    }
}
```

- [ ] **Step 5: 运行测试**

```bash
cargo test -p fa-data sina -- --nocapture
```

Expected: 3 个测试全部通过

- [ ] **Step 6: 提交**

```bash
git add crates/fa-data/src/sina.rs fixtures/sina_quote_sh600519.txt
git commit -m "feat(fa-data): add Sina Finance provider for A-share real-time quotes"
```

---

## Task 8: fa-data — CSV Provider & Provider Router

**Files:**
- Create: `crates/fa-data/src/csv.rs`
- Create: `crates/fa-data/src/router.rs`
- Create: `fixtures/sample_portfolio.csv`

- [ ] **Step 1: 创建 CSV fixture**

```csv
// fixtures/sample_portfolio.csv
symbol,market,quantity,cost_basis
AAPL,us,10,1800.00
TSLA,us,5,1100.00
600519,a_share,2,3660.00
BTC-USD,crypto,0.5,25000.00
```

- [ ] **Step 2: 实现 csv.rs（Portfolio 导入，无需独立 DataProvider）**

```rust
// crates/fa-data/src/csv.rs
use fa_core::{DataError, Market, Portfolio, Position, Symbol};
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn load_portfolio_from_csv(content: &str) -> Result<Portfolio, DataError> {
    let mut positions = Vec::new();

    for (i, line) in content.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // skip header
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < 4 {
            return Err(DataError::Parse(format!("line {i}: expected 4 fields, got {}", fields.len())));
        }
        let code   = fields[0].trim().to_string();
        let market = fields[1].trim().parse::<Market>()?;
        let qty    = Decimal::from_str(fields[2].trim())
            .map_err(|_| DataError::Parse(format!("invalid quantity: {}", fields[2])))?;
        let cost   = Decimal::from_str(fields[3].trim())
            .map_err(|_| DataError::Parse(format!("invalid cost_basis: {}", fields[3])))?;

        positions.push(Position {
            symbol: Symbol::new(code, market),
            quantity: qty,
            cost_basis: cost,
        });
    }

    Ok(Portfolio { positions })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_load_portfolio_from_csv() {
        let csv = include_str!("../../../fixtures/sample_portfolio.csv");
        let portfolio = load_portfolio_from_csv(csv).unwrap();
        assert_eq!(portfolio.positions.len(), 4);

        let aapl = &portfolio.positions[0];
        assert_eq!(aapl.symbol.code, "AAPL");
        assert_eq!(aapl.quantity, dec!(10));
        assert_eq!(aapl.cost_basis, dec!(1800.00));
    }

    #[test]
    fn test_load_portfolio_invalid_csv() {
        let result = load_portfolio_from_csv("symbol,market\nAAPL");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 3: 实现 router.rs**

```rust
// crates/fa-data/src/router.rs
use async_trait::async_trait;
use fa_core::{DataError, DataProvider, Market, Quote, Symbol, OHLCV, Period};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Automatically selects provider by market; falls back on failure.
pub struct ProviderRouter {
    providers: Vec<Arc<dyn DataProvider>>,
    max_retries: u32,
}

impl ProviderRouter {
    pub fn new(providers: Vec<Arc<dyn DataProvider>>) -> Self {
        Self { providers, max_retries: 3 }
    }

    fn providers_for(&self, market: &Market) -> Vec<&Arc<dyn DataProvider>> {
        self.providers.iter().filter(|p| p.supports(market)).collect()
    }
}

#[async_trait]
impl DataProvider for ProviderRouter {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
        let candidates = self.providers_for(&symbol.market);
        if candidates.is_empty() {
            return Err(DataError::MarketNotSupported { market: symbol.market.to_string() });
        }

        let mut last_err = DataError::Network("no providers tried".into());

        for provider in candidates {
            let mut delay = Duration::from_secs(1);
            for attempt in 0..self.max_retries {
                match provider.fetch_quote(symbol).await {
                    Ok(quote) => return Ok(quote),
                    Err(DataError::RateLimited { retry_after }) => {
                        sleep(Duration::from_secs(retry_after)).await;
                    }
                    Err(e) => {
                        last_err = e;
                        if attempt + 1 < self.max_retries {
                            sleep(delay).await;
                            delay *= 2;
                        }
                    }
                }
            }
        }

        Err(last_err)
    }

    async fn fetch_ohlcv(&self, symbol: &Symbol, period: Period) -> Result<Vec<OHLCV>, DataError> {
        let candidates = self.providers_for(&symbol.market);
        for provider in candidates {
            match provider.fetch_ohlcv(symbol, period).await {
                Ok(data) => return Ok(data),
                Err(_) => continue,
            }
        }
        Err(DataError::Network("all providers failed for OHLCV".into()))
    }

    fn name(&self) -> &'static str { "ProviderRouter" }

    fn supports(&self, market: &Market) -> bool {
        self.providers.iter().any(|p| p.supports(market))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use fa_core::{Market, Quote, Symbol};
    use rust_decimal::Decimal;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct AlwaysFailProvider;
    struct AlwaysSucceedProvider;

    #[async_trait]
    impl DataProvider for AlwaysFailProvider {
        async fn fetch_quote(&self, _: &Symbol) -> Result<Quote, DataError> {
            Err(DataError::Network("network error".into()))
        }
        async fn fetch_ohlcv(&self, _: &Symbol, _: Period) -> Result<Vec<OHLCV>, DataError> {
            Err(DataError::Network("network error".into()))
        }
        fn name(&self) -> &'static str { "FailProvider" }
        fn supports(&self, _: &Market) -> bool { true }
    }

    #[async_trait]
    impl DataProvider for AlwaysSucceedProvider {
        async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError> {
            Ok(Quote {
                symbol: symbol.clone(),
                price: Decimal::from(100),
                change: Decimal::ZERO,
                change_pct: Decimal::ZERO,
                open: None, high: None, low: None, volume: None,
                market_cap: None, pe_ratio: None,
                week_52_high: None, week_52_low: None,
                name: None, timestamp: Utc::now(),
            })
        }
        async fn fetch_ohlcv(&self, _: &Symbol, _: Period) -> Result<Vec<OHLCV>, DataError> {
            Ok(vec![])
        }
        fn name(&self) -> &'static str { "SucceedProvider" }
        fn supports(&self, _: &Market) -> bool { true }
    }

    #[tokio::test]
    async fn test_router_uses_fallback() {
        let router = ProviderRouter {
            providers: vec![
                Arc::new(AlwaysFailProvider),
                Arc::new(AlwaysSucceedProvider),
            ],
            max_retries: 1, // skip retry delay in tests
        };
        let symbol = Symbol::new("AAPL", Market::USStock);
        let quote = router.fetch_quote(&symbol).await.unwrap();
        assert_eq!(quote.price, Decimal::from(100));
    }

    #[tokio::test]
    async fn test_router_no_provider_for_market() {
        struct NoSupportProvider;
        #[async_trait]
        impl DataProvider for NoSupportProvider {
            async fn fetch_quote(&self, _: &Symbol) -> Result<Quote, DataError> { unimplemented!() }
            async fn fetch_ohlcv(&self, _: &Symbol, _: Period) -> Result<Vec<OHLCV>, DataError> { unimplemented!() }
            fn name(&self) -> &'static str { "NoSupport" }
            fn supports(&self, _: &Market) -> bool { false }
        }
        let router = ProviderRouter::new(vec![Arc::new(NoSupportProvider)]);
        let symbol = Symbol::new("AAPL", Market::USStock);
        assert!(router.fetch_quote(&symbol).await.is_err());
    }
}
```

- [ ] **Step 4: 运行所有 fa-data 测试**

```bash
cargo test -p fa-data -- --nocapture
```

Expected: 全部通过（cache: 4, yahoo: 3, sina: 3, csv: 2, router: 2 共 ~14 个）

- [ ] **Step 5: 提交**

```bash
git add crates/fa-data/src/ fixtures/
git commit -m "feat(fa-data): add CSV loader, ProviderRouter with retry/fallback"
```

---

## Task 9: 配置系统

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs` (add config loading)

- [ ] **Step 1: 写失败测试（config.rs 底部）**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.general.refresh_interval, 30);
        assert_eq!(cfg.general.default_currency, "USD");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[general]
refresh_interval = 60
default_currency = "CNY"

[[watchlist]]
symbol = "AAPL"
market = "us"

[[watchlist]]
symbol = "600519"
market = "a_share"

[[portfolio]]
symbol = "AAPL"
market = "us"
quantity = "10"
cost_basis = "1800.00"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.refresh_interval, 60);
        assert_eq!(cfg.watchlist.len(), 2);
        assert_eq!(cfg.watchlist[0].symbol, "AAPL");
        assert_eq!(cfg.portfolio.len(), 1);
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test --bin financial-analysis config 2>&1 | head -10
```

- [ ] **Step 3: 实现 config.rs**

```rust
// src/config.rs
use fa_core::{DataError, Market, Portfolio, Position, Symbol};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub refresh_interval: u64,   // seconds
    pub default_currency: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            refresh_interval: 30,
            default_currency: "USD".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistEntry {
    pub symbol: String,
    pub market: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioEntry {
    pub symbol: String,
    pub market: String,
    pub quantity: String,
    pub cost_basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    pub alpha_vantage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub api_keys: ApiKeys,
    #[serde(default)]
    pub watchlist: Vec<WatchlistEntry>,
    #[serde(default)]
    pub portfolio: Vec<PortfolioEntry>,
}

impl Config {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("fa")
            .join("config.toml")
    }

    pub fn load() -> Result<Self, DataError> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content).map_err(|e| DataError::Config(e.to_string()))
    }

    pub fn to_watchlist_symbols(&self) -> Vec<Symbol> {
        self.watchlist.iter().filter_map(|e| {
            e.market.parse::<Market>().ok().map(|m| Symbol::new(&e.symbol, m))
        }).collect()
    }

    pub fn to_portfolio(&self) -> Portfolio {
        let positions = self.portfolio.iter().filter_map(|e| {
            let market = e.market.parse::<Market>().ok()?;
            let qty  = Decimal::from_str(&e.quantity).ok()?;
            let cost = Decimal::from_str(&e.cost_basis).ok()?;
            Some(Position {
                symbol: Symbol::new(&e.symbol, market),
                quantity: qty,
                cost_basis: cost,
            })
        }).collect();
        Portfolio { positions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.general.refresh_interval, 30);
        assert_eq!(cfg.general.default_currency, "USD");
    }

    #[test]
    fn test_parse_config_toml() {
        let toml_str = r#"
[general]
refresh_interval = 60
default_currency = "CNY"

[[watchlist]]
symbol = "AAPL"
market = "us"

[[watchlist]]
symbol = "600519"
market = "a_share"

[[portfolio]]
symbol = "AAPL"
market = "us"
quantity = "10"
cost_basis = "1800.00"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.refresh_interval, 60);
        assert_eq!(cfg.watchlist.len(), 2);
        assert_eq!(cfg.watchlist[0].symbol, "AAPL");
        assert_eq!(cfg.portfolio.len(), 1);
    }

    #[test]
    fn test_to_watchlist_symbols() {
        let cfg = Config {
            watchlist: vec![
                WatchlistEntry { symbol: "AAPL".into(), market: "us".into() },
                WatchlistEntry { symbol: "600519".into(), market: "a_share".into() },
            ],
            ..Default::default()
        };
        let syms = cfg.to_watchlist_symbols();
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].code, "AAPL");
        assert_eq!(syms[1].market, fa_core::Market::AShare);
    }
}
```

- [ ] **Step 4: 将 `dirs` 依赖加入根 Cargo.toml**

在 `Cargo.toml` 的 `[dependencies]` 下添加：

```toml
dirs = "5"
```

（同时在 `[workspace.dependencies]` 里不需要，因为只有 main crate 用到）

- [ ] **Step 5: 运行测试**

```bash
cargo test --bin financial-analysis config -- --nocapture
```

Expected: 3 个测试通过

- [ ] **Step 6: 提交**

```bash
git add src/config.rs Cargo.toml
git commit -m "feat: add config system with TOML loading and portfolio/watchlist parsing"
```

---

## Task 10: fa-tui — AppState & AppAction

**Files:**
- Create: `crates/fa-tui/src/app.rs`
- Modify: `crates/fa-tui/src/lib.rs`

- [ ] **Step 1: 写失败测试（app.rs 底部）**

```rust
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
        assert_eq!(s.selected_watchlist, 0); // wraps to top
    }

    #[test]
    fn test_move_up_wraps() {
        let mut s = make_state();
        s.selected_watchlist = 0;
        s.apply(AppAction::MoveUp);
        assert_eq!(s.selected_watchlist, 1); // wraps to bottom
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
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p fa-tui app 2>&1 | head -10
```

- [ ] **Step 3: 实现 app.rs**

```rust
// crates/fa-tui/src/app.rs
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
                // Trigger is handled externally by DataFetcher; clear stale status
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
```

- [ ] **Step 4: 运行测试**

```bash
cargo test -p fa-tui app -- --nocapture
```

Expected: 7 个测试全部通过

- [ ] **Step 5: 提交**

```bash
git add crates/fa-tui/src/app.rs crates/fa-tui/src/lib.rs
git commit -m "feat(fa-tui): add AppState, State, AppAction with full action dispatch"
```

---

## Task 11: fa-tui — EventHandler

**Files:**
- Create: `crates/fa-tui/src/event.rs`

- [ ] **Step 1: 实现 event.rs**

EventHandler 是 crossterm 事件循环，无法用标准单元测试（依赖终端）。使用集成方式通过 `AppAction` 通道验证键盘映射正确性（参考注释中的键位表）。

```rust
// crates/fa-tui/src/event.rs
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
        Self { tx, tick_rate: Duration::from_millis(100) }
    }

    pub async fn run(&self) {
        loop {
            if self.tx.is_closed() { break; }

            let Ok(has_event) = tokio::task::spawn_blocking(|| {
                event::poll(Duration::from_millis(100))
            }).await else { break };

            if !has_event.unwrap_or(false) { continue; }

            let Ok(ev) = tokio::task::spawn_blocking(event::read).await else { break };

            let action = match ev {
                Ok(Event::Key(KeyEvent { code, modifiers, .. })) => {
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
```

- [ ] **Step 2: 运行测试**

```bash
cargo test -p fa-tui event -- --nocapture
```

Expected: 4 个测试通过

- [ ] **Step 3: 提交**

```bash
git add crates/fa-tui/src/event.rs
git commit -m "feat(fa-tui): add EventHandler mapping keyboard events to AppActions"
```

---

## Task 12: fa-tui — UI 面板

**Files:**
- Create: `crates/fa-tui/src/ui/mod.rs`
- Create: `crates/fa-tui/src/ui/watchlist.rs`
- Create: `crates/fa-tui/src/ui/portfolio.rs`
- Create: `crates/fa-tui/src/ui/detail.rs`
- Create: `crates/fa-tui/src/ui/statusbar.rs`

每个面板都接受 `&State` 和 `Frame`，将内容渲染到给定 `Rect`。

- [ ] **Step 1: 创建 ui/mod.rs**

```rust
// crates/fa-tui/src/ui/mod.rs
pub mod watchlist;
pub mod portfolio;
pub mod detail;
pub mod statusbar;
pub mod layout;
```

- [ ] **Step 2: 实现 ui/watchlist.rs**

```rust
// crates/fa-tui/src/ui/watchlist.rs
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::app::{FocusedPanel, State};

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let focused = state.focused_panel == FocusedPanel::Watchlist;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = state.watchlist.iter().map(|sym| {
        let quote = state.quotes.get(&sym.code);
        let (price_str, change_str, color) = if let Some(q) = quote {
            let color = if q.is_positive() { Color::Green } else { Color::Red };
            (
                format!("{:.2}", q.price),
                q.change_display(),
                color,
            )
        } else {
            ("--".into(), "".into(), Color::Gray)
        };

        let line = Line::from(vec![
            Span::raw(format!("{:<8}", sym.code)),
            Span::styled(format!("{:>10}", price_str), Style::default().fg(color)),
            Span::styled(format!("  {:>16}", change_str), Style::default().fg(color)),
        ]);
        ListItem::new(line)
    }).collect();

    let list = List::new(items)
        .block(Block::default().title(" Watchlist ").borders(Borders::ALL).border_style(border_style))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

    let mut list_state = ListState::default();
    if !state.watchlist.is_empty() {
        list_state.select(Some(state.selected_watchlist));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::State;
    use fa_core::{Market, Symbol};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_empty_watchlist() {
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::default();

        terminal.draw(|f| {
            render(f, &state, f.area());
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Watchlist"));
    }

    #[test]
    fn test_render_with_symbols() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = State::default();
        state.watchlist.push(Symbol::new("AAPL", Market::USStock));

        terminal.draw(|f| {
            render(f, &state, f.area());
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL"));
    }
}
```

- [ ] **Step 3: 实现 ui/portfolio.rs**

```rust
// crates/fa-tui/src/ui/portfolio.rs
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use rust_decimal::Decimal;
use crate::app::{FocusedPanel, State};

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let focused = state.focused_panel == FocusedPanel::Portfolio;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let prices: std::collections::HashMap<String, Decimal> = state.quotes.iter()
        .map(|(k, q)| (k.clone(), q.price))
        .collect();

    let mut items: Vec<ListItem> = state.portfolio.positions.iter().map(|pos| {
        let price = prices.get(&pos.symbol.code).copied().unwrap_or(Decimal::ZERO);
        let mv    = pos.market_value(price);
        let pnl   = pos.pnl(price);
        let pct   = pos.pnl_pct(price);
        let color = if pnl >= Decimal::ZERO { Color::Green } else { Color::Red };
        let sign  = if pnl >= Decimal::ZERO { "+" } else { "" };

        let line = Line::from(vec![
            Span::raw(format!("{:<8} x{}", pos.symbol.code, pos.quantity)),
            Span::raw(format!("  {:>10.2}", pos.cost_basis)),
            Span::raw(format!("  {:>10.2}", mv)),
            Span::styled(
                format!("  {}{:.2}({}{:.1}%)", sign, pnl, sign, pct),
                Style::default().fg(color),
            ),
        ]);
        ListItem::new(line)
    }).collect();

    // Summary row
    let total_cost = state.portfolio.total_cost();
    let total_mv   = state.portfolio.total_market_value(&prices);
    let total_pnl  = total_mv - total_cost;
    let pnl_color  = if total_pnl >= Decimal::ZERO { Color::Green } else { Color::Red };
    let sign       = if total_pnl >= Decimal::ZERO { "+" } else { "" };

    items.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("─── Total: {:.2}  MV:{:.2}  PnL:{}{:.2}", total_cost, total_mv, sign, total_pnl),
            Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
        ),
    ])));

    let list = List::new(items)
        .block(Block::default().title(" Portfolio ").borders(Borders::ALL).border_style(border_style))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = ListState::default();
    if !state.portfolio.positions.is_empty() {
        list_state.select(Some(state.selected_portfolio));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}
```

- [ ] **Step 4: 实现 ui/detail.rs**

```rust
// crates/fa-tui/src/ui/detail.rs
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::State;

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let content = if let Some(sym) = state.selected_symbol() {
        if let Some(q) = state.quotes.get(&sym.code) {
            let name = q.name.as_deref().unwrap_or(&sym.code);
            vec![
                Line::from(format!(
                    "{}  {}  最新: {:.2}  {}",
                    sym.code, sym.market, q.price, q.change_display()
                )),
                Line::from(format!(
                    "开: {}  高: {}  低: {}  量: {}",
                    q.open.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.high.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.low.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.volume.map(|v| {
                        if v >= 1_000_000 { format!("{:.1}M", v as f64 / 1e6) }
                        else { format!("{}", v) }
                    }).unwrap_or("--".into()),
                )),
                Line::from(format!(
                    "52周高/低: {} / {}  市值: {}  PE: {}",
                    q.week_52_high.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.week_52_low.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.market_cap.map(|v| {
                        if v >= 1e12 { format!("{:.2}T", v / rust_decimal::Decimal::from(1_000_000_000_000i64)) }
                        else { format!("{:.2}B", v / rust_decimal::Decimal::from(1_000_000_000i64)) }
                    }).unwrap_or("--".into()),
                    q.pe_ratio.map(|v| format!("{:.1}", v)).unwrap_or("--".into()),
                )),
            ]
        } else {
            vec![Line::from(format!("{} — 加载中...", sym.code))]
        }
    } else {
        vec![Line::from("选择一个股票查看详情")]
    };

    let para = Paragraph::new(content)
        .block(Block::default().title(" Detail ").borders(Borders::ALL));
    f.render_widget(para, area);
}
```

- [ ] **Step 5: 实现 ui/statusbar.rs**

```rust
// crates/fa-tui/src/ui/statusbar.rs
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::app::State;

pub fn render(f: &mut Frame, state: &State, area: Rect, refresh_interval: u64) {
    let updated = state.last_updated
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "从未".into());

    let status = state.status_message.as_deref().unwrap_or("");
    let search = if state.is_search_active {
        format!("  搜索: {}_", state.search_input)
    } else {
        "".into()
    };

    let line = Line::from(vec![
        Span::styled(
            format!("[更新: {}] [刷新: {}s]  {}{}",
                updated, refresh_interval, status, search),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    f.render_widget(Paragraph::new(line), area);
}
```

- [ ] **Step 6: 运行 fa-tui 测试**

```bash
cargo test -p fa-tui -- --nocapture
```

Expected: watchlist 渲染测试 2 个 + app 测试 7 个 + event 测试 4 个，全部通过

- [ ] **Step 7: 提交**

```bash
git add crates/fa-tui/src/ui/
git commit -m "feat(fa-tui): add UI panels (watchlist, portfolio, detail, statusbar)"
```

---

## Task 13: fa-tui — 根布局 & main.rs 组装

**Files:**
- Create: `crates/fa-tui/src/ui/layout.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 实现 ui/layout.rs**

```rust
// crates/fa-tui/src/ui/layout.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::State;
use super::{detail, portfolio, statusbar, watchlist};

pub fn render(f: &mut Frame, state: &State, refresh_interval: u64) {
    let area = f.area();

    // Top title bar
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // title
            Constraint::Min(0),     // main content
            Constraint::Length(1),  // status bar
        ])
        .split(area);

    render_title(f, root[0]);

    // Main area: left watchlist | right portfolio
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(root[1]);

    // Left: watchlist on top, detail on bottom
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main[0]);

    watchlist::render(f, state, left[0]);
    detail::render(f, state, left[1]);
    portfolio::render(f, state, main[1]);
    statusbar::render(f, state, root[2], refresh_interval);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(
        " Financial Analysis  [Tab] 切换面板  [/] 搜索  [d] 删除  [r] 刷新  [q] 退出"
    ))
    .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    f.render_widget(title, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::State;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_layout_renders_without_panic() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::default();

        terminal.draw(|f| {
            render(f, &state, 30);
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Financial Analysis"));
        assert!(content.contains("Watchlist"));
        assert!(content.contains("Portfolio"));
    }
}
```

- [ ] **Step 2: 实现 main.rs**

```rust
// src/main.rs
mod config;

use anyhow::Result;
use fa_data::{router::ProviderRouter, sina::SinaFinanceProvider, yahoo::YahooFinanceProvider};
use fa_tui::{
    app::{AppAction, AppState, State},
    event::EventHandler,
    ui::layout,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = config::Config::load()?;

    // Build provider router
    let router = Arc::new(ProviderRouter::new(vec![
        Arc::new(YahooFinanceProvider::new()),
        Arc::new(SinaFinanceProvider::new()),
    ]));

    // Initial state from config
    let initial_state = State {
        watchlist: cfg.to_watchlist_symbols(),
        portfolio: cfg.to_portfolio(),
        ..Default::default()
    };
    let app_state: AppState = Arc::new(tokio::sync::RwLock::new(initial_state));

    // Action channel
    let (tx, mut rx) = mpsc::channel::<AppAction>(64);

    // Spawn EventHandler task
    let event_tx = tx.clone();
    tokio::spawn(async move {
        EventHandler::new(event_tx).run().await;
    });

    // Spawn DataFetcher task
    let fetcher_state = Arc::clone(&app_state);
    let fetcher_tx = tx.clone();
    let refresh_secs = cfg.general.refresh_interval;
    tokio::spawn(async move {
        loop {
            let symbols = {
                let s = fetcher_state.read().await;
                s.watchlist.clone()
            };

            let mut quotes = Vec::new();
            for sym in &symbols {
                use fa_core::DataProvider;
                match router.fetch_quote(sym).await {
                    Ok(q) => quotes.push(q),
                    Err(e) => {
                        let _ = fetcher_tx.send(AppAction::StatusMessage(
                            format!("[!] {} 获取失败: {}", sym.code, e)
                        )).await;
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

    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(info);
    }));

    // Main render loop
    let result = run_app(&mut terminal, &app_state, &mut rx, refresh_secs).await;

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
) -> Result<()> {
    let tick = Duration::from_millis(16);

    loop {
        // Render
        {
            let state = app_state.read().await;
            terminal.draw(|f| layout::render(f, &state, refresh_secs))?;

            if state.should_quit {
                break;
            }
        }

        // Process actions (non-blocking drain)
        let deadline = tokio::time::Instant::now() + tick;
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(action)) => {
                    let mut state = app_state.write().await;
                    state.apply(action);
                    if state.should_quit { return Ok(()); }
                }
                Ok(None) => return Ok(()), // channel closed
                Err(_) => break,           // tick elapsed
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 3: 在 fa-tui/Cargo.toml 中确认 crossterm 已添加（已在 workspace 中）**

确认 `crates/fa-tui/Cargo.toml` 包含：

```toml
[dependencies]
crossterm = { workspace = true }
```

- [ ] **Step 4: 编译整个项目**

```bash
cargo build --release 2>&1 | tail -20
```

Expected: 编译成功，生成 `target/release/financial-analysis`

- [ ] **Step 5: 运行全部测试**

```bash
cargo test --workspace -- --nocapture 2>&1 | tail -30
```

Expected: 所有测试通过。示例输出：
```
test result: ok. 4 passed; 0 failed; 0 ignored
test result: ok. 14 passed; 0 failed; 0 ignored
test result: ok. 13 passed; 0 failed; 0 ignored
test result: ok. 3 passed; 0 failed; 0 ignored
```

- [ ] **Step 6: 创建默认配置目录和模板**

在根目录创建 `config/default.toml` 供用户参考：

```toml
# config/default.toml — 复制到 ~/.config/fa/config.toml 并修改

[general]
refresh_interval = 30
default_currency = "CNY"

[api_keys]
# alpha_vantage = "YOUR_KEY_HERE"

[[watchlist]]
symbol = "AAPL"
market = "us"

[[watchlist]]
symbol = "600519"
market = "a_share"

[[portfolio]]
symbol = "AAPL"
market = "us"
quantity = "10"
cost_basis = "1800.00"
```

- [ ] **Step 7: 最终提交**

```bash
git add src/main.rs crates/fa-tui/src/ui/layout.rs config/default.toml
git commit -m "feat: complete Phase 1 TUI assembly with DataFetcher, EventHandler, render loop"
```

---

## 自检（Spec Coverage）

| 规格要求 | 对应任务 |
|----------|---------|
| Cargo Workspace (fa-core/fa-data/fa-tui) | Task 1 |
| DataError 错误类型 | Task 2 |
| Market enum + Symbol struct | Task 2-3 |
| Quote + OHLCV + Period | Task 3-4 |
| Portfolio + Position + PnL | Task 4 |
| DataProvider trait (async) | Task 4 |
| InMemoryCache with TTL | Task 5 |
| Yahoo Finance provider + cache | Task 6 |
| Sina Finance provider (A股) | Task 7 |
| CSV portfolio import | Task 8 |
| ProviderRouter (retry + fallback) | Task 8 |
| Config TOML loading | Task 9 |
| AppState + AppAction + State transitions | Task 10 |
| EventHandler (keyboard → AppAction) | Task 11 |
| Watchlist panel (ratatui) | Task 12 |
| Portfolio panel with PnL | Task 12 |
| Detail panel | Task 12 |
| Status bar | Task 12 |
| 根布局组装 | Task 13 |
| main.rs tokio Actor 组装 | Task 13 |
| Panic hook + terminal restore | Task 13 |
| 16ms render tick | Task 13 |
| 30s data refresh | Task 13 |
