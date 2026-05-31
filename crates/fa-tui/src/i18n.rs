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
    pub data_source_label: &'static str,
    pub data_source_sina: &'static str,
    pub updated_label: &'static str,
    pub refresh_label: &'static str,
    pub never: &'static str,
    pub search_prompt: &'static str,
    // ── Watchlist ───────────────────────────────────────────────────
    pub watchlist_title: &'static str,
    pub add_title_prefix: &'static str,
    // ── Portfolio ───────────────────────────────────────────────────
    pub portfolio_title: &'static str,
    // ── Detail ──────────────────────────────────────────────────────
    pub detail_title: &'static str,
    pub detail_loading: &'static str,
    pub detail_select_hint: &'static str,
    pub detail_latest: &'static str,
    pub detail_open: &'static str,
    pub detail_high: &'static str,
    pub detail_low: &'static str,
    pub detail_volume: &'static str,
    pub detail_vol_unit_yi: &'static str,
    pub detail_vol_unit_wan: &'static str,
    // ── Chart ───────────────────────────────────────────────────────
    pub chart_loading: &'static str,
    pub chart_no_data: &'static str,
    pub chart_help: &'static str,
    pub chart_max_history: &'static str,
    // ── Backtest ────────────────────────────────────────────────────
    pub bt_config_title: &'static str,
    pub bt_result_title: &'static str,
    pub bt_trades_title: &'static str,
    pub bt_nav_help: &'static str,
    pub bt_status_idle: &'static str,
    pub bt_status_running: &'static str,
    pub bt_status_done: &'static str,
    pub bt_press_r: &'static str,
    pub bt_running_msg: &'static str,
    pub bt_no_trades: &'static str,
    pub bt_label_stock: &'static str,
    pub bt_label_strategy: &'static str,
    pub bt_label_cash: &'static str,
    pub bt_label_commission: &'static str,
    pub bt_label_slippage: &'static str,
    pub bt_label_period: &'static str,
    pub bt_label_total_return: &'static str,
    pub bt_label_annualized: &'static str,
    pub bt_label_max_drawdown: &'static str,
    pub bt_label_win_rate: &'static str,
    pub bt_label_sharpe: &'static str,
    pub bt_label_trades: &'static str,
    pub bt_label_initial: &'static str,
    pub bt_label_final: &'static str,
    pub bt_trade_date: &'static str,
    pub bt_trade_action: &'static str,
    pub bt_trade_price: &'static str,
    pub bt_trade_qty: &'static str,
    pub bt_trade_amount: &'static str,
    pub bt_trade_pnl: &'static str,
    pub bt_trade_buy: &'static str,
    pub bt_trade_sell: &'static str,
    // ── Settings ────────────────────────────────────────────────────
    pub settings_title: &'static str,
    pub settings_data_source: &'static str,
    pub settings_akshare_url: &'static str,
    pub settings_akshare_only: &'static str,
    pub settings_language: &'static str,
    pub settings_help: &'static str,
    pub settings_lang_zh: &'static str,
    pub settings_lang_en: &'static str,
    // ── Quit confirm ────────────────────────────────────────────────
    pub quit_title: &'static str,
    pub quit_yes: &'static str,
    pub quit_no: &'static str,
    pub quit_help: &'static str,
}

pub static ZH: Strings = Strings {
    data_source_label: "数据源",
    data_source_sina: "新浪",
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
    chart_help: "数据源: {} | F1-F5:分钟 | 1:日 5:周 m:月 q:季 y:年 | ←→:移动 | []:缩放 | Esc:返回",
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
    quit_title: "确认退出？",
    quit_yes: "[ 确认 ]",
    quit_no: "[ 取消 ]",
    quit_help: "y/Enter 确认   n/Esc 取消",
};

pub static EN: Strings = Strings {
    data_source_label: "Source",
    data_source_sina: "Sina",
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
    detail_vol_unit_yi: "100M lots",
    detail_vol_unit_wan: "10k lots",
    chart_loading: "Loading data...",
    chart_no_data: "No data available",
    chart_help: "Source: {} | F1-F5:min | 1:day 5:wk m:mo q:qtr y:yr | ←→:scroll | []:zoom | Esc:back",
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
    quit_title: "Quit?",
    quit_yes: "[ Yes ]",
    quit_no: "[ No  ]",
    quit_help: "y/Enter confirm   n/Esc cancel",
};

#[cfg(test)]
mod tests {
    use super::*;
    fn check_all_fields(s: &Strings) {
        assert!(!s.data_source_label.is_empty());
        assert!(!s.data_source_sina.is_empty());
        assert!(!s.updated_label.is_empty());
        assert!(!s.refresh_label.is_empty());
        assert!(!s.never.is_empty());
        assert!(!s.search_prompt.is_empty());
        assert!(!s.watchlist_title.is_empty());
        assert!(!s.add_title_prefix.is_empty());
        assert!(!s.portfolio_title.is_empty());
        assert!(!s.detail_title.is_empty());
        assert!(!s.detail_loading.is_empty());
        assert!(!s.detail_select_hint.is_empty());
        assert!(!s.detail_latest.is_empty());
        assert!(!s.detail_open.is_empty());
        assert!(!s.detail_high.is_empty());
        assert!(!s.detail_low.is_empty());
        assert!(!s.detail_volume.is_empty());
        assert!(!s.detail_vol_unit_yi.is_empty());
        assert!(!s.detail_vol_unit_wan.is_empty());
        assert!(!s.chart_loading.is_empty());
        assert!(!s.chart_no_data.is_empty());
        assert!(!s.chart_help.is_empty());
        assert!(!s.chart_max_history.is_empty());
        assert!(!s.bt_config_title.is_empty());
        assert!(!s.bt_result_title.is_empty());
        assert!(!s.bt_trades_title.is_empty());
        assert!(!s.bt_nav_help.is_empty());
        assert!(!s.bt_status_idle.is_empty());
        assert!(!s.bt_status_running.is_empty());
        assert!(!s.bt_status_done.is_empty());
        assert!(!s.bt_press_r.is_empty());
        assert!(!s.bt_running_msg.is_empty());
        assert!(!s.bt_no_trades.is_empty());
        assert!(!s.bt_label_stock.is_empty());
        assert!(!s.bt_label_strategy.is_empty());
        assert!(!s.bt_label_cash.is_empty());
        assert!(!s.bt_label_commission.is_empty());
        assert!(!s.bt_label_slippage.is_empty());
        assert!(!s.bt_label_period.is_empty());
        assert!(!s.bt_label_total_return.is_empty());
        assert!(!s.bt_label_annualized.is_empty());
        assert!(!s.bt_label_max_drawdown.is_empty());
        assert!(!s.bt_label_win_rate.is_empty());
        assert!(!s.bt_label_sharpe.is_empty());
        assert!(!s.bt_label_trades.is_empty());
        assert!(!s.bt_label_initial.is_empty());
        assert!(!s.bt_label_final.is_empty());
        assert!(!s.bt_trade_date.is_empty());
        assert!(!s.bt_trade_action.is_empty());
        assert!(!s.bt_trade_price.is_empty());
        assert!(!s.bt_trade_qty.is_empty());
        assert!(!s.bt_trade_amount.is_empty());
        assert!(!s.bt_trade_pnl.is_empty());
        assert!(!s.bt_trade_buy.is_empty());
        assert!(!s.bt_trade_sell.is_empty());
        assert!(!s.settings_title.is_empty());
        assert!(!s.settings_data_source.is_empty());
        assert!(!s.settings_akshare_url.is_empty());
        assert!(!s.settings_akshare_only.is_empty());
        assert!(!s.settings_language.is_empty());
        assert!(!s.settings_help.is_empty());
        assert!(!s.settings_lang_zh.is_empty());
        assert!(!s.settings_lang_en.is_empty());
        assert!(!s.quit_title.is_empty());
        assert!(!s.quit_yes.is_empty());
        assert!(!s.quit_no.is_empty());
        assert!(!s.quit_help.is_empty());
    }

    #[test]
    fn test_zh_all_fields_non_empty() {
        check_all_fields(&ZH);
    }
    #[test]
    fn test_en_all_fields_non_empty() {
        check_all_fields(&EN);
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
