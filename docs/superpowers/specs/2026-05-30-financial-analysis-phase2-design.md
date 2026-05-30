# Financial Analysis TUI — Phase 2 设计文档

**日期：** 2026-05-30  
**范围：** K 线图全屏视图 + MA 技术指标（1/5/10/20/60 日均线）  
**依赖：** Phase 1 完成（fa-core / fa-data / fa-tui / main.rs 均已实现）

---

## 1. 目标

在 Phase 1 的基础上，为用户提供 K 线图全屏分析视图：
- 按 `Enter` 从 Watchlist 进入任意股票的全屏 K 线图
- 支持 5 个时间周期切换：1D / 5D / 1M / 3M / 1Y
- 叠加 MA5 / MA10 / MA20 均线（可配置）
- 光标移动查看历史数据，缩放 K 线宽度
- 按 `Esc` 返回主界面

---

## 2. 架构变更

### 2.1 新增 Crate：`fa-indicator`

```
fa-tui → fa-indicator → fa-core ← fa-data
```

`crates/fa-indicator/` — 纯数学计算，无 IO / TUI 依赖：

```
crates/fa-indicator/
├── Cargo.toml          # 依赖: fa-core
└── src/
    ├── lib.rs           # pub mod ma; pub use ma::sma;
    └── ma.rs            # Simple Moving Average 实现
```

**API：**
```rust
/// 简单移动平均。前 period-1 个值返回 None。
/// data 为时间升序 OHLCV 切片，使用收盘价计算。
pub fn sma(data: &[OHLCV], period: usize) -> Vec<Option<Decimal>>
```

### 2.2 Workspace 更新

`Cargo.toml` workspace members 新增 `"crates/fa-indicator"`。  
根 `Cargo.toml` `[dependencies]` 新增 `fa-indicator = { path = "crates/fa-indicator" }`。  
`crates/fa-tui/Cargo.toml` `[dependencies]` 新增 `fa-indicator = { path = "../fa-indicator" }`。

---

## 3. fa-core 扩展

### 3.1 Period 枚举扩展

Phase 1 已实现 `Period` 枚举。K 线图使用以下 5 个周期：

| Period 变体 | Yahoo range | Yahoo interval | K 线数量（约） |
|-------------|-------------|----------------|----------------|
| `OneDay`    | `1d`        | `5m`           | 78 根（分钟线） |
| `FiveDay`   | `5d`        | `15m`          | 130 根 |
| `OneMonth`  | `1mo`       | `1d`           | ~22 根（日线） |
| `ThreeMonth`| `3mo`       | `1d`           | ~66 根 |
| `OneYear`   | `1y`        | `1d`           | ~252 根 |

确认 `fa_core::Period` 已有上述变体（Phase 1 已实现），无需新增。

---

## 4. AppState 扩展

### 4.1 新类型

```rust
// crates/fa-tui/src/app.rs 新增

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
    pub data: Vec<OHLCV>,         // 时间升序
    pub cursor: usize,            // 当前高亮的 K 线索引（0 = 最旧）
    pub bar_width: u16,           // K 线像素宽（2~8），默认 3
    pub ma_periods: Vec<usize>,   // 显示的均线，默认 [5, 10, 20]
    pub loading: bool,            // true = 等待数据加载
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
    
    /// 当前光标指向的 OHLCV（若有数据）
    pub fn current_bar(&self) -> Option<&OHLCV> {
        self.data.get(self.cursor)
    }
}
```

### 4.2 State 扩展

```rust
// 在 State 结构体新增字段
pub screen: AppScreen,
```

`State::apply` 新增分支处理 Chart 相关 Action。

### 4.3 AppAction 新变体

```rust
// 新增到 AppAction enum
EnterChart(Symbol),                    // 进入 K 线图（默认周期 1M）
ExitChart,                             // 返回主界面
ChartDataLoaded(Vec<OHLCV>),           // K 线数据加载完毕
ChartMoveCursor(i32),                  // 光标移动（+1 右, -1 左）
ChartZoom(bool),                       // true = 放大, false = 缩小
ChartChangePeriod(Period),             // 切换时间周期
```

### 4.4 State::apply 逻辑

```rust
AppAction::EnterChart(sym) => {
    self.screen = AppScreen::Chart(ChartState::new(sym, Period::OneMonth));
}
AppAction::ExitChart => {
    self.screen = AppScreen::Main;
}
AppAction::ChartDataLoaded(data) => {
    if let AppScreen::Chart(ref mut cs) = self.screen {
        let len = data.len();
        cs.data = data;
        cs.cursor = len.saturating_sub(1); // 默认指向最新
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

---

## 5. EventHandler 扩展

`EventHandler::map_key` 新增 chart 模式键位（通过 `AppScreen` 状态判断）：

由于 `map_key` 是静态方法，需改为接收当前 screen 信息，或在 `run()` 中判断。

**方案：** `run()` 读取 `Arc<RwLock<AppScreen>>` 决定键位映射。但为保持 map_key 纯静态可测试，改为：

```rust
pub fn map_key(code: KeyCode, modifiers: KeyModifiers, screen: &AppScreen) -> Option<AppAction>
```

Chart 模式新键位：

| 键 | AppAction |
|----|-----------|
| `←` / `→` | `ChartMoveCursor(-1 / +1)` |
| `[` | `ChartZoom(false)` |
| `]` | `ChartZoom(true)` |
| `1` | `ChartChangePeriod(Period::OneDay)` |
| `5` | `ChartChangePeriod(Period::FiveDay)` |
| `m` | `ChartChangePeriod(Period::OneMonth)` |
| `q` | `ChartChangePeriod(Period::ThreeMonth)` |
| `y` | `ChartChangePeriod(Period::OneYear)` |
| `Esc` | `ExitChart` |

Main 模式额外新增：

| 键 | AppAction |
|----|-----------|
| `Enter` | `EnterChart(selected_symbol)` — 由 run() 注入当前选中 symbol |

---

## 6. UI：K 线图全屏面板

### 6.1 文件结构

```
crates/fa-tui/src/ui/
└── chart.rs    [NEW]  全屏 K 线图渲染
```

`ui/mod.rs` 新增 `pub mod chart;`

### 6.2 布局（全屏 3 行）

```
┌─────────────────────────────────────────────────────────────────────┐  ← row[0]: 标题栏 1行
│  AAPL · 1M  [1]1D [5]5D [m]1M [q]3M [y]1Y  [←→]光标 [[]缩放 Esc  │
├─────────────────────────────────────────────────────────────────────┤  ← row[1]: 图表区（主体）
│                                                                     │
│ $195 ┤      ┃        ┃                              ┃ ← 光标K线    │
│ $190 ┤  ─── ┃ ──── MA20     ════ MA10   ── MA5     ┃              │
│ $185 ┤  ┃  ████ ██ ████                            ████            │
│ $180 ┤  ┃  ████ ██ ████                            ████            │
│      └─────────────────────────────────────────────────────         │
│       4/1   4/8   4/15  4/22                       5/22            │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤  ← row[2]: 光标信息 1行
│ 2026-04-22  开:183.50  高:186.10  低:182.90  收:185.20  涨:+1.2%  │
└─────────────────────────────────────────────────────────────────────┘
```

### 6.3 渲染技术

使用 `ratatui::widgets::canvas::Canvas`：

```rust
Canvas::default()
    .block(Block::default().borders(Borders::ALL))
    .x_bounds([0.0, width as f64])
    .y_bounds([price_min, price_max])
    .paint(|ctx| {
        // 画每根 K 线
        for (i, bar) in visible_bars.iter().enumerate() {
            let x = i as f64 * bar_width;
            let color = if bar.close >= bar.open { Color::Red } else { Color::Green };
            // 上影线
            ctx.draw(&Line { x1: x+1.0, y1: bar.high, x2: x+1.0, y2: bar.open.max(bar.close), color });
            // 下影线
            ctx.draw(&Line { x1: x+1.0, y1: bar.low,  x2: x+1.0, y2: bar.open.min(bar.close), color });
            // 实体（用多条 Line 填充矩形）
            let body_top = bar.open.max(bar.close);
            let body_bot = bar.open.min(bar.close);
            ctx.draw(&Rectangle { x, y: body_bot, width: bar_width-0.5, height: body_top-body_bot, color });
        }
        // 画 MA 线
        for &period in &ma_periods {
            let ma_values = sma(&data, period);
            let ma_color = ma_color(period); // MA5=黄, MA10=橙, MA20=紫
            // 连接相邻非 None 的点
            ...
        }
        // 光标竖线高亮
        ctx.draw(&Line { x1: cursor_x, y1: price_min, x2: cursor_x, y2: price_max, color: Color::White });
    })
```

### 6.4 渲染函数签名

```rust
// crates/fa-tui/src/ui/chart.rs
pub fn render(f: &mut Frame, chart_state: &ChartState, area: Rect)
```

### 6.5 价格轴与日期轴

- Y 轴（价格）：根据可见 K 线的 high/low 确定范围，留 5% padding
- X 轴（日期）：每隔 N 根 K 线显示一个日期标签，N 由面板宽度决定
- 可见 K 线数量 = `(chart_width / bar_width)` 根，居中对齐光标

---

## 7. main.rs 扩展：ChartFetcher Task

当 `AppScreen` 变为 `Chart` 且 `loading == true` 时，启动 OHLCV 数据获取：

```rust
// 在 run_app 的 action 处理循环中：
AppAction::EnterChart(_) | AppAction::ChartChangePeriod(_) => {
    // 读取当前 chart state
    let (symbol, period) = { ... read from state ... };
    // 派发独立任务
    let router = Arc::clone(&router);
    let tx = tx.clone();
    tokio::spawn(async move {
        match router.fetch_ohlcv(&symbol, period).await {
            Ok(data) => { let _ = tx.send(AppAction::ChartDataLoaded(data)).await; }
            Err(e)   => { let _ = tx.send(AppAction::StatusMessage(format!("K线获取失败: {}", e))).await; }
        }
    });
}
```

---

## 8. 测试策略

| 测试目标 | 工具 | 内容 |
|---------|------|------|
| `fa-indicator::sma` | `#[test]` | 已知数据验证 MA 计算；period > data.len() 时全 None；period=1 时等于原数据 |
| `ChartState::apply` | `#[test]` | cursor clamp（不越界）；bar_width clamp（2~8）；period 切换重置 loading |
| `map_key` (chart mode) | `#[test]` | ← → [ ] 1 5 m q y Esc 各键映射正确 |
| `chart::render` | `TestBackend` | 渲染不 panic；含股票代码；Loading 时显示提示 |
| `sma` 边界 | `#[test]` | period=0 时返回空（或 panic with clear message） |

---

## 9. 文件变更汇总

### 新建
- `crates/fa-indicator/Cargo.toml`
- `crates/fa-indicator/src/lib.rs`
- `crates/fa-indicator/src/ma.rs`
- `crates/fa-tui/src/ui/chart.rs`

### 修改
- `Cargo.toml` — workspace members, root deps
- `crates/fa-tui/Cargo.toml` — 新增 fa-indicator dep
- `crates/fa-tui/src/app.rs` — AppScreen, ChartState, AppAction 扩展
- `crates/fa-tui/src/event.rs` — map_key 接收 screen 参数，chart 键位
- `crates/fa-tui/src/ui/mod.rs` — pub mod chart
- `src/main.rs` — ChartFetcher 任务，EnterChart/ChartChangePeriod 处理，chart 渲染分支

---

## 10. 不在 Phase 2 范围内

- MACD / RSI / 布林带（Phase 3）
- 成交量柱（可在 Phase 2.5 追加）
- K 线图内搜索 / 标注
- 历史数据本地持久化
- 分时图（intraday tick-level）
