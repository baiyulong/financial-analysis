# fa — 终端股票金融分析系统

基于 Rust + Ratatui 构建的 TUI 股票分析工具，支持 A 股与美股实时行情、K 线图（Unicode 块字符绘制）及均线指标。

```
┌ 观察列表 ───────────────┐ ┌ 行情 ─────────────────────────────────────────────┐
│ sh000001  上证指数       │ │ sh000001  3,286.72  +12.45  +0.38%                │
│ sh000300  沪深 300      │ │ sz399001  10,876.33  -23.11  -0.21%               │
│ sz399001  深证成指  ◄   │ └───────────────────────────────────────────────────┘
│ sz399006  创业板指       │ ┌ 持仓 ─────────────────────────────────────────────┐
│ sh000016  上证 50        │ │ AAPL   10 股  成本 $1800  现价 $182  盈亏 +$20  │
│ 600519    贵州茅台       │ └───────────────────────────────────────────────────┘
│ AAPL      Apple Inc.    │
└────────────────────────┘
```

## 功能

- **实时行情**：Yahoo Finance 数据源，每 30 秒自动刷新
- **K 线图**：Unicode 块字符渲染（`█` `│`），红涨绿跌（A 股惯例）
- **均线指标**：MA5（黄）/ MA10（青）/ MA20（品红），连续折线
- **自选股管理**：支持 A 股（sh/sz 前缀）和美股
- **持仓跟踪**：记录成本价、数量，计算盈亏

## 快速开始

### 依赖

- Rust 1.70+（推荐使用 [rustup](https://rustup.rs/) 安装）

### 构建与运行

```bash
git clone https://github.com/baiyulong/financial-analysis.git
cd financial-analysis

# 配置自选股（可选）
cp config/default.toml ~/.config/fa/config.toml

# 构建并运行
cargo run --release
```

### 配置

配置文件路径：`~/.config/fa/config.toml`（不存在时自动使用内置默认值）。

```toml
[general]
refresh_interval = 30      # 行情刷新间隔（秒）
default_currency = "CNY"

# A 股：使用 sh（上交所）或 sz（深交所）前缀
[[watchlist]]
symbol = "sh000001"
market = "a_share"         # 上证指数

[[watchlist]]
symbol = "sz399001"
market = "a_share"         # 深证成指

[[watchlist]]
symbol = "600519"
market = "a_share"         # 贵州茅台（自动补全 sh 前缀）

# 美股：直接使用 ticker
[[watchlist]]
symbol = "AAPL"
market = "us"

# 持仓记录
[[portfolio]]
symbol = "AAPL"
market = "us"
quantity = "10"
cost_basis = "1800.00"
```

**市场代码对照：**

| `market` 值 | 说明 |
|---|---|
| `a_share` | A 股（上交所 `sh` / 深交所 `sz`） |
| `us` | 美股 |

## 键盘操作

### 主界面

| 按键 | 功能 |
|---|---|
| `↑` / `↓` | 在自选股列表中移动 |
| `Tab` | 切换面板 |
| `Enter` | 进入 K 线图 |
| `/` | 搜索（输入后实时过滤） |
| `Backspace` | 删除搜索字符 |
| `Esc` | 取消搜索 |
| `d` | 从列表删除当前股票 |
| `r` | 手动刷新行情 |
| `q` / `Ctrl+C` | 退出 |

### K 线图界面

| 按键 | 功能 |
|---|---|
| `←` / `→` | 移动时间轴光标 |
| `[` | 缩小（每根 bar 更窄，显示更多） |
| `]` | 放大（每根 bar 更宽） |
| `1` | 切换为日线（近 1 天） |
| `5` | 切换为周线（近 5 天） |
| `m` | 切换为月线（近 1 月） |
| `q` | 切换为季线（近 3 月） |
| `y` | 切换为年线（近 1 年） |
| `Esc` | 返回主界面 |

底部状态栏实时显示光标所在 bar 的 OHLCV 数据及涨跌幅。

## 项目结构

```
financial-analysis/
├── src/main.rs              # 入口：事件循环、TUI 初始化、ChartFetcher 任务
├── config/default.toml      # 内置默认配置（复制到 ~/.config/fa/ 自定义）
└── crates/
    ├── fa-core/             # 基础数据类型：OHLCV、Symbol、Period、Market
    ├── fa-data/             # 数据源：Yahoo Finance HTTP 客户端、缓存路由
    ├── fa-indicator/        # 技术指标：SMA（简单移动平均）
    └── fa-tui/              # TUI 层：AppState、EventHandler、图表渲染
```

### 架构概览

```
DataFetcher (Tokio task)
    │  fetch OHLCV via Yahoo Finance /v8/finance/chart/
    ▼
Arc<RwLock<AppState>>   ←──  EventHandler (16ms tick, single read lock)
    │
    ▼
Ratatui renderer
    ├── Main screen：行情面板 + 持仓面板 + 自选列表
    └── Chart screen：KlineChart Widget（Unicode 块字符）
```

## 开发

```bash
# 运行所有测试
cargo test --workspace

# 仅运行图表模块测试
cargo test -p fa-tui

# 检查编译
cargo check --workspace
```

## 数据来源

行情数据来自 [Yahoo Finance](https://finance.yahoo.com/) 非官方 API。A 股 ticker 映射规则：

- `sh000001` → `000001.SS`（上交所）
- `sz399001` → `399001.SZ`（深交所）
- 个股 `600519` → `600519.SS`

> **注意**：Yahoo Finance 对高频请求有速率限制，默认刷新间隔 30 秒可正常使用。
