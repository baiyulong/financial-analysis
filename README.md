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

- **实时行情**：默认使用新浪财经，支持 A 股与美股行情，每 30 秒自动刷新
- **K 线图**：Unicode 块字符渲染（`█` `│`），红涨绿跌（A 股惯例）；默认支持日 / 周 / 月线，AkShare 模式下可查看分钟线
- **均线指标**：MA5（黄）/ MA10（青）/ MA20（品红），连续折线
- **自选股管理**：支持 A 股（sh/sz 前缀）和美股，运行时按 `a` 添加新股票，支持模糊搜索
- **回测引擎**：按 `b` 对选中股票运行历史策略回测（双均线、RSI、布林带），查看收益、回撤、Sharpe 比率
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

[data_source]
provider = "sina"         # 可选："sina" / "akshare"
akshare_url = "http://127.0.0.1:8080"

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

## 数据源设置

应用目前支持两种 A 股 K 线数据源：**新浪财经（Sina Finance）**（默认）与 **AkShare（通过 AKTools HTTP 服务）**。

- **新浪财经**：内置可用，无需额外部署。适合直接查看 A 股实时行情，以及日 / 周 / 月级别 K 线。
- **AkShare（AKTools）**：通过本地 Python HTTP 服务提供 A 股分钟级 K 线（`1m` / `5m` / `15m` / `30m` / `60m`）。

```bash
pip install aktools
python -m aktools
```

默认服务地址：`http://127.0.0.1:8080`

运行时可在主界面按 `s` 打开设置：使用 `Space` 切换数据源；若焦点在 URL 字段上，再按 `Space` 开始编辑地址；按 `Enter` 保存。

数据源设置会持久化到 `~/.config/fa/config.toml`：

```toml
[data_source]
provider = "sina"         # 或 "akshare"
akshare_url = "http://127.0.0.1:8080"
```

## 快捷键

### 主界面

| 按键 | 功能 |
|---|---|
| `↑` / `↓` | 在当前列表中移动 |
| `Tab` | 在自选股 / 持仓面板之间切换 |
| `a` | 添加股票（支持模糊搜索） |
| `d` | 删除当前选中的股票 |
| `Enter` | 查看当前股票的 K 线图 |
| `s` | 打开设置 |
| `b` | 进入回测模式 |
| `/` | 搜索（输入后实时过滤） |
| `r` | 手动刷新行情 |
| `Esc` | 取消搜索 / 取消添加 |
| `q` / `Ctrl+C` | 退出 |

> **添加股票说明：** 按 `a` 后可输入股票代码或名称关键字，候选列表会实时更新；按 `Enter` 确认，按 `Esc` 取消。
>
> - A 股：`sh000001`（上交所）、`sz399001`（深交所）、或直接输入 6 位数字如 `600519`
> - 美股：直接输入 ticker，如 `TSLA`、`NVDA`

### K 线图界面

| 按键 | 功能 |
|---|---|
| `1` | 切换为日线 |
| `5` | 切换为周线 |
| `m` | 切换为月线 |
| `q` | 切换为季线 |
| `y` | 切换为年线 |
| `F1` | 1 分钟线（仅 AkShare） |
| `F2` | 5 分钟线（仅 AkShare） |
| `F3` | 15 分钟线（仅 AkShare） |
| `F4` | 30 分钟线（仅 AkShare） |
| `F5` | 60 分钟线（仅 AkShare） |
| `←` / `→` | 移动光标；在最左侧继续按 `←` 会自动请求更长周期历史数据 |
| `[` / `]` | 缩小 / 放大每根 bar 的宽度 |
| `Esc` | 返回主界面 |

底部状态栏会显示当前数据源，以及光标所在 bar 的 OHLCV 数据与涨跌幅。

### 设置界面

| 按键 | 功能 |
|---|---|
| `↑` / `↓` | 在字段间移动 |
| `Space` | 切换数据源 / 开始或结束 URL 编辑 |
| `Enter` | 保存并关闭 |
| `Esc` | 取消并返回主界面 |

### 回测界面

| 按键 | 功能 |
|---|---|
| `←` / `→` | 切换回测策略 |
| `↑` / `↓` | 滚动交易记录 |
| `r` | 运行回测（获取 1 年日线数据后执行） |
| `Esc` | 返回主界面 |

内置三种策略：
- **双均线穿越 (MA5×MA20)**：金叉买入、死叉卖出
- **RSI 均值回归 (14/30/70)**：RSI < 30 买入，RSI > 70 卖出
- **布林带 (20, 2σ)**：价格跌破下轨买入，突破上轨卖出

回测结果显示：总收益率、年化收益、最大回撤、胜率、Sharpe 比率、交易记录。

## 项目结构

```
financial-analysis/
├── src/main.rs              # 入口：事件循环、TUI 初始化、ChartFetcher 任务
├── config/default.toml      # 内置默认配置（复制到 ~/.config/fa/ 自定义）
└── crates/
    ├── fa-core/             # 基础数据类型：OHLCV、Symbol、Period、Market
    ├── fa-data/             # 数据源：新浪 / AkShare / Yahoo Finance 路由
    ├── fa-indicator/        # 技术指标：SMA（简单移动平均）
    └── fa-tui/              # TUI 层：AppState、EventHandler、图表渲染
```

### 架构概览

```
DataFetcher (Tokio task)
    │  fetch quote / OHLCV via ProviderRouter（新浪 / AkShare / Yahoo）
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

应用会根据数据类型自动选择可用的数据提供方：

- **新浪财经（默认）**：A 股实时行情、股票搜索，以及默认模式下的常用 A 股查看体验
- **AkShare / AKTools**：A 股 `1m` / `5m` / `15m` / `30m` / `60m` 分钟 K 线（需本地 HTTP 服务）
- **Yahoo Finance**：部分历史行情与美股数据的后备来源

> **注意**：第三方非官方数据源可能存在速率限制或临时不可用的情况；若使用 AkShare，请先确认本地 `python -m aktools` 服务已启动。
