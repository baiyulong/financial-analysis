# Design: Multi-language Support & Exit Confirmation

## Overview

Two independent UI improvements:
1. **i18n**: Switch between Chinese (default) and English in the settings screen; persisted in SQLite.
2. **Quit confirmation**: Pressing `q` shows a centered modal dialog instead of immediately exiting.

---

## Feature 1: Multi-language Support (i18n)

### Languages
- Chinese (`zh`, default)
- English (`en`)

### Architecture

**New file: `crates/fa-tui/src/i18n.rs`**

Defines `Language` enum and a `Strings` struct holding all UI text as `&'static str` fields. Two static constants (`ZH`, `EN`) provide the translations.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language { Zh, En }

pub struct Strings {
    // Common
    pub confirm_yes: &'static str,   // "确认" / "Yes"
    pub confirm_no: &'static str,    // "取消" / "No"
    // Main screen
    pub watchlist: &'static str,     // "自选股" / "Watchlist"
    pub portfolio: &'static str,     // "持仓" / "Portfolio"
    pub updated_at: &'static str,    // "更新" / "Updated"
    pub never: &'static str,         // "从未" / "Never"
    pub data_source: &'static str,   // "数据源" / "Source"
    pub refresh: &'static str,       // "刷新" / "Refresh"
    pub add_stock: &'static str,     // "添加" / "Add"
    pub search_hint: &'static str,   // "搜索股票..." / "Search stocks..."
    // Chart
    pub loading: &'static str,       // "加载中..." / "Loading..."
    pub max_history: &'static str,   // "已显示最多历史数据" / "Max history loaded"
    pub kline_help: &'static str,    // "←→移动 +/-缩放 q退出" / "←→ move +/- zoom q quit"
    // Backtest
    pub backtest_title: &'static str,
    pub strategy: &'static str,
    pub run: &'static str,
    pub trades: &'static str,
    pub return_: &'static str,
    pub win_rate: &'static str,
    pub max_drawdown: &'static str,
    // Settings
    pub settings_title: &'static str,  // "设置" / "Settings"
    pub settings_data_source: &'static str,
    pub settings_language: &'static str,  // "语言" / "Language"
    pub settings_akshare_url: &'static str,
    pub settings_save: &'static str,
    pub settings_cancel: &'static str,
    pub settings_help: &'static str,
    // Quit confirmation
    pub quit_title: &'static str,    // "确认退出？" / "Quit?"
    pub quit_help: &'static str,     // "y/Enter 确认  n/Esc 取消" / "y/Enter confirm  n/Esc cancel"
    // Language names (displayed in settings)
    pub lang_zh: &'static str,       // "中文" / "中文"
    pub lang_en: &'static str,       // "English" / "English"
}

pub static ZH: Strings = Strings { /* Chinese values */ };
pub static EN: Strings = Strings { /* English values */ };
```

**`State` integration:**

```rust
// In State struct
pub language: Language,

impl State {
    pub fn strings(&self) -> &'static Strings {
        match self.language {
            Language::Zh => &i18n::ZH,
            Language::En => &i18n::EN,
        }
    }
}
```

All UI draw functions access text via `state.strings().xxx` instead of hardcoded strings.

### Settings Integration

`SettingsState` gains a `language: Language` field. The settings UI renders a new row:

```
語言/Language   ◉ 中文   ○ English
```

New action: `AppAction::SettingsSelectLanguage(Language)`.

On `SettingsSaved`, the language is immediately applied to `State.language` and persisted to SQLite as key `"language"` (value `"zh"` or `"en"`). The entire UI re-renders in the new language on the next frame.

### Persistence

SQLite `settings` table (already exists):
- Key: `"language"`, Value: `"zh"` | `"en"`
- Read on startup in `build_initial_state()`
- Written in `SettingsSaved` handler alongside data source

### String Coverage

All user-visible strings in these UI files are extracted:
- `ui/layout.rs` — help bar, panel titles
- `ui/statusbar.rs` — data source label, timestamp
- `ui/watchlist.rs` — search/add prompts, column headers
- `ui/portfolio.rs` — column headers, totals
- `ui/chart.rs` — loading, help text, history messages
- `ui/backtest.rs` — strategy label, result fields
- `ui/settings.rs` — field labels, help text, language selector
- `ui/mod.rs` — quit confirmation overlay

Technical strings (error messages, API logs) remain in English as they are developer-facing.

---

## Feature 2: Exit Confirmation Dialog

### State

`State` gains `pub confirm_quit: bool` (default `false`). This is an overlay flag — the underlying screen (`AppScreen`) does not change.

### Actions

| Action | Trigger | Effect |
|--------|---------|--------|
| `RequestQuit` | User presses `q` in any screen | `state.confirm_quit = true` |
| `ToggleQuitButton` | User presses `←` or `→` in dialog | Flips `state.confirm_quit_focused` |
| `CancelQuit` | User presses `n`, `q`, or `Esc` in dialog | `state.confirm_quit = false` |
| `Quit` (existing) | User presses `y` or `Enter` in dialog, OR `Ctrl+C` anywhere | `state.should_quit = true` |

`Ctrl+C` continues to dispatch `Quit` directly (no confirmation) — it signals force-exit intent.

### Dialog Rendering

Rendered in `ui/mod.rs` as a top-level overlay after all other content. Uses Ratatui `Clear` to erase the background:

```
         ┌──────────────────────────┐
         │                          │
         │       确认退出？          │
         │                          │
         │    [ 确认 ]  [ 取消 ]    │
         │                          │
         └──────────────────────────┘
           y/Enter 确认   n/Esc 取消
```

- Centered in terminal (30×8 fixed size)
- Active button highlighted with bold/reversed style
- `←`/`→` switches focused button (confirm vs cancel)
- `Enter` activates the focused button
- `y` directly confirms; `n`/`q`/`Esc` directly cancels

Dialog text is i18n-aware: uses `state.strings().quit_title` and `state.strings().quit_help`.

### Keyboard Routing

In `event.rs`, `resolve_action()` checks `state.confirm_quit` before any other screen routing:

```rust
if state.confirm_quit {
    return match code {
        KeyCode::Char('y') | KeyCode::Enter => Some(AppAction::Quit),
        KeyCode::Char('n') | KeyCode::Char('q') | KeyCode::Esc => Some(AppAction::CancelQuit),
        KeyCode::Left | KeyCode::Right => Some(AppAction::ToggleQuitButton),
        _ => None,
    };
}
```

`Ctrl+C` check remains before the `confirm_quit` check (retains force-quit behavior).

### Quit button in State

`State` gains `confirm_quit_focused: bool` (which button is highlighted: `false` = "确认", `true` = "取消"). `ToggleQuitButton` action flips it.

---

## Implementation Order

1. `i18n.rs` — Language enum + Strings struct + ZH/EN constants
2. `State` — add `language`, `confirm_quit`, `confirm_quit_focused` fields
3. `Storage` — load/save language setting
4. `Settings` UI — add language row, new action
5. All UI files — replace hardcoded strings with `state.strings().xxx`
6. Quit confirmation — new actions + overlay rendering + keyboard routing
7. Tests — new unit tests for actions; update existing tests that check hardcoded strings

---

## Out of Scope

- Runtime locale detection (always default to Chinese)
- Pluralization rules
- Right-to-left language support
- Translating error messages or log output
