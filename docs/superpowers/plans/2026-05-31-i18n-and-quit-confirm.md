# i18n + Quit Confirmation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Chinese/English language switching (in settings, persisted to SQLite) and a modal quit-confirmation dialog (triggered by `q`, dismissed by `n`/Esc, confirmed by `y`/Enter).

**Architecture:** A new `i18n.rs` file holds the `Language` enum and a `Strings` struct with two static constants (`ZH`, `EN`). `State::strings()` returns the right constant. All UI render functions accept `&'static Strings` (or access it from `&State`). A `confirm_quit: bool` overlay flag in `State` controls the modal; `event.rs` routes keys into it at highest priority.

**Tech Stack:** Rust, ratatui, rusqlite (existing), async-trait (existing). No new dependencies.

---

## File Map

| Action | File |
|--------|------|
| **Create** | `crates/fa-tui/src/i18n.rs` |
| **Modify** | `crates/fa-tui/src/lib.rs` (add `pub mod i18n`) |
| **Modify** | `crates/fa-tui/src/app.rs` — State fields + SettingsState + new actions |
| **Modify** | `src/storage.rs` — load/save language |
| **Modify** | `src/main.rs` — read language on startup; persist on save |
| **Modify** | `crates/fa-tui/src/ui/mod.rs` — quit overlay + pass strings |
| **Modify** | `crates/fa-tui/src/ui/watchlist.rs` — use strings |
| **Modify** | `crates/fa-tui/src/ui/portfolio.rs` — use strings |
| **Modify** | `crates/fa-tui/src/ui/detail.rs` — use strings |
| **Modify** | `crates/fa-tui/src/ui/chart.rs` — accept + use strings |
| **Modify** | `crates/fa-tui/src/ui/backtest.rs` — accept + use strings |
| **Modify** | `crates/fa-tui/src/ui/settings.rs` — language row |
| **Modify** | `crates/fa-tui/src/event.rs` — quit confirm routing + `RequestQuit` |

---

### Task 1: Create `crates/fa-tui/src/i18n.rs`

**Files:**
- Create: `crates/fa-tui/src/i18n.rs`
- Modify: `crates/fa-tui/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Add a temporary test file or inline tests at bottom of `i18n.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_zh_strings_not_empty() {
        assert!(!ZH.watchlist_title.is_empty());
        assert!(!ZH.quit_title.is_empty());
    }
    #[test]
    fn test_en_strings_not_empty() {
        assert!(!EN.watchlist_title.is_empty());
        assert!(!EN.quit_title.is_empty());
    }
    #[test]
    fn test_language_default_is_zh() {
        assert_eq!(Language::default(), Language::Zh);
    }
}
```

- [ ] **Step 2: Run test — expect compile error** (`cargo test -p fa-tui 2>&1 | head -20`)

- [ ] **Step 3: Create `crates/fa-tui/src/i18n.rs`** with the full content:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    Zh,
    En,
}

impl Default for Language {
    fn default() -> Self {
        Language::Zh
    }
}

impl Language {
    /// Stable DB key string.
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Zh => "zh",
            Language::En => "en",
        }
    }
}

impl std::str::FromStr for Language {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "zh" => Ok(Language::Zh),
            "en" => Ok(Language::En),
            _ => Err(()),
        }
    }
}

/// All user-visible UI strings.
pub struct Strings {
    // ── Status bar ──────────────────────────────────────────────────
    pub data_source_label: &'static str, // "数据源" / "Source"
    pub updated_label: &'static str,     // "Updated" (same both langs)
    pub refresh_label: &'static str,     // "Refresh" (same)
    pub never: &'static str,             // "从未" / "Never"
    pub search_prompt: &'static str,     // "Search: " (same)
    // ── Watchlist ───────────────────────────────────────────────────
    pub watchlist_title: &'static str,   // " 自选股 " / " Watchlist "
    pub add_title_prefix: &'static str,  // " Add: " (same)
    // ── Portfolio ───────────────────────────────────────────────────
    pub portfolio_title: &'static str,   // " 持仓 " / " Portfolio "
    // ── Detail ──────────────────────────────────────────────────────
    pub detail_title: &'static str,      // " Detail " (same)
    pub detail_loading: &'static str,    // "加载中..." / "Loading..."
    pub detail_select_hint: &'static str,// "Select a symbol to view details" (same)
    pub detail_latest: &'static str,     // "最新" / "Last"
    pub detail_open: &'static str,       // "开" / "Open"
    pub detail_high: &'static str,       // "高" / "High"
    pub detail_low: &'static str,        // "低" / "Low"
    pub detail_volume: &'static str,     // "成交量" / "Volume"
    pub detail_vol_unit_yi: &'static str,// "亿手" / "00M lots"
    pub detail_vol_unit_wan: &'static str,// "万手" / "0k lots"
    // ── Chart ───────────────────────────────────────────────────────
    pub chart_loading: &'static str,     // "Loading data..." (same)
    pub chart_no_data: &'static str,     // "No data available" (same)
    pub chart_help: &'static str,        // full help bar text
    pub chart_data_source_prefix: &'static str, // "数据源: " / "Source: "
    pub chart_max_history: &'static str, // "已显示最多历史数据" / "Max history loaded"
    // ── Backtest ────────────────────────────────────────────────────
    pub bt_config_title: &'static str,   // " 回测配置 " / " Backtest Config "
    pub bt_result_title: &'static str,   // " 回测结果 " / " Backtest Results "
    pub bt_trades_title: &'static str,   // " 交易记录  ↑/↓ 滚动 " / " Trades  ↑/↓ scroll "
    pub bt_nav_help: &'static str,       // "← → 切换策略  r 运行  Esc 返回"
    pub bt_status_idle: &'static str,    // " [r 运行]" / " [r run]"
    pub bt_status_running: &'static str, // " ⏳ 运行中..." / " ⏳ Running..."
    pub bt_status_done: &'static str,    // " ✅ 完成" / " ✅ Done"
    pub bt_press_r: &'static str,        // "按 r 开始回测" / "Press r to run backtest"
    pub bt_running_msg: &'static str,    // "正在运行回测..." / "Running backtest..."
    pub bt_no_trades: &'static str,      // " 暂无交易记录" / " No trades"
    pub bt_label_stock: &'static str,    // "股票: " / "Stock: "
    pub bt_label_strategy: &'static str, // "策略: " / "Strategy: "
    pub bt_label_cash: &'static str,     // "资金: " / "Cash: "
    pub bt_label_commission: &'static str,// "手续费: " / "Commission: "
    pub bt_label_slippage: &'static str, // "滑点:   " / "Slippage: "
    pub bt_label_period: &'static str,   // "时间: " / "Period: "
    pub bt_label_total_return: &'static str, // "总收益:  " / "Return:  "
    pub bt_label_annualized: &'static str,   // "年化:  " / "Ann.Ret:  "
    pub bt_label_max_drawdown: &'static str, // "最大回撤: " / "Max DD: "
    pub bt_label_win_rate: &'static str,     // "胜率:  " / "Win Rate:  "
    pub bt_label_sharpe: &'static str,       // "Sharpe:   " (same)
    pub bt_label_trades: &'static str,       // "交易次数: " / "Trades: "
    pub bt_label_initial: &'static str,      // "初始: " / "Initial: "
    pub bt_label_final: &'static str,        // "最终: " / "Final: "
    pub bt_trade_date: &'static str,         // "日期" / "Date"
    pub bt_trade_action: &'static str,       // "操作" / "Action"
    pub bt_trade_price: &'static str,        // "价格" / "Price"
    pub bt_trade_qty: &'static str,          // "数量" / "Qty"
    pub bt_trade_amount: &'static str,       // "金额" / "Amount"
    pub bt_trade_pnl: &'static str,          // "盈亏" / "PnL"
    pub bt_trade_buy: &'static str,          // "买入" / "Buy"
    pub bt_trade_sell: &'static str,         // "卖出" / "Sell"
    // ── Settings ────────────────────────────────────────────────────
    pub settings_title: &'static str,        // "⚙ 系统设置 (按 Esc 取消 / Enter 保存)"
    pub settings_data_source: &'static str,  // "数据源      " / "Data Source  "
    pub settings_akshare_url: &'static str,  // "AkShare URL " (same)
    pub settings_akshare_only: &'static str, // "(仅 AkShare 使用)" / "(AkShare only)"
    pub settings_language: &'static str,     // "语言/Language" (same)
    pub settings_help: &'static str,         // "↑↓ 切换 | Space 选择 | Enter 保存 | Esc 取消"
    pub settings_lang_zh: &'static str,      // "中文" (same both)
    pub settings_lang_en: &'static str,      // "English" (same both)
    // ── Quit confirm ────────────────────────────────────────────────
    pub quit_title: &'static str,   // "  确认退出？  " / "  Quit?  "
    pub quit_yes: &'static str,     // "[ 确认 ]" / "[ Yes ]"
    pub quit_no: &'static str,      // "[ 取消 ]" / "[ No  ]"
    pub quit_help: &'static str,    // "y/Enter 确认   n/Esc 取消"
}

pub static ZH: Strings = Strings {
    data_source_label: "数据源",
    updated_label: "Updated",
    refresh_label: "Refresh",
    never: "从未",
    search_prompt: "Search: ",
    watchlist_title: " 自选股 ",
    add_title_prefix: " Add: ",
    portfolio_title: " 持仓 ",
    detail_title: " Detail ",
    detail_loading: "加载中...",
    detail_select_hint: "Select a symbol to view details",
    detail_latest: "最新",
    detail_open: "开",
    detail_high: "高",
    detail_low: "低",
    detail_volume: "成交量",
    detail_vol_unit_yi: "亿手",
    detail_vol_unit_wan: "万手",
    chart_loading: "Loading data...",
    chart_no_data: "No data available",
    chart_help: " 数据源: {} | F1-F5:分钟 | 1:日 5:周 m:月 q:季 y:年 | ←→:移动 | []:缩放 | Esc:返回 ",
    chart_data_source_prefix: "数据源: ",
    chart_max_history: "已显示最多历史数据",
    bt_config_title: " 回测配置 ",
    bt_result_title: " 回测结果 ",
    bt_trades_title: " 交易记录  ↑/↓ 滚动 ",
    bt_nav_help: "← → 切换策略  r 运行  Esc 返回",
    bt_status_idle: " [r 运行]",
    bt_status_running: " ⏳ 运行中...",
    bt_status_done: " ✅ 完成",
    bt_press_r: "按 r 开始回测",
    bt_running_msg: "正在运行回测...",
    bt_no_trades: " 暂无交易记录",
    bt_label_stock: "股票: ",
    bt_label_strategy: "策略: ",
    bt_label_cash: "资金: ",
    bt_label_commission: "手续费: ",
    bt_label_slippage: "滑点:   ",
    bt_label_period: "时间: ",
    bt_label_total_return: "总收益:  ",
    bt_label_annualized: "年化:  ",
    bt_label_max_drawdown: "最大回撤: ",
    bt_label_win_rate: "胜率:  ",
    bt_label_sharpe: "Sharpe:   ",
    bt_label_trades: "交易次数: ",
    bt_label_initial: "初始: ",
    bt_label_final: "最终: ",
    bt_trade_date: "日期",
    bt_trade_action: "操作",
    bt_trade_price: "价格",
    bt_trade_qty: "数量",
    bt_trade_amount: "金额",
    bt_trade_pnl: "盈亏",
    bt_trade_buy: "买入",
    bt_trade_sell: "卖出",
    settings_title: "⚙ 系统设置 (按 Esc 取消 / Enter 保存)",
    settings_data_source: "数据源      ",
    settings_akshare_url: "AkShare URL ",
    settings_akshare_only: "(仅 AkShare 使用)",
    settings_language: "语言/Language",
    settings_help: "↑↓ 切换 | Space 选择 | Enter 保存 | Esc 取消",
    settings_lang_zh: "中文",
    settings_lang_en: "English",
    quit_title: "  确认退出？  ",
    quit_yes: "[ 确认 ]",
    quit_no: "[ 取消 ]",
    quit_help: "y/Enter 确认   n/Esc 取消",
};

pub static EN: Strings = Strings {
    data_source_label: "Source",
    updated_label: "Updated",
    refresh_label: "Refresh",
    never: "Never",
    search_prompt: "Search: ",
    watchlist_title: " Watchlist ",
    add_title_prefix: " Add: ",
    portfolio_title: " Portfolio ",
    detail_title: " Detail ",
    detail_loading: "Loading...",
    detail_select_hint: "Select a symbol to view details",
    detail_latest: "Last",
    detail_open: "Open",
    detail_high: "High",
    detail_low: "Low",
    detail_volume: "Volume",
    detail_vol_unit_yi: "00M lots",
    detail_vol_unit_wan: "0k lots",
    chart_loading: "Loading data...",
    chart_no_data: "No data available",
    chart_help: " Source: {} | F1-F5:min | 1:day 5:wk m:mo q:qtr y:yr | ←→:scroll | []:zoom | Esc:back ",
    chart_data_source_prefix: "Source: ",
    chart_max_history: "Max history loaded",
    bt_config_title: " Backtest Config ",
    bt_result_title: " Backtest Results ",
    bt_trades_title: " Trades  ↑/↓ scroll ",
    bt_nav_help: "← → strategy  r run  Esc back",
    bt_status_idle: " [r run]",
    bt_status_running: " ⏳ Running...",
    bt_status_done: " ✅ Done",
    bt_press_r: "Press r to run backtest",
    bt_running_msg: "Running backtest...",
    bt_no_trades: " No trades",
    bt_label_stock: "Stock: ",
    bt_label_strategy: "Strategy: ",
    bt_label_cash: "Cash: ",
    bt_label_commission: "Commission: ",
    bt_label_slippage: "Slippage:   ",
    bt_label_period: "Period: ",
    bt_label_total_return: "Return:  ",
    bt_label_annualized: "Ann.Ret: ",
    bt_label_max_drawdown: "Max DD:   ",
    bt_label_win_rate: "Win Rate: ",
    bt_label_sharpe: "Sharpe:   ",
    bt_label_trades: "Trades:   ",
    bt_label_initial: "Initial: ",
    bt_label_final: "Final:   ",
    bt_trade_date: "Date",
    bt_trade_action: "Action",
    bt_trade_price: "Price",
    bt_trade_qty: "Qty",
    bt_trade_amount: "Amount",
    bt_trade_pnl: "PnL",
    bt_trade_buy: "Buy",
    bt_trade_sell: "Sell",
    settings_title: "⚙ Settings (Esc cancel / Enter save)",
    settings_data_source: "Data Source  ",
    settings_akshare_url: "AkShare URL  ",
    settings_akshare_only: "(AkShare only)",
    settings_language: "语言/Language",
    settings_help: "↑↓ nav | Space select | Enter save | Esc cancel",
    settings_lang_zh: "中文",
    settings_lang_en: "English",
    quit_title: "  Quit?  ",
    quit_yes: "[ Yes ]",
    quit_no: "[ No  ]",
    quit_help: "y/Enter confirm   n/Esc cancel",
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_zh_strings_not_empty() {
        assert!(!ZH.watchlist_title.is_empty());
        assert!(!ZH.quit_title.is_empty());
    }
    #[test]
    fn test_en_strings_not_empty() {
        assert!(!EN.watchlist_title.is_empty());
        assert!(!EN.quit_title.is_empty());
    }
    #[test]
    fn test_language_default_is_zh() {
        assert_eq!(Language::default(), Language::Zh);
    }
    #[test]
    fn test_language_roundtrip() {
        use std::str::FromStr;
        assert_eq!(Language::from_str("zh").unwrap(), Language::Zh);
        assert_eq!(Language::from_str("en").unwrap(), Language::En);
        assert!(Language::from_str("fr").is_err());
        assert_eq!(Language::Zh.as_str(), "zh");
        assert_eq!(Language::En.as_str(), "en");
    }
}
```

- [ ] **Step 4: Add `pub mod i18n;` to `crates/fa-tui/src/lib.rs`**

Open `crates/fa-tui/src/lib.rs` and add: `pub mod i18n;`

- [ ] **Step 5: Run tests**

```bash
cargo test -p fa-tui -- i18n 2>&1
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/fa-tui/src/i18n.rs crates/fa-tui/src/lib.rs
git commit -m "feat(i18n): add Language enum and Strings struct with ZH/EN constants"
```

---

### Task 2: Add fields + actions to `app.rs`

**Files:**
- Modify: `crates/fa-tui/src/app.rs`

- [ ] **Step 1: Write failing tests** (add to test module at bottom of `app.rs`)

```rust
#[test]
fn test_request_quit_sets_confirm_flag() {
    let mut s = State::default();
    assert!(!s.confirm_quit);
    s.apply(AppAction::RequestQuit);
    assert!(s.confirm_quit);
    assert!(!s.should_quit);
}

#[test]
fn test_cancel_quit_clears_flag() {
    let mut s = State::default();
    s.confirm_quit = true;
    s.apply(AppAction::CancelQuit);
    assert!(!s.confirm_quit);
}

#[test]
fn test_toggle_quit_button_flips_focus() {
    let mut s = State::default();
    assert!(!s.confirm_quit_focused);
    s.apply(AppAction::ToggleQuitButton);
    assert!(s.confirm_quit_focused);
    s.apply(AppAction::ToggleQuitButton);
    assert!(!s.confirm_quit_focused);
}

#[test]
fn test_state_strings_returns_zh_by_default() {
    let s = State::default();
    assert_eq!(s.strings().watchlist_title, crate::i18n::ZH.watchlist_title);
}

#[test]
fn test_state_strings_returns_en_when_set() {
    use crate::i18n::Language;
    let mut s = State::default();
    s.language = Language::En;
    assert_eq!(s.strings().watchlist_title, crate::i18n::EN.watchlist_title);
}
```

- [ ] **Step 2: Run tests — expect compile errors** (`cargo test -p fa-tui 2>&1 | head -30`)

- [ ] **Step 3: Add `language`, `confirm_quit`, `confirm_quit_focused` to `State` struct**

In `State` struct (around line 180), add three fields:
```rust
pub language: crate::i18n::Language,
pub confirm_quit: bool,
pub confirm_quit_focused: bool,  // false = "confirm" button focused, true = "cancel"
```

In `Default for State`, add:
```rust
language: crate::i18n::Language::default(),
confirm_quit: false,
confirm_quit_focused: false,
```

Add `strings()` method to `impl State`:
```rust
pub fn strings(&self) -> &'static crate::i18n::Strings {
    match self.language {
        crate::i18n::Language::Zh => &crate::i18n::ZH,
        crate::i18n::Language::En => &crate::i18n::EN,
    }
}
```

- [ ] **Step 4: Add `language` to `SettingsState`** (same struct block, ~line 27):

```rust
pub language: crate::i18n::Language,
```

Update `SettingsState::new` to accept `language: crate::i18n::Language`:
```rust
pub fn new(provider: DataSourceKind, akshare_url: String, language: crate::i18n::Language) -> Self {
    Self {
        provider,
        akshare_url,
        language,
        focused_field: 0,
        editing_url: false,
    }
}
```

Update `field_count()` to `3` (provider=0, URL=1, language=2):
```rust
pub fn field_count() -> usize { 3 }
```

- [ ] **Step 5: Add new `AppAction` variants** (in the `AppAction` enum near end of file):

```rust
RequestQuit,
CancelQuit,
ToggleQuitButton,
SettingsSelectLanguage(crate::i18n::Language),
```

- [ ] **Step 6: Add `apply()` handlers** for new actions (in `State::apply` match):

Replace the existing `AppAction::Quit` handler and add new ones:
```rust
AppAction::Quit => {
    self.should_quit = true;
    self.confirm_quit = false;
}
AppAction::RequestQuit => {
    self.confirm_quit = true;
    self.confirm_quit_focused = false;
}
AppAction::CancelQuit => {
    self.confirm_quit = false;
    self.confirm_quit_focused = false;
}
AppAction::ToggleQuitButton => {
    self.confirm_quit_focused = !self.confirm_quit_focused;
}
```

In `AppAction::OpenSettings`, update `SettingsState::new` call (it now takes 3 args):
```rust
AppAction::OpenSettings => {
    let ss = SettingsState::new(
        self.data_source.clone(),
        self.akshare_url.clone(),
        self.language,
    );
    self.screen = AppScreen::Settings(ss);
}
```

In `AppAction::SettingsSaved`, apply language from settings:
```rust
AppAction::SettingsSaved => {
    if let AppScreen::Settings(ref ss) = self.screen {
        self.data_source = ss.provider.clone();
        self.akshare_url = ss.akshare_url.clone();
        self.language = ss.language;  // ← apply language
        self.screen = AppScreen::Main;
    }
}
```

Add handler for `SettingsSelectLanguage`:
```rust
AppAction::SettingsSelectLanguage(lang) => {
    if let AppScreen::Settings(ref mut ss) = self.screen {
        ss.language = lang;
    }
}
```

- [ ] **Step 7: Run tests**

```bash
cargo test -p fa-tui 2>&1 | tail -10
```

Expected: all previous + 5 new tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/fa-tui/src/app.rs
git commit -m "feat(i18n): add language/confirm_quit state + RequestQuit/CancelQuit/SettingsSelectLanguage actions"
```

---

### Task 3: Storage + `main.rs` — persist language

**Files:**
- Modify: `src/storage.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing test** (add to `storage.rs` test module)

```rust
#[test]
fn test_save_and_load_language() {
    let db = Storage::open(":memory:").unwrap();
    db.save_language("en").unwrap();
    assert_eq!(db.load_language(), Some("en".to_string()));
    db.save_language("zh").unwrap();
    assert_eq!(db.load_language(), Some("zh".to_string()));
}
```

- [ ] **Step 2: Run test — expect compile error** (`cargo test --bin financial-analysis 2>&1 | head -20`)

- [ ] **Step 3: Add methods to `Storage` in `src/storage.rs`**

```rust
pub fn save_language(&self, lang: &str) -> Result<()> {
    self.set_setting("language", lang)
}

pub fn load_language(&self) -> Option<String> {
    self.get_setting("language")
}
```

- [ ] **Step 4: Update `build_initial_state` in `src/main.rs`** to read language from DB:

```rust
fn build_initial_state(db: &Storage) -> State {
    let (provider_opt, url_opt) = db.load_data_source();
    let data_source = match provider_opt.as_deref() {
        Some("akshare") => DataSourceKind::AkShare,
        _ => DataSourceKind::Sina,
    };
    let akshare_url = url_opt.unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    use std::str::FromStr;
    let language = db
        .load_language()
        .as_deref()
        .and_then(|s| fa_tui::i18n::Language::from_str(s).ok())
        .unwrap_or_default();

    State {
        watchlist: db.load_watchlist(),
        portfolio: db.load_portfolio(),
        data_source,
        akshare_url,
        language,
        ..Default::default()
    }
}
```

Add required import at top of `src/main.rs` (if not already there): the `Language` type comes through `fa_tui::i18n`.

- [ ] **Step 5: Update `SettingsSaved` handling in `run_app`** (in `src/main.rs`) to persist language:

Find the block that handles `router_config` (after `SettingsSaved`). Add language persistence alongside data source persistence:

```rust
if let Some((kind, akshare_url)) = router_config {
    // ... existing router rebuild code ...
    if let Ok(db) = storage.lock() {
        if let Err(err) = db.save_data_source(provider_str, &akshare_url) { /* ... */ }
        // NEW: persist language
        let lang_str = {
            let state = app_state.read().await;  // brief read after write lock released
            state.language.as_str()
        };
        let _ = db.save_language(lang_str);
    }
}
```

Wait — the write lock is released before this block. We can read `state.language` inside the block:
```rust
if let Some((kind, akshare_url)) = router_config {
    let lang_str = {
        let state = app_state.read().await;
        state.language.as_str()
    };
    // rebuild router ...
    if let Ok(db) = storage.lock() {
        let _ = db.save_data_source(provider_str, &akshare_url);
        let _ = db.save_language(lang_str);
    }
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test 2>&1 | tail -10
```

Expected: all tests pass including `test_save_and_load_language`.

- [ ] **Step 7: Commit**

```bash
git add src/storage.rs src/main.rs
git commit -m "feat(i18n): persist language to SQLite; load on startup"
```

---

### Task 4: Update `settings.rs` event handlers for language field navigation

**Files:**
- Modify: `crates/fa-tui/src/event.rs`

The settings screen keyboard routing uses `focused_field` counts. Now there are 3 fields (0=provider, 1=URL, 2=language). Space/Enter on field 2 should cycle through languages. `SettingsSelectProvider` stays for field 0. For field 2, pressing Space/Enter toggles Zh↔En.

- [ ] **Step 1: Write failing test** (in `event.rs` tests)

```rust
#[test]
fn test_settings_space_on_language_field_toggles_language() {
    use crate::i18n::Language;
    let mut ss = crate::app::SettingsState::new(
        crate::app::DataSourceKind::Sina,
        "http://localhost".into(),
        Language::Zh,
    );
    ss.focused_field = 2;
    let action = EventHandler::map_key_settings(
        crossterm::event::KeyCode::Char(' '),
        crossterm::event::KeyModifiers::NONE,
        &ss,
    );
    assert_eq!(action, Some(AppAction::SettingsSelectLanguage(Language::En)));
}
```

- [ ] **Step 2: Run test — expect compile error**

- [ ] **Step 3: Update `map_key_settings` in `event.rs`**

Find the `map_key_settings` function. The Space/Enter handler for field 0 selects provider. Add field 2 handling:

```rust
fn map_key_settings(code: KeyCode, modifiers: KeyModifiers, ss: &SettingsState) -> Option<AppAction> {
    match (code, modifiers) {
        (KeyCode::Up, _) => Some(AppAction::SettingsNavUp),
        (KeyCode::Down, _) => Some(AppAction::SettingsNavDown),
        (KeyCode::Esc, _) => Some(AppAction::ExitSettings),
        (KeyCode::Enter, KeyModifiers::NONE) if !ss.editing_url => Some(AppAction::SettingsSaved),
        (KeyCode::Char(' '), KeyModifiers::NONE) | (KeyCode::Enter, KeyModifiers::NONE)
            if ss.focused_field == 0 =>
        {
            let next = match ss.provider {
                DataSourceKind::Sina => DataSourceKind::AkShare,
                DataSourceKind::AkShare => DataSourceKind::Sina,
            };
            Some(AppAction::SettingsSelectProvider(next))
        }
        (KeyCode::Char(' '), KeyModifiers::NONE) if ss.focused_field == 1 => {
            Some(AppAction::SettingsToggleUrlEdit)
        }
        (KeyCode::Char(' '), KeyModifiers::NONE) if ss.focused_field == 2 => {
            use crate::i18n::Language;
            let next = match ss.language {
                Language::Zh => Language::En,
                Language::En => Language::Zh,
            };
            Some(AppAction::SettingsSelectLanguage(next))
        }
        (KeyCode::Char(c), KeyModifiers::NONE) => {
            if ss.editing_url && ss.focused_field == 1 {
                Some(AppAction::SettingsEditUrlChar(c))
            } else {
                None
            }
        }
        (KeyCode::Backspace, _) => {
            if ss.editing_url && ss.focused_field == 1 {
                Some(AppAction::SettingsEditUrlBackspace)
            } else {
                None
            }
        }
        _ => None,
    }
}
```

Note: The `Enter` key for `SettingsSaved` must take priority over field-specific Enter. Check the current implementation carefully and maintain the correct priority order. If the current code uses if-else chains, restructure to check `!ss.editing_url` before field checks.

- [ ] **Step 4: Run tests**

```bash
cargo test -p fa-tui -- event 2>&1 | tail -15
```

Expected: all event tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/fa-tui/src/event.rs
git commit -m "feat(i18n): add Space-on-language-field routing in settings event handler"
```

---

### Task 5: Replace UI strings in `mod.rs`, `watchlist.rs`, `portfolio.rs`, `detail.rs`

**Files:**
- Modify: `crates/fa-tui/src/ui/mod.rs`
- Modify: `crates/fa-tui/src/ui/watchlist.rs`
- Modify: `crates/fa-tui/src/ui/portfolio.rs`
- Modify: `crates/fa-tui/src/ui/detail.rs`

All these already receive `&State`, so they can call `state.strings()`.

- [ ] **Step 1: Verify existing tests still pass as baseline**

```bash
cargo test -p fa-tui -- ui 2>&1 | tail -15
```

- [ ] **Step 2: Update `mod.rs`**

In `data_source_label()`:
```rust
// Remove this function — no longer needed (callers use strings.data_source_label directly)
```

Actually keep `data_source_label()` for now since chart.rs still uses it — change its body to be unused internally. Or just leave it and deprecate. Safest: keep the function, update `render_main_statusbar` to use `state.strings()`:

```rust
fn render_main_statusbar(f: &mut Frame, state: &State, area: Rect, refresh_interval: u64) {
    let s = state.strings();
    let updated = state.last_updated
        .map(|t| t.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| s.never.into());
    let status = state.status_message.as_deref().unwrap_or("");
    let search = if state.is_search_active {
        format!("  {}{}_", s.search_prompt, state.search_input)
    } else {
        String::new()
    };

    let line = Line::from(vec![Span::styled(
        format!(
            "{}: {} | [{}]: {} | [{}]: {}s  {}{}",
            s.data_source_label,
            data_source_label(&state.data_source),  // Sina/AkShare name (not i18n'd — brand name)
            s.updated_label,
            updated,
            s.refresh_label,
            refresh_interval,
            status,
            search
        ),
        Style::default().fg(Color::DarkGray),
    )]);
    f.render_widget(Paragraph::new(line), area);
}
```

Note: `data_source_label()` returns the brand name ("新浪" / "AkShare") — for "新浪" keep the Chinese name since it's a brand. Actually "新浪" should also be translated. Use `s.data_source_label_sina` concept: the provider name for Sina should be translated. The current `data_source_label()` returns "新浪" for Sina and "AkShare" for AkShare. Let's update it to use strings:

```rust
pub(crate) fn data_source_name(data_source: &DataSourceKind, s: &'static crate::i18n::Strings) -> &'static str {
    match data_source {
        DataSourceKind::Sina => s.data_source_label,  // "新浪" / "Sina"
        DataSourceKind::AkShare => "AkShare",
    }
}
```

Wait — looking at the Strings struct, `data_source_label` was defined as the label prefix ("数据源" / "Source"). The sina brand name is "新浪". Add a separate field `data_source_sina: &'static str` = "新浪" / "Sina" to the `Strings` struct, or just hardcode "AkShare" and translate "新浪". Add to Strings:

```rust
pub data_source_sina: &'static str,  // "新浪" / "Sina"
```

ZH: `data_source_sina: "新浪"`, EN: `data_source_sina: "Sina"`.

Then update `i18n.rs` with this field (goes back to Task 1 code — add it there), or add it here. Add it in this task: open `i18n.rs`, add the field to the `Strings` struct and both constants.

Actually this is a small addition. Just add `pub data_source_sina: &'static str,` to the struct, and `data_source_sina: "新浪"` to ZH and `data_source_sina: "Sina"` to EN.

In `mod.rs`, update `data_source_label()` function to accept strings:
```rust
pub(crate) fn data_source_label(data_source: &DataSourceKind, s: &'static crate::i18n::Strings) -> &'static str {
    match data_source {
        DataSourceKind::Sina => s.data_source_sina,
        DataSourceKind::AkShare => "AkShare",
    }
}
```

Update all callers of `data_source_label` (chart.rs calls `super::data_source_label(data_source)` — update in Task 6).

- [ ] **Step 3: Update `watchlist.rs`**

Change block title:
```rust
let block_title = if state.is_add_active {
    format!("{}{}█ ", state.strings().add_title_prefix, state.add_input)
} else {
    state.strings().watchlist_title.to_string()
};
```

- [ ] **Step 4: Update `portfolio.rs`**

Change `.title(" Portfolio ")` to `.title(state.strings().portfolio_title)`.

- [ ] **Step 5: Update `detail.rs`**

```rust
vec![Line::from(format!("  {}  {}...", label, s.detail_loading))]
// and:
"Select a symbol to view details"  →  s.detail_select_hint
// format string with detail labels:
Line::from(format!(
    "  {}: {:.2}   {}: {}   {}: {}   {}: {}",
    s.detail_latest, q.price,
    s.detail_open, q.open.map(...).unwrap_or("--".into()),
    s.detail_high, q.high.map(...).unwrap_or("--".into()),
    s.detail_low, q.low.map(...).unwrap_or("--".into()),
)),
Line::from(format!(
    "  {}: {}",
    s.detail_volume,
    q.volume.map(|v| {
        if v >= 100_000_000 { format!("{:.2}{}", v as f64 / 1e8, s.detail_vol_unit_yi) }
        else if v >= 10_000 { format!("{:.2}{}", v as f64 / 1e4, s.detail_vol_unit_wan) }
        else { format!("{}", v) }
    }).unwrap_or("--".into()),
)),
```

Block title: `.title(s.detail_title)`.

- [ ] **Step 6: Run tests**

```bash
cargo test -p fa-tui 2>&1 | tail -15
```

Expected: all tests pass. Some tests check for "数据源:" chars — update those tests to be language-agnostic or check for the English equivalent. The tests in `mod.rs` (`test_draw_main_shows_data_source`) check `"数据源:".chars()` — these will need updating since the default state uses Zh, so they'll still pass. But if a future test uses En, update accordingly.

- [ ] **Step 7: Commit**

```bash
git add crates/fa-tui/src/i18n.rs crates/fa-tui/src/ui/mod.rs crates/fa-tui/src/ui/watchlist.rs crates/fa-tui/src/ui/portfolio.rs crates/fa-tui/src/ui/detail.rs
git commit -m "feat(i18n): replace hardcoded strings in statusbar, watchlist, portfolio, detail"
```

---

### Task 6: Replace UI strings in `chart.rs` and `backtest.rs`

**Files:**
- Modify: `crates/fa-tui/src/ui/chart.rs`
- Modify: `crates/fa-tui/src/ui/backtest.rs`
- Modify: `crates/fa-tui/src/ui/mod.rs` (update call sites)

These render functions don't take `&State` directly. Add `strings: &'static crate::i18n::Strings` parameter.

- [ ] **Step 1: Update `chart.rs`**

Change `pub fn render` signature:
```rust
pub fn render(f: &mut Frame, cs: &ChartState, data_source: &DataSourceKind, strings: &'static crate::i18n::Strings, area: Rect)
```

Pass strings through to internal helpers. Update `render_statusbar`:
```rust
fn render_statusbar(f: &mut Frame, data_source: &DataSourceKind, strings: &'static crate::i18n::Strings, area: Rect) {
    let src_name = match data_source {
        crate::app::DataSourceKind::Sina => strings.data_source_sina,
        crate::app::DataSourceKind::AkShare => "AkShare",
    };
    // chart_help contains "{}" placeholder for data source name:
    let text = format!("{}", strings.chart_help.replacen("{}", src_name, 1));
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}
```

Update "Loading data..." and "No data available" in `render_chart`:
```rust
let msg = if cs.loading { strings.chart_loading } else { strings.chart_no_data };
```

The `chart_max_history` string (currently "已显示最多历史数据") is dispatched as a `StatusMessage` from `event.rs`. Update `event.rs` to read it from state:

In `event.rs` `resolve_action()`:
```rust
// Instead of hardcoded Chinese, read from current state strings:
Some(AppAction::StatusMessage(state.strings().chart_max_history.to_string()))
```

Since `resolve_action` already receives `&State`, this works.

- [ ] **Step 2: Update `backtest.rs`**

Change signature: `pub fn render(f: &mut Frame, bs: &BacktestState, strings: &'static crate::i18n::Strings, area: Rect)`

Replace all hardcoded Chinese strings:
- `" [r 运行]"` → `strings.bt_status_idle`
- `" ⏳ 运行中..."` → `strings.bt_status_running`
- `" ✅ 完成"` → `strings.bt_status_done`
- `"股票: "` → `strings.bt_label_stock`
- `"策略: "` → `strings.bt_label_strategy`
- `"资金: "` → `strings.bt_label_cash`
- `"手续费: "` → `strings.bt_label_commission`
- `"滑点:   "` → `strings.bt_label_slippage`
- `"时间: "` → `strings.bt_label_period`
- `"← → 切换策略  r 运行  Esc 返回"` → `strings.bt_nav_help`
- `" 回测配置 "` → `strings.bt_config_title`
- `"正在运行回测..."` → `strings.bt_running_msg`
- `"按 r 开始回测"` → `strings.bt_press_r`
- `"总收益:  "` → `strings.bt_label_total_return`
- `"年化:  "` → `strings.bt_label_annualized`
- `"最大回撤: "` → `strings.bt_label_max_drawdown`
- `"胜率:  "` → `strings.bt_label_win_rate`
- `"Sharpe:   "` → `strings.bt_label_sharpe`
- `"交易次数: "` → `strings.bt_label_trades`
- `"初始: "` → `strings.bt_label_initial`
- `"最终: "` → `strings.bt_label_final`
- `" 回测结果 "` → `strings.bt_result_title`
- `" 暂无交易记录"` → `strings.bt_no_trades`
- `"日期", "操作", "价格", "数量", "金额", "盈亏"` in header → `strings.bt_trade_date`, etc.
- `"买入"` / `"卖出"` → `strings.bt_trade_buy` / `strings.bt_trade_sell`
- `" 交易记录  ↑/↓ 滚动 "` → `strings.bt_trades_title`

- [ ] **Step 3: Update `mod.rs` call sites**

```rust
// In draw():
AppScreen::Chart(cs) => {
    chart::render(f, cs, &state.data_source, state.strings(), f.area());
}
AppScreen::Backtest(bs) => {
    backtest::render(f, bs, state.strings(), f.area());
}
```

Remove the now-unused `data_source_label` function (or keep for reference — but confirm no callers remain).

- [ ] **Step 4: Run tests**

```bash
cargo test -p fa-tui 2>&1 | tail -15
```

Expected: all pass. The `test_render_backtest_*` tests check for "策略" or "回测" — they'll still pass since default language is Zh. If a test fails due to English, check which string key it uses.

- [ ] **Step 5: Commit**

```bash
git add crates/fa-tui/src/ui/chart.rs crates/fa-tui/src/ui/backtest.rs crates/fa-tui/src/ui/mod.rs crates/fa-tui/src/event.rs
git commit -m "feat(i18n): replace hardcoded strings in chart and backtest renderers"
```

---

### Task 7: Settings UI — add language row

**Files:**
- Modify: `crates/fa-tui/src/ui/settings.rs`

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_draw_settings_shows_language_row() {
    use crate::app::{DataSourceKind, SettingsState};
    use crate::i18n::Language;
    let ss = SettingsState::new(DataSourceKind::Sina, "http://localhost".into(), Language::Zh);
    let backend = ratatui::backend::TestBackend::new(80, 14);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_settings(f, f.area(), &ss)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content.iter().map(|c| c.symbol()).collect();
    assert!(content.contains("Language") || content.contains("语言"), "language row must appear");
    assert!(content.contains("中文") || content.contains("English"));
}
```

- [ ] **Step 2: Run test — expect fail**

- [ ] **Step 3: Update `draw_settings` function signature** to accept strings:

```rust
pub fn draw_settings(f: &mut Frame, area: Rect, ss: &SettingsState)
```

The function currently doesn't take strings. Since `SettingsState` now has a `language` field, retrieve strings inside:

```rust
pub fn draw_settings(f: &mut Frame, area: Rect, ss: &SettingsState) {
    use crate::i18n::{Language, ZH, EN};
    let s: &'static crate::i18n::Strings = match ss.language {
        Language::Zh => &ZH,
        Language::En => &EN,
    };
    // ... rest of function uses s.settings_title etc.
```

- [ ] **Step 4: Add one more row constraint** to the layout (was 7 rows including Constraint::Min(0) gap; add 1 more Length(1) for language row):

Current constraints: `[Length(1), Length(1), Length(1), Length(1), Length(1), Min(0), Length(1)]` → 7 rows (row indices 0-6).

New constraints: `[Length(1), Length(1), Length(1), Length(1), Length(1), Length(1), Min(0), Length(1)]` → 8 rows (indices 0-7).

Map:
- rows[0]: gap
- rows[1]: data source (focused_field == 0)
- rows[2]: gap
- rows[3]: URL (focused_field == 1)
- rows[4]: gap
- rows[5]: language (focused_field == 2) ← NEW
- rows[6]: Min(0) spacer
- rows[7]: help text

- [ ] **Step 5: Replace hardcoded strings with `s.*` equivalents** in `draw_settings`:

- `"⚙ 系统设置 (按 Esc 取消 / Enter 保存)"` → `s.settings_title`
- `"> 数据源      "` → `format!("> {}  ", s.settings_data_source)` (prefix > or spaces based on focus)
- `"> AkShare URL "` → `format!("> {}", s.settings_akshare_url)`
- `"(仅 AkShare 使用)"` → `s.settings_akshare_only`
- `"↑↓ 切换 | Space 选择 | Enter 保存 | Esc 取消"` → `s.settings_help`

- [ ] **Step 6: Render language row** at `rows[5]`:

```rust
let lang_line = Line::from(vec![
    Span::styled(
        if ss.focused_field == 2 {
            format!("> {} ", s.settings_language)
        } else {
            format!("  {} ", s.settings_language)
        },
        if ss.focused_field == 2 { focused_prefix } else { label_style },
    ),
    Span::styled(
        format!("[{}]", s.settings_lang_zh),
        if ss.language == Language::Zh { active_button } else { inactive_button },
    ),
    Span::raw(" "),
    Span::styled(
        format!("[{}]", s.settings_lang_en),
        if ss.language == Language::En { active_button } else { inactive_button },
    ),
]);
f.render_widget(Paragraph::new(lang_line), rows[5]);
```

- [ ] **Step 7: Update `mod.rs` call** — `draw_settings` signature hasn't changed (strings are derived from `ss.language` internally), so no change needed in caller.

- [ ] **Step 8: Run tests**

```bash
cargo test -p fa-tui -- settings 2>&1
```

Expected: all settings tests pass including new language row test.

- [ ] **Step 9: Commit**

```bash
git add crates/fa-tui/src/ui/settings.rs
git commit -m "feat(i18n): add language selection row to settings UI"
```

---

### Task 8: Quit confirmation overlay + event routing

**Files:**
- Modify: `crates/fa-tui/src/ui/mod.rs`
- Modify: `crates/fa-tui/src/event.rs`

- [ ] **Step 1: Write failing test** for overlay rendering (in `mod.rs` test module):

```rust
#[test]
fn test_draw_quit_confirm_overlay_appears() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = State::default();
    state.confirm_quit = true;
    terminal.draw(|f| draw(f, &state, 5)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    // Default language is Zh
    assert!("确认".chars().all(|c| content.contains(c)), "quit dialog must show in Chinese by default");
}
```

- [ ] **Step 2: Run test — expect fail** (overlay not rendered yet)

- [ ] **Step 3: Add `render_quit_confirm` function** to `mod.rs`:

```rust
use ratatui::layout::{Constraint, Direction, Flex};
use ratatui::widgets::{Clear, Padding};

fn render_quit_confirm(f: &mut Frame, state: &State) {
    let s = state.strings();
    let area = f.area();

    // Center a 36×9 popup
    let popup_w = 36u16.min(area.width);
    let popup_h = 9u16.min(area.height);
    let x = area.x + (area.width.saturating_sub(popup_w)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_h)) / 2;
    let popup_area = Rect::new(x, y, popup_w, popup_h);

    f.render_widget(Clear, popup_area);

    let block = Block::default().borders(Borders::ALL).title(format!(" {} ", s.quit_title.trim()));
    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    // Button row
    let confirm_style = if !state.confirm_quit_focused {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    let cancel_style = if state.confirm_quit_focused {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let btn_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(s.quit_yes, confirm_style),
        Span::raw("   "),
        Span::styled(s.quit_no, cancel_style),
    ]);
    f.render_widget(Paragraph::new(btn_line).alignment(Alignment::Center), rows[1]);

    let help_line = Line::from(Span::styled(s.quit_help, Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), rows[3]);
}
```

- [ ] **Step 4: Call `render_quit_confirm` at the END of `draw()`** (after all screens render):

```rust
pub fn draw(f: &mut Frame, state: &State, refresh_interval: u64) {
    match &state.screen {
        // ... existing arms ...
    }
    // Overlay: quit confirmation (rendered on top of any screen)
    if state.confirm_quit {
        render_quit_confirm(f, state);
    }
}
```

- [ ] **Step 5: Update `event.rs` — change `'q'` dispatch to `RequestQuit`**

Find and replace the `'q'` → `Quit` dispatches:

In `map_key_main`:
```rust
(KeyCode::Char('q'), KeyModifiers::NONE) => Some(AppAction::RequestQuit),
```

In `map_key_chart`:
```rust
(KeyCode::Char('q'), KeyModifiers::NONE) => Some(AppAction::RequestQuit),  // exits chart (was Quit)
```

Wait — in chart mode, `'q'` currently just quits the app. But users might expect `Esc` to exit the chart and `q` to quit the app. Check the current map. After this change, `q` everywhere shows the confirm dialog, and `Esc` exits chart. This is consistent behavior.

In `map_key_backtest`:
```rust
(KeyCode::Char('q'), KeyModifiers::NONE) => Some(AppAction::ExitBacktest),  // keep as-is (q exits backtest)
```

Actually checking: in backtest, `'q'` exits the backtest (goes back to main). Only main screen and chart `'q'` should trigger quit confirm. Leave `map_key_backtest` as-is.

- [ ] **Step 6: Add quit-confirm routing at TOP of `resolve_action()`** (before screen-specific routing):

```rust
fn resolve_action(state: &State, code: KeyCode, modifiers: KeyModifiers) -> Option<AppAction> {
    // Force-quit: Ctrl+C always exits immediately (no confirmation)
    if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
        return Some(AppAction::Quit);
    }

    // Quit confirmation dialog intercepts all keys when visible
    if state.confirm_quit {
        return match code {
            KeyCode::Char('y') | KeyCode::Enter => Some(AppAction::Quit),
            KeyCode::Char('n') | KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::CancelQuit),
            KeyCode::Left | KeyCode::Right => Some(AppAction::ToggleQuitButton),
            _ => None,
        };
    }

    // ... rest of existing routing ...
}
```

- [ ] **Step 7: Write failing test for quit confirm key routing** (add to `event.rs` tests):

```rust
#[test]
fn test_quit_confirm_y_dispatches_quit() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut state = State::default();
    state.confirm_quit = true;
    let action = EventHandler::resolve_action(&state, KeyCode::Char('y'), KeyModifiers::NONE);
    assert_eq!(action, Some(AppAction::Quit));
}

#[test]
fn test_quit_confirm_esc_cancels() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let mut state = State::default();
    state.confirm_quit = true;
    let action = EventHandler::resolve_action(&state, KeyCode::Esc, KeyModifiers::NONE);
    assert_eq!(action, Some(AppAction::CancelQuit));
}

#[test]
fn test_q_dispatches_request_quit_on_main_screen() {
    use crossterm::event::{KeyCode, KeyModifiers};
    let state = State::default();
    let action = EventHandler::resolve_action(&state, KeyCode::Char('q'), KeyModifiers::NONE);
    assert_eq!(action, Some(AppAction::RequestQuit));
}
```

Run: `cargo test -p fa-tui -- event::tests::test_quit 2>&1`

- [ ] **Step 8: Run all tests**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass. If any test used `AppAction::Quit` as the expected action for `'q'`, update it to `AppAction::RequestQuit`.

- [ ] **Step 9: Commit**

```bash
git add crates/fa-tui/src/ui/mod.rs crates/fa-tui/src/event.rs
git commit -m "feat: add quit confirmation modal dialog with i18n support"
```

---

### Task 9: Final verification and push

**Files:** None (verification only)

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace 2>&1 | grep -E "FAILED|passed|failed"
```

Expected: 0 failed, all crates green.

- [ ] **Step 2: Build release binary to confirm no warnings**

```bash
cargo build --release 2>&1 | grep -E "error|warning" | grep -v "unused imports" | head -20
```

Expected: clean build (warnings about unused imports are acceptable if they're pre-existing).

- [ ] **Step 3: Smoke-test manually** (optional if running in a terminal):

```
cargo run
# Press 's' to open settings — verify language row appears
# Press Space on language to toggle Zh/En
# Press Enter to save — verify UI switches language
# Press 'q' — verify quit dialog appears
# Press 'n' — verify dialog dismisses
# Press 'q' again, then 'y' — verify app exits
```

- [ ] **Step 4: Push to remote**

```bash
git push origin main
```
