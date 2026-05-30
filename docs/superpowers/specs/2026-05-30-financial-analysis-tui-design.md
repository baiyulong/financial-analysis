# Financial Analysis TUI — 设计文档

**日期：** 2026-05-30  
**技术栈：** Rust + Ratatui  
**目标用户：** 个人投资者 / 技术分析师 / 高级开发者

---

## 1. 项目概述

基于 Rust + Ratatui 构建的终端股票金融分析系统，支持多市场（A 股、美股、加密货币）实时行情监控、持仓管理、技术指标分析和回测引擎。面向高级 Rust 用户，注重性能与可扩展性。

### 阶段规划

| 阶段 | 内容 | 状态 |
|------|------|------|
| **Phase 1** | Core + 实时行情监控 + 持仓管理 | 📋 当前设计范围 |
| **Phase 2** | K 线图 + 技术指标 (MA/MACD/RSI/布林带) | 待设计 |
| **Phase 3** | 回测引擎 + 策略框架 | 待设计 |

---

## 2. 架构：Cargo Workspace + Tokio Actor 模型

### 2.1 Crate 结构

```
financial-analysis/          ← Cargo workspace 根
├── Cargo.toml               ← workspace 配置
├── crates/
│   ├── fa-core/             ← 核心数据模型 + trait 定义（无外部依赖）
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── market.rs    ← Market enum (AShare, USStock, Crypto...)
│   │       ├── symbol.rs    ← Symbol struct (code, market, name)
│   │       ├── quote.rs     ← Quote struct (price, change, volume...)
│   │       ├── ohlcv.rs     ← OHLCV struct for candlestick data
│   │       ├── portfolio.rs ← Portfolio, Position, PnL 计算
│   │       └── provider.rs  ← DataProvider trait (async)
│   ├── fa-data/             ← 数据源实现（依赖 fa-core，无 TUI 依赖）
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── yahoo.rs     ← Yahoo Finance API 客户端
│   │       ├── alpha.rs     ← Alpha Vantage API 客户端
│   │       ├── csv.rs       ← 本地 CSV/JSON 导入
│   │       └── cache.rs     ← 内存缓存层（失效策略）
│   └── fa-tui/              ← Ratatui TUI 渲染层（依赖 fa-core）
│       └── src/
│           ├── lib.rs
│           ├── app.rs       ← App struct, AppState, AppAction enum
│           ├── event.rs     ← EventHandler（键盘事件 → AppAction）
│           └── ui/
│               ├── mod.rs
│               ├── layout.rs    ← 根布局
│               ├── watchlist.rs ← 自选股面板
│               ├── portfolio.rs ← 持仓面板
│               ├── detail.rs    ← 个股详情面板
│               └── statusbar.rs ← 状态栏
└── src/
    └── main.rs              ← 组装入口，启动 tokio runtime
```

**依赖方向（严格单向）：**
```
fa-tui  ──► fa-core ◄── fa-data
 (渲染)      (模型)     (数据获取)
```

### 2.2 异步数据流（Actor 模型）

```
┌─────────────────────────────────────────────────────────────┐
│                         main.rs                             │
│                                                             │
│  ┌─────────────────┐   mpsc::channel<Vec<Quote>>           │
│  │  DataFetcher    │ ──────────────────────────────────►   │
│  │  Task (tokio)   │                         ┌──────────┐  │
│  │  轮询: 30s间隔  │                         │ AppState │  │
│  └─────────────────┘                         │Arc<RwLock>│ │
│                                              └─────┬────┘  │
│  ┌─────────────────┐   mpsc::channel<AppAction>   │        │
│  │  EventHandler   │ ─────────────────────────►   │        │
│  │  Task (tokio)   │                         ┌────▼─────┐  │
│  │  crossterm 事件 │                         │  TUI     │  │
│  └─────────────────┘                         │ Renderer │  │
│                                              │ 16ms tick│  │
│                                              └──────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**关键设计决策：**
- `DataFetcher` 每 30s 轮询数据源，通过 channel 推送更新
- `AppState` 用 `Arc<RwLock<State>>` 在多任务间共享，写少读多
- TUI 按 16ms tick 刷新，不阻塞等待数据（读 RwLock 非阻塞）
- 用户输入产生 `AppAction` enum，统一驱动状态变更

---

## 3. TUI 界面设计（Phase 1）

### 3.1 主界面布局

```
┌─ Financial Analysis ──────────────────── [Q]uit [/]Search [Tab]Focus ─┐
│                                                                         │
│  ┌─ Watchlist ───────────┐  ┌─ Portfolio ───────────────────────────┐  │
│  │ ● AAPL   185.20 +1.2% │  │  持仓         成本    市值    盈亏     │  │
│  │ ● TSLA   245.80 -0.8% │  │  AAPL x10   1800   1852   +52(+2.9%) │  │
│  │ ● 600519 1730.0 +0.3% │  │  TSLA x5    1100   1229  +129(+11.7%)│  │
│  │ ● BTC    67200  +3.5% │  │  ────────────────────────────────     │  │
│  │                       │  │  总资产: ¥45,230   总盈亏: +¥2,341   │  │
│  └───────────────────────┘  └──────────────────────────────────────┘  │
│                                                                         │
│  ┌─ Detail: AAPL ─────────────────────────────────────────────────┐   │
│  │  Apple Inc.  NASDAQ   最新: $185.20  涨跌: +2.20 (+1.20%)      │   │
│  │  开: 183.50  高: 186.10  低: 182.90  量: 52.3M                 │   │
│  │  52周高/低: 198.23 / 124.17   市值: $2.87T   PE: 28.5          │   │
│  └────────────────────────────────────────────────────────────────┘   │
│                                                                         │
│  [数据源: Yahoo Finance]  [最后更新: 14:32:05]  [刷新: 30s]            │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 键盘交互

| 按键 | 动作 |
|------|------|
| `Tab` | 在面板间切换焦点 |
| `↑` / `↓` | 在列表中导航 |
| `/` | 搜索并添加股票代码 |
| `a` | 添加持仓（输入代码、数量、成本） |
| `d` | 删除选中条目 |
| `r` | 手动刷新数据 |
| `q` / `Ctrl+C` | 退出 |

---

## 4. 数据层设计

### 4.1 DataProvider Trait

```rust
#[async_trait]
pub trait DataProvider: Send + Sync {
    async fn fetch_quote(&self, symbol: &Symbol) -> Result<Quote, DataError>;
    async fn fetch_ohlcv(
        &self,
        symbol: &Symbol,
        period: Period,
    ) -> Result<Vec<OHLCV>, DataError>;
    fn name(&self) -> &'static str;
    fn supports(&self, market: Market) -> bool;
}
```

### 4.2 支持的数据源（Phase 1）

| 数据源 | 市场 | 限制 |
|--------|------|------|
| Yahoo Finance | 美股、港股、部分 A 股 | 免费，无需 API Key |
| Alpha Vantage | 美股、外汇、加密货币 | 免费 25 次/天 |
| 新浪财经 HTTP API | A 股实时行情 | 免费非官方 API，直接 HTTP 调用 |
| 东方财富 HTTP API | A 股/港股行情 | 免费非官方 API，直接 HTTP 调用 |
| 本地 CSV/JSON | 任意 | 历史数据导入 |

**Provider 选择策略：** 根据 `Symbol.market` 自动选择最优 Provider，失败时自动 fallback 到下一个。

### 4.3 缓存策略

- 行情数据：60s TTL（盘中），4h TTL（盘后）
- OHLCV 历史：永久缓存（存 SQLite），增量更新
- 缓存存储位置：`~/.config/fa/cache.db`

---

## 5. 错误处理

```rust
// fa-core/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum DataError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Rate limited, retry after {retry_after}s")]
    RateLimited { retry_after: u64 },
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("Cache error: {0}")]
    Cache(String),
}
```

**降级策略：**
- API 请求失败 → 指数退避重试（1s, 2s, 4s），最多 3 次
- 3 次失败后 → 使用缓存数据，状态栏显示警告 `[!] 使用缓存数据`
- TUI panic → `std::panic::set_hook` 捕获，恢复终端原始模式后退出

---

## 6. 关键依赖

```toml
[workspace.dependencies]
# TUI
ratatui = "0.28"
crossterm = "0.28"

# 异步运行时
tokio = { version = "1", features = ["full"] }

# HTTP
reqwest = { version = "0.12", features = ["json"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 错误处理
thiserror = "1"
anyhow = "1"

# 时间
chrono = { version = "0.4", features = ["serde"] }

# 金融计算（避免 f64 精度问题）
rust_decimal = "1"

# 数据库（缓存）
rusqlite = "0.31"

# 测试
mockito = "1"           # Mock HTTP server
```

---

## 7. 测试策略

| 层次 | 工具 | 测试内容 |
|------|------|----------|
| `fa-core` 单元测试 | 标准 `#[test]` | 盈亏计算、百分比、数据模型 |
| `fa-data` 集成测试 | `mockito` | API 响应解析、错误处理、重试逻辑 |
| `fa-tui` 组件测试 | `ratatui::backend::TestBackend` | 渲染输出验证 |
| 端到端 | 手动 + CI | 启动、数据加载、基本交互 |

---

## 8. 配置文件格式

存储于 `~/.config/fa/config.toml`：

```toml
[general]
refresh_interval = 30      # 秒
default_currency = "CNY"

[data_sources]
primary = "yahoo"
fallback = ["alpha_vantage", "local"]

[api_keys]
alpha_vantage = "YOUR_KEY"

[[watchlist]]
symbol = "AAPL"
market = "us"

[[watchlist]]
symbol = "600519"
market = "a_share"

[[portfolio]]
symbol = "AAPL"
market = "us"
quantity = 10
cost_basis = 1800.0
```

---

## 9. 可行性评估

| 方面 | 结论 |
|------|------|
| **技术可行性** | ✅ Ratatui 生态成熟，tokio async 完全胜任 |
| **数据获取** | ✅ Yahoo Finance/Alpha Vantage 免费且稳定，A 股可用 AKShare |
| **TUI 渲染性能** | ✅ Ratatui 16ms 重绘无压力，Rust 性能充足 |
| **复杂度** | ⚠️ Phase 1 中等复杂度，Phase 2-3 显著增加（K 线图需自制绘图逻辑） |
| **A 股支持** | ⚠️ 官方免费 API 不开放，使用新浪/东财非官方接口，有被限流风险，建议加请求间隔 |
| **总体结论** | ✅ **完全可行**，分阶段实现风险可控 |
