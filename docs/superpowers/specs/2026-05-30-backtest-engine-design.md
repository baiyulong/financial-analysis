# Phase 3：回测引擎 + 策略框架 设计文档

**日期：** 2026-05-30  
**技术栈：** Rust + Ratatui + fa-backtest crate  
**方案：** 方案 A — 简单 bar-by-bar 循环

---

## 1. 概述

在现有系统基础上新增**日线回测引擎**，支持：
- 内置 3 种经典策略（双均线、RSI、布林带）
- 手续费（基点）+ 滑点模型
- TUI 内交互式运行（按 `b` 进入回测模式）
- 结果展示：关键指标 + 交易记录 + CSV 导出

数据来源：复用 `fa-data` 已有的 Yahoo Finance OHLCV 接口。

---

## 2. Crate 结构

### 2.1 新增 `fa-backtest` crate

```
crates/fa-backtest/src/
├── lib.rs           # 导出公共 API
├── engine.rs        # Engine::run() 主循环
├── strategy.rs      # Strategy trait + Signal + BarContext
├── portfolio.rs     # 持仓/现金/成本跟踪（内部）
├── result.rs        # BacktestResult + 指标计算 + CSV 导出
└── strategies/
    ├── mod.rs
    ├── ma_cross.rs  # 双均线穿越策略
    ├── rsi.rs       # RSI 超买超卖策略
    └── bollinger.rs # 布林带策略
```

### 2.2 依赖方向（严格单向）

```
fa-tui ──► fa-backtest ──► fa-indicator ──► fa-core
                      ──► fa-core
```

`fa-backtest` 不依赖 `fa-data`（数据由调用方传入 `Vec<OHLCV>`）。

---

## 3. 核心 API

### 3.1 `Strategy` trait

```rust
pub trait Strategy: Send {
    fn name(&self) -> &str;
    /// 每根 K 线收盘后调用，返回交易信号
    fn on_bar(&mut self, ctx: &BarContext) -> Signal;
    /// 重置策略状态（复用同一实例运行多次回测时调用）
    fn reset(&mut self);
}

pub struct BarContext<'a> {
    pub bar: &'a OHLCV,       // 当前 K 线（含 OHLCV）
    pub position: i64,        // 当前持股数（0 = 空仓）
    pub cash: Decimal,        // 当前可用现金
    pub history: &'a [OHLCV], // 截至当前的历史 K 线（含当前 bar）
}

pub enum Signal {
    BuyAll,          // 全仓买入（以成交价成交）
    SellAll,         // 全仓卖出
    Buy(Decimal),    // 买入指定金额（自动计算股数）
    Sell(i64),       // 卖出指定股数
    Hold,            // 不操作
}
```

### 3.2 `BacktestConfig`

```rust
pub struct BacktestConfig {
    pub initial_cash: Decimal,    // 初始资金，默认 100_000
    pub commission_bps: Decimal,  // 手续费基点，如 5.0 = 0.05%
    pub slippage_bps: Decimal,    // 滑点基点，如 3.0 = 0.03%
    pub start_date: NaiveDate,    // 回测开始日（含）
    pub end_date: NaiveDate,      // 回测结束日（含）
}
```

### 3.3 成交价格计算

- 买入成交价 = `close * (1 + slippage_bps / 10000)`（模拟以略高于收盘价成交）
- 卖出成交价 = `close * (1 - slippage_bps / 10000)`
- 每笔手续费 = `成交金额 * commission_bps / 10000`（买卖均收）

### 3.4 `Engine::run()`

```rust
pub struct Engine {
    config: BacktestConfig,
}

impl Engine {
    pub fn run(
        &self,
        data: &[OHLCV],         // 完整历史 K 线，已按日期升序排列
        strategy: &mut dyn Strategy,
    ) -> BacktestResult;
}
```

引擎流程：
1. 过滤 `data` 到 `[start_date, end_date]` 范围
2. 初始化 `Portfolio`（初始现金，持仓 0）
3. 逐根 K 线调用 `strategy.on_bar(&ctx)` → `Signal`
4. 应用信号（BuyAll/SellAll 等），计算成交价，扣除手续费
5. 记录每笔交易到 `TradeLog`
6. 返回 `BacktestResult`

### 3.5 `BacktestResult`

```rust
pub struct BacktestResult {
    pub total_return: Decimal,      // 总收益率，如 0.234 = 23.4%
    pub annualized_return: Decimal, // 年化收益率
    pub max_drawdown: Decimal,      // 最大回撤（负数），如 -0.123 = -12.3%
    pub win_rate: Decimal,          // 胜率（盈利交易数 / 总交易次数）
    pub sharpe_ratio: Decimal,      // Sharpe 比率（无风险利率取 3%）
    pub total_trades: usize,        // 总交易次数（买+卖）
    pub trades: Vec<Trade>,         // 详细交易记录
    pub equity_curve: Vec<Decimal>, // 每日末资产净值序列
}

pub struct Trade {
    pub date: NaiveDate,
    pub action: TradeAction,        // Buy / Sell
    pub price: Decimal,             // 成交价
    pub quantity: i64,              // 股数
    pub amount: Decimal,            // 成交金额（含手续费）
    pub pnl: Option<Decimal>,       // 本笔盈亏（卖出时计算，买入为 None）
}

pub enum TradeAction { Buy, Sell }
```

CSV 导出：`BacktestResult::export_csv(path: &Path) -> std::io::Result<()>`，输出所有 `trades` 字段。

---

## 4. 内置策略

### 4.1 双均线穿越（`MaCrossStrategy`）

| 参数 | 默认值 | 说明 |
|---|---|---|
| `fast` | 5 | 快线周期 |
| `slow` | 20 | 慢线周期 |

- 进场：fast MA 上穿 slow MA（前一根 fast < slow，当前 fast >= slow）→ `BuyAll`
- 出场：fast MA 下穿 slow MA → `SellAll`
- 周期不足时（历史数据 < slow）→ `Hold`

### 4.2 RSI 均值回归（`RsiStrategy`）

| 参数 | 默认值 | 说明 |
|---|---|---|
| `period` | 14 | RSI 计算周期 |
| `oversold` | 30 | 超卖阈值 |
| `overbought` | 70 | 超买阈值 |

- 进场：RSI < oversold → `BuyAll`
- 出场：RSI > overbought → `SellAll`
- 数据不足时 → `Hold`

### 4.3 布林带（`BollingerStrategy`）

| 参数 | 默认值 | 说明 |
|---|---|---|
| `period` | 20 | 均线周期 |
| `std_dev` | 2.0 | 标准差倍数 |

- 进场：收盘价 <= 下轨 → `BuyAll`
- 出场：收盘价 >= 上轨 → `SellAll`
- 数据不足时 → `Hold`

---

## 5. TUI 集成

### 5.1 新增 Screen

```rust
pub enum AppScreen {
    Main,
    Chart(ChartState),
    Backtest(BacktestState),  // 新增
}

pub struct BacktestState {
    pub symbol: Symbol,
    pub strategy_idx: usize,          // 选中的策略下标
    pub config: BacktestConfig,
    pub result: Option<BacktestResult>,
    pub trade_scroll: usize,          // 交易记录滚动位置
    pub status: BacktestStatus,
}

pub enum BacktestStatus {
    Idle,
    Running,
    Done,
    Error(String),
}
```

### 5.2 键盘操作

| 按键 | 功能 |
|---|---|
| `b`（主界面） | 进入选中股票的回测界面 |
| `↑` / `↓` | 切换策略 |
| `Tab` | 在「配置区」和「交易记录」间切换焦点 |
| `r` | 运行回测（异步，完成后刷新） |
| `e` | 导出交易记录到 `~/fa-backtest-<symbol>-<date>.csv` |
| `Esc` | 返回主界面 |

### 5.3 布局（2 行 / 2 列）

```
┌─ 回测配置 ──────────────┐ ┌─ 回测结果 ────────────────────────────────┐
│ 股票: AAPL              │ │ 总收益:  +23.4%   年化:  +8.7%            │
│ 策略: [双均线穿越 ▼]    │ │ 最大回撤: -12.3%  胜率:  58.3%            │
│ 时间: 2022-01 ~ 今      │ │ Sharpe:  1.24    交易次数: 24             │
│ 资金: ¥100,000          │ └───────────────────────────────────────────┘
│ 手续费: 5bps            │ ┌─ 交易记录 ────────────────────────────────┐
│ 滑点:   3bps            │ │ 日期       操作  价格    数量   盈亏        │
│ [r 运行]  [e 导出]      │ │ 2022-03-15 买入  180.50  55股             │
└─────────────────────────┘ │ 2022-07-22 卖出  156.30  55股  -¥1,342   │
                            │ ↑/↓ 滚动 · e 导出 CSV                    │
                            └───────────────────────────────────────────┘
```

### 5.4 异步运行回测

回测触发后在 Tokio task 中执行（OHLCV 数据已缓存于 `ChartState` 或重新从 `fa-data` 获取），完成后通过 `AppAction::BacktestDone(BacktestResult)` 更新状态。

---

## 6. 文件变更清单

| 文件 | 变更类型 | 说明 |
|---|---|---|
| `Cargo.toml`（workspace） | 修改 | 添加 `fa-backtest` 成员 |
| `crates/fa-backtest/` | 新建 | 全部新建 |
| `crates/fa-tui/Cargo.toml` | 修改 | 添加 `fa-backtest` 依赖 |
| `crates/fa-tui/src/app.rs` | 修改 | 添加 `BacktestState`、相关 `AppAction` 变体 |
| `crates/fa-tui/src/event.rs` | 修改 | `b` 键触发 `StartBacktest`；回测界面键映射 |
| `crates/fa-tui/src/ui/backtest.rs` | 新建 | 回测界面渲染 |
| `crates/fa-tui/src/ui/mod.rs` | 修改 | 添加 `pub mod backtest` |
| `src/main.rs` | 修改 | 添加 `BacktestDone` action 处理 |
| `README.md` | 修改 | 添加 `b` 键说明 |

---

## 7. 测试策略

| 层次 | 测试内容 |
|---|---|
| `fa-backtest` 单元测试 | Engine 基本买卖逻辑、手续费计算、Sharpe 公式 |
| 策略测试 | 已知数据集验证信号生成（double MA golden cross 等） |
| `BacktestResult::export_csv` | 输出格式验证 |
| TUI 测试 | `TestBackend` 验证 BacktestScreen 渲染不 panic |

---

## 8. 实现顺序（建议）

1. `fa-backtest` crate：`Strategy` trait + `Engine` + `portfolio.rs` + `result.rs`
2. 内置策略：`ma_cross` → `rsi` → `bollinger`
3. `fa-tui` 集成：`app.rs` + `event.rs` + `ui/backtest.rs` + `main.rs`
4. 文档：README 更新

---

## 9. 范围边界（不包含）

- 分钟级 / Tick 级回测
- 多标的同时回测（组合回测）
- 参数优化（网格搜索）
- 持久化回测结果（不写数据库，仅 CSV）
- 图形化资金曲线（文本 ASCII 近似，不做精确曲线图）
