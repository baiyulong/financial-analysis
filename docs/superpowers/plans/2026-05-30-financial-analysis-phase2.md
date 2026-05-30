# Financial Analysis TUI — Phase 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 添加全屏 K 线图视图，支持 5 个时间周期、MA5/10/20 均线叠加、光标移动和宽度缩放，按 Enter 进入、Esc 返回。

**Architecture:** 新增 `fa-indicator` crate 封装纯 SMA 计算；`fa-tui` 新增 `AppScreen` 枚举和 `ChartState`；`EventHandler::run()` 接受 `AppState` 引用决定按键映射；main.rs 在 `EnterChart`/`ChartChangePeriod` 事件后派发独立 tokio 任务抓取 OHLCV 数据。

**Tech Stack:** Rust, Ratatui 0.28 (`Canvas` widget), fa-core `Period`/`OHLCV`, `rust_decimal::prelude::ToPrimitive`

---

## File Map

### 新建
| 文件 | 职责 |
|------|------|
| `crates/fa-indicator/Cargo.toml` | fa-indicator crate 配置，依赖 fa-core |
| `crates/fa-indicator/src/lib.rs` | 重导出 sma |
| `crates/fa-indicator/src/ma.rs` | `sma(data, period)` 实现 + 测试 |
| `crates/fa-tui/src/ui/chart.rs` | 全屏 K 线图渲染（标题栏 + Canvas + 光标信息栏） |

### 修改
| 文件 | 修改内容 |
|------|----------|
| `Cargo.toml` | workspace members 增加 fa-indicator；root deps 增加 fa-indicator |
| `crates/fa-tui/Cargo.toml` | deps 增加 fa-indicator |
| `crates/fa-tui/src/app.rs` | 新增 AppScreen/ChartState；State.screen 字段；AppAction 新变体；apply() 新 arm |
| `crates/fa-tui/src/event.rs` | run() 接受 AppState；拆分为 map_key_main/map_key_chart；Enter 键映射 |
| `crates/fa-tui/src/ui/mod.rs` | 新增 `pub mod chart;` |
| `src/main.rs` | EventHandler::run 传 state；render 按 AppScreen 分支；ChartFetcher 任务 |

---

## Task 1: fa-indicator crate — SMA 计算

**Files:**
- Create: `crates/fa-indicator/Cargo.toml`
- Create: `crates/fa-indicator/src/lib.rs`
- Create: `crates/fa-indicator/src/ma.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: 写失败测试（TDD 先行）**

`crates/fa-indicator/src/ma.rs` 测试先于实现：

```rust
// crates/fa-indicator/src/ma.rs
use fa_core::OHLCV;
use rust_decimal::Decimal;

pub fn sma(data: &[OHLCV], period: usize) -> Vec<Option<Decimal>> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn make_bar(close: Decimal) -> OHLCV {
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
        }
    }

    #[test]
    fn test_sma_period_1_equals_close() {
        let data = vec![make_bar(dec!(10)), make_bar(dec!(20)), make_bar(dec!(30))];
        let result = sma(&data, 1);
        assert_eq!(result, vec![Some(dec!(10)), Some(dec!(20)), Some(dec!(30))]);
    }

    #[test]
    fn test_sma_period_3() {
        let data = vec![
            make_bar(dec!(10)),
            make_bar(dec!(20)),
            make_bar(dec!(30)),
            make_bar(dec!(40)),
        ];
        let result = sma(&data, 3);
        assert_eq!(result[0], None);
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some(dec!(20)));
        assert_eq!(result[3], Some(dec!(30)));
    }

    #[test]
    fn test_sma_period_greater_than_data_returns_all_none() {
        let data = vec![make_bar(dec!(100)), make_bar(dec!(200))];
        let result = sma(&data, 5);
        assert_eq!(result, vec![None, None]);
    }

    #[test]
    fn test_sma_empty_data() {
        let result = sma(&[], 5);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_sma_period_zero_returns_all_none() {
        let data = vec![make_bar(dec!(10)), make_bar(dec!(20))];
        let result = sma(&data, 0);
        assert_eq!(result, vec![None, None]);
    }
}
```

- [ ] **Step 2: 创建 Cargo.toml 文件**

创建 `crates/fa-indicator/Cargo.toml`：

```toml
[package]
name = "fa-indicator"
version = "0.1.0"
edition = "2021"

[dependencies]
fa-core          = { path = "../fa-core" }
rust_decimal     = { workspace = true }
rust_decimal_macros = { workspace = true }

[dev-dependencies]
chrono = { workspace = true }
```

- [ ] **Step 3: 创建 lib.rs**

```rust
// crates/fa-indicator/src/lib.rs
pub mod ma;
pub use ma::sma;
```

- [ ] **Step 4: 运行测试，确认失败**

```bash
cargo test -p fa-indicator -- --nocapture 2>&1 | head -20
```

Expected: 编译错误或 `todo!()` panic

- [ ] **Step 5: 实现 sma()**

```rust
// crates/fa-indicator/src/ma.rs（完整文件）
use fa_core::OHLCV;
use rust_decimal::Decimal;

/// 简单移动平均（SMA）。
/// - 前 period-1 个值返回 None。
/// - period == 0 或 period > data.len() 时，全部返回 None。
/// - data 应为时间升序，使用收盘价计算。
pub fn sma(data: &[OHLCV], period: usize) -> Vec<Option<Decimal>> {
    if period == 0 || data.is_empty() {
        return vec![None; data.len()];
    }
    data.iter()
        .enumerate()
        .map(|(i, _)| {
            if i + 1 < period {
                None
            } else {
                let window = &data[i + 1 - period..=i];
                let sum: Decimal = window.iter().map(|b| b.close).sum();
                Some(sum / Decimal::from(period))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    // ... （同 Step 1 的测试内容）
    use super::*;
    use fa_core::{Market, Symbol, OHLCV};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use chrono::Utc;

    fn make_bar(close: Decimal) -> OHLCV {
        OHLCV {
            symbol: Symbol::new("TEST", Market::USStock),
            timestamp: Utc::now(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 0,
        }
    }

    #[test]
    fn test_sma_period_1_equals_close() {
        let data = vec![make_bar(dec!(10)), make_bar(dec!(20)), make_bar(dec!(30))];
        let result = sma(&data, 1);
        assert_eq!(result, vec![Some(dec!(10)), Some(dec!(20)), Some(dec!(30))]);
    }

    #[test]
    fn test_sma_period_3() {
        let data = vec![
            make_bar(dec!(10)),
            make_bar(dec!(20)),
            make_bar(dec!(30)),
            make_bar(dec!(40)),
        ];
        let result = sma(&data, 3);
        assert_eq!(result[0], None);
        assert_eq!(result[1], None);
        assert_eq!(result[2], Some(dec!(20)));
        assert_eq!(result[3], Some(dec!(30)));
    }

    #[test]
    fn test_sma_period_greater_than_data_returns_all_none() {
        let data = vec![make_bar(dec!(100)), make_bar(dec!(200))];
        let result = sma(&data, 5);
        assert_eq!(result, vec![None, None]);
    }

    #[test]
    fn test_sma_empty_data() {
        let result = sma(&[], 5);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_sma_period_zero_returns_all_none() {
        let data = vec![make_bar(dec!(10)), make_bar(dec!(20))];
        let result = sma(&data, 0);
        assert_eq!(result, vec![None, None]);
    }
}
```

- [ ] **Step 6: 更新根 Cargo.toml**

在 `Cargo.toml` 的 `[workspace]` members 中新增 `"crates/fa-indicator"`；在根 `[dependencies]` 中新增：

```toml
[workspace]
members = [".", "crates/fa-core", "crates/fa-data", "crates/fa-tui", "crates/fa-indicator"]
```

```toml
# 根 [dependencies] 新增（fa-indicator 暂时不在 root binary 中使用，但需要 workspace 知道它）
# 不需要在 root [dependencies] 里加，只在 workspace members 里加即可
```

- [ ] **Step 7: 运行测试，确认通过**

```bash
cargo test -p fa-indicator -- --nocapture
```

Expected: `test result: ok. 5 passed; 0 failed`

- [ ] **Step 8: 提交**

```bash
git add crates/fa-indicator/ Cargo.toml Cargo.lock
git commit -m "feat(fa-indicator): add SMA calculation crate with 5 tests"
```

---

## Task 2: fa-tui AppState 扩展

**Files:**
- Modify: `crates/fa-tui/src/app.rs`
- Modify: `crates/fa-tui/Cargo.toml`

- [ ] **Step 1: 更新 fa-tui/Cargo.toml**

在 `[dependencies]` 中增加：

```toml
fa-indicator = { path = "../fa-indicator" }
```

- [ ] **Step 2: 写失败测试（添加到 app.rs 底部 tests 模块）**

在 `app.rs` 的 `mod tests` 中新增以下测试（在 `test_delete_cleans_up_quotes` 之后）：

```rust
    #[test]
    fn test_enter_chart_sets_screen() {
        let mut s = make_state();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.apply(AppAction::EnterChart(sym));
        assert!(matches!(s.screen, AppScreen::Chart(_)));
    }

    #[test]
    fn test_exit_chart_returns_to_main() {
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(
            Symbol::new("AAPL", Market::USStock),
            fa_core::Period::Month1,
        ));
        s.apply(AppAction::ExitChart);
        assert!(matches!(s.screen, AppScreen::Main));
    }

    #[test]
    fn test_chart_data_loaded_sets_cursor_to_last() {
        use fa_core::{OHLCV, Period};
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1));
        let bars = vec![
            OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                    open: dec!(100), high: dec!(110), low: dec!(90), close: dec!(105), volume: 1000 },
            OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                    open: dec!(105), high: dec!(115), low: dec!(95), close: dec!(110), volume: 2000 },
        ];
        s.apply(AppAction::ChartDataLoaded(bars));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.cursor, 1); // last bar
            assert!(!cs.loading);
            assert_eq!(cs.data.len(), 2);
        } else {
            panic!("expected Chart screen");
        }
    }

    #[test]
    fn test_chart_cursor_clamps_at_boundaries() {
        use fa_core::{OHLCV, Period};
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        let bar = OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                          open: dec!(100), high: dec!(110), low: dec!(90), close: dec!(105), volume: 0 };
        s.screen = AppScreen::Chart(ChartState {
            symbol: Symbol::new("AAPL", Market::USStock),
            period: Period::Month1,
            data: vec![bar.clone(), bar],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5],
            loading: false,
        });
        // Move left at position 0 → stays at 0
        s.apply(AppAction::ChartMoveCursor(-1));
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.cursor, 0); }
        // Move right twice → clamps at 1 (last index)
        s.apply(AppAction::ChartMoveCursor(1));
        s.apply(AppAction::ChartMoveCursor(1));
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.cursor, 1); }
    }

    #[test]
    fn test_chart_zoom_clamps() {
        use fa_core::Period;
        let mut s = make_state();
        s.screen = AppScreen::Chart(ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1));
        // Default bar_width is 3; zoom out to min=2
        s.apply(AppAction::ChartZoom(false));
        s.apply(AppAction::ChartZoom(false)); // clamp at 2
        s.apply(AppAction::ChartZoom(false)); // still 2
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.bar_width, 2); }
        // Zoom in to 8
        for _ in 0..10 { s.apply(AppAction::ChartZoom(true)); }
        if let AppScreen::Chart(ref cs) = s.screen { assert_eq!(cs.bar_width, 8); }
    }

    #[test]
    fn test_chart_change_period_resets_data() {
        use fa_core::{OHLCV, Period};
        use chrono::Utc;
        use rust_decimal_macros::dec;
        let mut s = make_state();
        let bar = OHLCV { symbol: Symbol::new("AAPL", Market::USStock), timestamp: Utc::now(),
                          open: dec!(100), high: dec!(110), low: dec!(90), close: dec!(105), volume: 0 };
        s.screen = AppScreen::Chart(ChartState {
            symbol: Symbol::new("AAPL", Market::USStock),
            period: Period::Month1,
            data: vec![bar],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5],
            loading: false,
        });
        s.apply(AppAction::ChartChangePeriod(Period::Year1));
        if let AppScreen::Chart(ref cs) = s.screen {
            assert_eq!(cs.period, Period::Year1);
            assert!(cs.loading);
            assert!(cs.data.is_empty());
        }
    }
```

- [ ] **Step 3: 实现 AppScreen, ChartState 并扩展 app.rs**

在 `crates/fa-tui/src/app.rs` 顶部 `use` 语句后，现有 `FocusedPanel` 前，插入：

```rust
use fa_core::{OHLCV, Period};
```

（`Symbol`, `Portfolio`, `Quote` 已有，新增 `OHLCV`, `Period`）

在文件中，在 `FocusedPanel` 定义后插入：

```rust
#[derive(Debug, Clone)]
pub enum AppScreen {
    Main,
    Chart(ChartState),
}

impl Default for AppScreen {
    fn default() -> Self { AppScreen::Main }
}

#[derive(Debug, Clone)]
pub struct ChartState {
    pub symbol: Symbol,
    pub period: Period,
    pub data: Vec<OHLCV>,
    pub cursor: usize,
    pub bar_width: u16,
    pub ma_periods: Vec<usize>,
    pub loading: bool,
}

impl ChartState {
    pub fn new(symbol: Symbol, period: Period) -> Self {
        Self {
            symbol,
            period,
            data: vec![],
            cursor: 0,
            bar_width: 3,
            ma_periods: vec![5, 10, 20],
            loading: true,
        }
    }

    pub fn current_bar(&self) -> Option<&OHLCV> {
        self.data.get(self.cursor)
    }
}
```

在 `State` 结构体定义中，在 `should_quit: bool,` 后新增字段：

```rust
    pub screen: AppScreen,
```

在 `AppAction` enum 的最后一个变体 `StatusMessage(String),` 后新增：

```rust
    EnterChart(Symbol),
    ExitChart,
    ChartDataLoaded(Vec<OHLCV>),
    ChartMoveCursor(i32),
    ChartZoom(bool),
    ChartChangePeriod(Period),
```

在 `State::apply()` 的 match 中，在 `AppAction::Refresh =>` 分支后新增：

```rust
            AppAction::EnterChart(sym) => {
                self.screen = AppScreen::Chart(ChartState::new(sym, Period::Month1));
            }
            AppAction::ExitChart => {
                self.screen = AppScreen::Main;
            }
            AppAction::ChartDataLoaded(data) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    let len = data.len();
                    cs.data = data;
                    cs.cursor = len.saturating_sub(1);
                    cs.loading = false;
                }
            }
            AppAction::ChartMoveCursor(delta) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    let len = cs.data.len();
                    if len > 0 {
                        cs.cursor = (cs.cursor as i64 + delta as i64)
                            .clamp(0, len as i64 - 1) as usize;
                    }
                }
            }
            AppAction::ChartZoom(zoom_in) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    if zoom_in && cs.bar_width < 8 { cs.bar_width += 1; }
                    else if !zoom_in && cs.bar_width > 2 { cs.bar_width -= 1; }
                }
            }
            AppAction::ChartChangePeriod(period) => {
                if let AppScreen::Chart(ref mut cs) = self.screen {
                    cs.period = period;
                    cs.loading = true;
                    cs.data.clear();
                }
            }
```

- [ ] **Step 4: 运行测试，确认通过**

```bash
cargo test -p fa-tui app -- --nocapture
```

Expected: `test result: ok. 14 passed; 0 failed`（原有 8 个 + 新增 6 个）

- [ ] **Step 5: 提交**

```bash
git add crates/fa-tui/src/app.rs crates/fa-tui/Cargo.toml
git commit -m "feat(fa-tui): add AppScreen/ChartState, extend AppAction with chart actions"
```

---

## Task 3: EventHandler 扩展（screen-aware 键位映射）

**Files:**
- Modify: `crates/fa-tui/src/event.rs`

- [ ] **Step 1: 写失败测试**

在 `event.rs` 的 `mod tests` 中新增：

```rust
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
        use fa_core::Period;
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
```

- [ ] **Step 2: 实现 map_key_chart 并重构 event.rs**

完整替换 `crates/fa-tui/src/event.rs` 内容：

```rust
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
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
                    let is_chart = {
                        let s = state.read().await;
                        matches!(s.screen, AppScreen::Chart(_))
                    };
                    if is_chart {
                        Self::map_key_chart(code, modifiers)
                    } else {
                        // Handle Enter key: inject selected symbol from state
                        if code == KeyCode::Enter && modifiers == KeyModifiers::NONE {
                            let s = state.read().await;
                            s.selected_symbol().map(|sym| AppAction::EnterChart(sym.clone()))
                        } else {
                            Self::map_key_main(code, modifiers)
                        }
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
        use fa_core::Period;
        match (code, modifiers) {
            (KeyCode::Esc, _)                         => Some(AppAction::ExitChart),
            (KeyCode::Left, _)                        => Some(AppAction::ChartMoveCursor(-1)),
            (KeyCode::Right, _)                       => Some(AppAction::ChartMoveCursor(1)),
            (KeyCode::Char('['), KeyModifiers::NONE)  => Some(AppAction::ChartZoom(false)),
            (KeyCode::Char(']'), KeyModifiers::NONE)  => Some(AppAction::ChartZoom(true)),
            (KeyCode::Char('1'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Day1)),
            (KeyCode::Char('5'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Week1)),
            (KeyCode::Char('m'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Month1)),
            (KeyCode::Char('q'), KeyModifiers::NONE)  => Some(AppAction::ChartChangePeriod(Period::Month3)),
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
}
```

- [ ] **Step 3: 运行测试，确认通过**

```bash
cargo test -p fa-tui event -- --nocapture
```

Expected: `test result: ok. 8 passed; 0 failed`（原 4 个 + 新增 4 个）

注意：`run()` 签名从 `pub async fn run(&self)` 改为 `pub async fn run(&self, state: AppState)`，`main.rs` 暂时会编译失败——Task 5 会修复。现在先用 `-p fa-tui` 单独测试。

- [ ] **Step 4: 提交**

```bash
git add crates/fa-tui/src/event.rs
git commit -m "feat(fa-tui): extend EventHandler with chart mode key bindings and screen-aware dispatch"
```

---

## Task 4: K 线图全屏 UI 面板

**Files:**
- Create: `crates/fa-tui/src/ui/chart.rs`
- Modify: `crates/fa-tui/src/ui/mod.rs`

- [ ] **Step 1: 写渲染测试**

创建 `crates/fa-tui/src/ui/chart.rs`，先写测试：

```rust
// crates/fa-tui/src/ui/chart.rs

use fa_core::{Market, Period, Symbol, OHLCV};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        canvas::{self, Canvas},
        Block, Borders, Paragraph,
    },
    Frame,
};
use rust_decimal::prelude::ToPrimitive;
use crate::app::ChartState;

pub fn render(f: &mut Frame, cs: &ChartState, area: Rect) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ChartState;
    use fa_core::{Market, Period, Symbol, OHLCV};
    use ratatui::{backend::TestBackend, Terminal};
    use chrono::Utc;
    use rust_decimal_macros::dec;

    fn make_chart_state_loading() -> ChartState {
        ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1)
    }

    fn make_chart_state_with_data() -> ChartState {
        let sym = Symbol::new("AAPL", Market::USStock);
        let bar = |o: i64, h: i64, l: i64, c: i64| OHLCV {
            symbol: sym.clone(),
            timestamp: Utc::now(),
            open: dec!(1) * rust_decimal::Decimal::from(o),
            high: dec!(1) * rust_decimal::Decimal::from(h),
            low:  dec!(1) * rust_decimal::Decimal::from(l),
            close: dec!(1) * rust_decimal::Decimal::from(c),
            volume: 1000,
        };
        ChartState {
            symbol: sym,
            period: Period::Month1,
            data: vec![
                bar(100, 110, 90, 105),
                bar(105, 115, 95, 98),
                bar(98, 108, 88, 102),
            ],
            cursor: 2,
            bar_width: 3,
            ma_periods: vec![],
            loading: false,
        }
    }

    #[test]
    fn test_render_loading_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let cs = make_chart_state_loading();

        terminal.draw(|f| render(f, &cs, f.area())).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL") || content.contains("Loading"));
    }

    #[test]
    fn test_render_with_data_no_panic() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let cs = make_chart_state_with_data();

        terminal.draw(|f| render(f, &cs, f.area())).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL"));
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

```bash
cargo test -p fa-tui ui::chart -- --nocapture 2>&1 | head -10
```

Expected: 编译失败（`todo!()`）或 panic

- [ ] **Step 3: 实现 chart.rs 完整渲染**

```rust
// crates/fa-tui/src/ui/chart.rs（完整实现）
use fa_core::{Period, OHLCV};
use fa_indicator::sma;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Rectangle},
        Block, Borders, Paragraph,
    },
    Frame,
};
use rust_decimal::prelude::ToPrimitive;
use crate::app::ChartState;

pub fn render(f: &mut Frame, cs: &ChartState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // 标题栏
            Constraint::Min(0),     // 图表区
            Constraint::Length(1),  // 光标信息
        ])
        .split(area);

    render_titlebar(f, cs, chunks[0]);
    render_chart(f, cs, chunks[1]);
    render_cursor_info(f, cs, chunks[2]);
}

fn period_label(p: &Period) -> &'static str {
    match p {
        Period::Day1   => "1D",
        Period::Week1  => "5D",
        Period::Month1 => "1M",
        Period::Month3 => "3M",
        Period::Year1  => "1Y",
        _              => "?",
    }
}

fn render_titlebar(f: &mut Frame, cs: &ChartState, area: Rect) {
    let text = format!(
        " {}  {}  | [1]1D [5]5D [m]1M [q]3M [y]1Y  [←→]光标  [[]缩放  [Esc]返回",
        cs.symbol.display_code(),
        period_label(&cs.period),
    );
    f.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        area,
    );
}

fn render_chart(f: &mut Frame, cs: &ChartState, area: Rect) {
    if cs.loading || cs.data.is_empty() {
        let msg = if cs.loading { "Loading data..." } else { "No data available" };
        f.render_widget(
            Paragraph::new(msg)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    // Calculate how many bars fit in the chart area (subtract borders + Y-axis)
    let chart_inner_width = area.width.saturating_sub(8) as usize;
    let bar_w = cs.bar_width as usize;
    let max_visible = (chart_inner_width / bar_w).max(1);

    // Window: center cursor in view
    let half = max_visible / 2;
    let end = (cs.cursor + half + 1).min(cs.data.len());
    let start = end.saturating_sub(max_visible);
    let end = (start + max_visible).min(cs.data.len());
    let visible = &cs.data[start..end];

    // Y-axis price bounds
    let price_min = visible.iter()
        .filter_map(|b| b.low.to_f64())
        .fold(f64::MAX, f64::min);
    let price_max = visible.iter()
        .filter_map(|b| b.high.to_f64())
        .fold(f64::MIN, f64::max);

    let padding = ((price_max - price_min) * 0.05).max(0.01);
    let y_min = price_min - padding;
    let y_max = price_max + padding;
    let x_max = (visible.len() * bar_w) as f64;

    // Pre-compute MA values for visible range
    let ma_configs: &[(usize, Color)] = &[(5, Color::Yellow), (10, Color::Cyan), (20, Color::Magenta)];
    let cursor_in_view = cs.cursor.saturating_sub(start);

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL))
        .x_bounds([0.0, x_max])
        .y_bounds([y_min, y_max])
        .paint(|ctx| {
            // Draw K-line bars
            for (i, bar) in visible.iter().enumerate() {
                let x_center = i as f64 * bar_w as f64 + bar_w as f64 / 2.0;
                let open  = bar.open.to_f64().unwrap_or(y_min);
                let high  = bar.high.to_f64().unwrap_or(y_min);
                let low   = bar.low.to_f64().unwrap_or(y_min);
                let close = bar.close.to_f64().unwrap_or(y_min);
                let color = if close >= open { Color::Red } else { Color::Green };
                let body_top = open.max(close);
                let body_bot = open.min(close);

                // Upper wick
                ctx.draw(&CanvasLine { x1: x_center, y1: body_top, x2: x_center, y2: high, color });
                // Lower wick
                ctx.draw(&CanvasLine { x1: x_center, y1: low, x2: x_center, y2: body_bot, color });
                // Body
                ctx.draw(&Rectangle {
                    x: i as f64 * bar_w as f64,
                    y: body_bot,
                    width: (bar_w as f64 - 0.5).max(0.5),
                    height: (body_top - body_bot).max(0.05 * (y_max - y_min)),
                    color,
                });

                // Cursor highlight (white vertical line)
                if i == cursor_in_view {
                    ctx.draw(&CanvasLine {
                        x1: x_center, y1: y_min,
                        x2: x_center, y2: y_max,
                        color: Color::White,
                    });
                }
            }

            // Draw MA lines
            for &(period, color) in ma_configs {
                if !cs.ma_periods.contains(&period) { continue; }
                let all_ma = sma(&cs.data, period);
                if all_ma.len() < end { return; }
                let vis_ma = &all_ma[start..end];
                let mut prev: Option<(f64, f64)> = None;
                for (i, v) in vis_ma.iter().enumerate() {
                    if let Some(y) = v.and_then(|d| d.to_f64()) {
                        let x = i as f64 * bar_w as f64 + bar_w as f64 / 2.0;
                        if let Some((px, py)) = prev {
                            ctx.draw(&CanvasLine { x1: px, y1: py, x2: x, y2: y, color });
                        }
                        prev = Some((x, y));
                    } else {
                        prev = None;
                    }
                }
            }
        });

    f.render_widget(canvas, area);
}

fn render_cursor_info(f: &mut Frame, cs: &ChartState, area: Rect) {
    let text = if let Some(bar) = cs.current_bar() {
        let change_pct = if !bar.open.is_zero() {
            (bar.close - bar.open) / bar.open * rust_decimal::Decimal::from(100)
        } else {
            rust_decimal::Decimal::ZERO
        };
        let sign = if bar.close >= bar.open { "+" } else { "" };
        format!(
            " {}  开:{:.2}  高:{:.2}  低:{:.2}  收:{:.2}  {}{:.2}%",
            bar.timestamp.format("%Y-%m-%d"),
            bar.open, bar.high, bar.low, bar.close,
            sign, change_pct,
        )
    } else {
        " 无数据".to_string()
    };

    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Gray)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ChartState;
    use fa_core::{Market, Period, Symbol, OHLCV};
    use ratatui::{backend::TestBackend, Terminal};
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn make_chart_state_loading() -> ChartState {
        ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1)
    }

    fn make_chart_state_with_data() -> ChartState {
        let sym = Symbol::new("AAPL", Market::USStock);
        let bar = |o: i64, h: i64, l: i64, c: i64| OHLCV {
            symbol: sym.clone(),
            timestamp: Utc::now(),
            open:  Decimal::from(o),
            high:  Decimal::from(h),
            low:   Decimal::from(l),
            close: Decimal::from(c),
            volume: 1000,
        };
        ChartState {
            symbol: sym,
            period: Period::Month1,
            data: vec![
                bar(100, 110, 90, 105),
                bar(105, 115, 95,  98),
                bar( 98, 108, 88, 102),
            ],
            cursor: 2,
            bar_width: 3,
            ma_periods: vec![],
            loading: false,
        }
    }

    #[test]
    fn test_render_loading_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let cs = make_chart_state_loading();
        terminal.draw(|f| render(f, &cs, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL") || content.contains("Loading"));
    }

    #[test]
    fn test_render_with_data_no_panic() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let cs = make_chart_state_with_data();
        terminal.draw(|f| render(f, &cs, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL"));
    }
}
```

- [ ] **Step 4: 更新 ui/mod.rs**

在 `crates/fa-tui/src/ui/mod.rs` 中新增：

```rust
pub mod chart;
```

- [ ] **Step 5: 运行测试，确认通过**

```bash
cargo test -p fa-tui -- --nocapture 2>&1 | tail -10
```

Expected: chart 的 2 个新测试通过，加上所有原有测试

- [ ] **Step 6: 提交**

```bash
git add crates/fa-tui/src/ui/chart.rs crates/fa-tui/src/ui/mod.rs
git commit -m "feat(fa-tui): add full-screen K-line chart panel with Canvas rendering and MA lines"
```

---

## Task 5: main.rs 组装（ChartFetcher + 渲染分发）

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: 读取当前 main.rs**

```bash
cat src/main.rs
```

理解当前结构后再修改。

- [ ] **Step 2: 完整替换 src/main.rs**

```rust
// src/main.rs
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
    ui::{chart, detail, layout, portfolio, statusbar, watchlist},
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

    let result = run_app(&mut terminal, &app_state, &mut rx, refresh_secs, &router).await;

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
) -> Result<()> {
    let tick = Duration::from_millis(16);

    loop {
        // Render current screen
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
                }
            })?;

            if state.should_quit {
                break;
            }
        }

        // Process actions (drain until tick deadline)
        let deadline = tokio::time::Instant::now() + tick;
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(action)) => {
                    // Spawn chart data fetcher on entering chart or changing period
                    let should_fetch = matches!(
                        &action,
                        AppAction::EnterChart(_) | AppAction::ChartChangePeriod(_)
                    );

                    let mut state = app_state.write().await;
                    state.apply(action);

                    if should_fetch {
                        if let AppScreen::Chart(cs) = &state.screen {
                            let symbol = cs.symbol.clone();
                            let period = cs.period;
                            let router = Arc::clone(router);
                            let tx_clone = {
                                // We need to send via a fresh clone;
                                // get tx from outside the lock scope below
                                drop(state); // release write lock before spawn
                                let r = Arc::clone(router);
                                tokio::spawn(async move {
                                    // (tx_clone captured below)
                                    let _ = r; // placeholder; actual tx captured separately
                                });
                                // This approach won't work directly — see note below
                                break; // ← handle via outer channel clone
                            };
                        }
                    }

                    if state.should_quit { return Ok(()); }
                }
                Ok(None) => return Ok(()),
                Err(_) => break,
            }
        }
    }

    Ok(())
}
```

**注意：** 上面的 chart fetch 派发有问题——write lock 和 spawn 冲突。正确做法是在 `apply` 后记录 flag，然后在 lock 释放后派发。实际的正确实现：

```rust
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
                                Ok(data) => { let _ = tx.send(AppAction::ChartDataLoaded(data)).await; }
                                Err(e)   => { let _ = tx.send(AppAction::StatusMessage(
                                    format!("K线获取失败: {}", e)
                                )).await; }
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
```

`main()` 调用改为：

```rust
let result = run_app(&mut terminal, &app_state, &mut rx, refresh_secs, &router, &tx).await;
```

- [ ] **Step 3: 编译整个 workspace**

```bash
cargo build 2>&1 | grep "error\|warning:" | head -30
```

修复所有 error（warning 可忽略）。常见问题：
- `AppScreen` 未导入到 main.rs → 已在 use 中引入
- `run_app` 签名增加了 `tx` 参数 → 确保调用处也传 `&tx`
- `fa_tui::ui::chart` 未在 use 中 → 已包含

- [ ] **Step 4: 运行全部测试**

```bash
cargo test --workspace 2>&1 | grep "test result"
```

Expected（每行）：
```
test result: ok. 18 passed; 0 failed  ← fa-core
test result: ok. 14 passed; 0 failed  ← fa-data
test result: ok.  5 passed; 0 failed  ← fa-indicator
test result: ok. 22 passed; 0 failed  ← fa-tui (14 app + 8 event + 2 chart)
test result: ok.  3 passed; 0 failed  ← main binary
```

- [ ] **Step 5: 提交**

```bash
git add src/main.rs
git commit -m "feat: wire Phase 2 chart screen into main render loop with ChartFetcher task"
```

---

## 自检（Spec Coverage）

| 设计要求 | 对应 Task |
|----------|-----------|
| fa-indicator crate + sma() | Task 1 |
| AppScreen / ChartState | Task 2 |
| AppAction chart variants (6个) | Task 2 |
| State::apply chart arms | Task 2 |
| EventHandler map_key_chart (10个键位) | Task 3 |
| EventHandler Enter → EnterChart | Task 3 |
| chart.rs 全屏渲染（3层布局） | Task 4 |
| Canvas K线 + MA叠加 | Task 4 |
| Loading 状态显示 | Task 4 |
| 光标高亮竖线 | Task 4 |
| 光标信息栏 (OHLCV详情) | Task 4 |
| main.rs ChartFetcher tokio task | Task 5 |
| main.rs 渲染分发 (Main/Chart) | Task 5 |
| 5个时间周期 (Day1/Week1/Month1/Month3/Year1) | Task 3+4 |
| MA5/10/20 均线颜色 (黄/青/品红) | Task 4 |
| 光标移动不越界 (clamp) | Task 2 |
| bar_width 缩放 2~8 | Task 2 |
| Esc 返回主界面 | Task 3 |
