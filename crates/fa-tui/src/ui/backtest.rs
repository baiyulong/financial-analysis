use crate::app::{BacktestState, BacktestStatus};
use fa_backtest::{BuiltinStrategy, TradeAction};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use rust_decimal_macros::dec;

pub fn render(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    render_config(f, bs, cols[0]);

    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(cols[1]);

    render_metrics(f, bs, right_rows[0]);
    render_trades(f, bs, right_rows[1]);
}

fn render_config(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let strategy_name = bs.strategy_name();

    let status_str = match &bs.status {
        BacktestStatus::Idle    => " [r 运行]".to_string(),
        BacktestStatus::Running => " ⏳ 运行中...".to_string(),
        BacktestStatus::Done    => " ✅ 完成".to_string(),
        BacktestStatus::Error(e) => format!(" ❌ {e}"),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("股票: ", Style::default().fg(Color::Gray)),
            Span::styled(bs.symbol.code.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("策略: ", Style::default().fg(Color::Gray)),
        ]),
        Line::from(vec![
            Span::styled(format!("← {strategy_name} →"), Style::default().fg(Color::Yellow)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("资金: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("¥{:.0}", bs.config.initial_cash)),
        ]),
        Line::from(vec![
            Span::styled("手续费: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}bps", bs.config.commission_bps)),
        ]),
        Line::from(vec![
            Span::styled("滑点:   ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{}bps", bs.config.slippage_bps)),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("时间: ", Style::default().fg(Color::Gray)),
            Span::raw(format!("{} ~ {}", bs.config.start_date, bs.config.end_date)),
        ]),
        Line::raw(""),
        Line::from(Span::styled(status_str, Style::default().fg(Color::Green))),
        Line::raw(""),
        Line::from(Span::styled("← → 切换策略  r 运行  Esc 返回", Style::default().fg(Color::DarkGray))),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().title(" 回测配置 ").borders(Borders::ALL));
    f.render_widget(para, area);
}

fn render_metrics(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let content = match &bs.result {
        None => {
            let hint = match &bs.status {
                BacktestStatus::Running => "正在运行回测...",
                _ => "按 r 开始回测",
            };
            vec![Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray)))]
        }
        Some(r) => {
            let ret_color = if r.total_return >= dec!(0) { Color::Red } else { Color::Green };
            vec![
                Line::from(vec![
                    Span::styled("总收益:  ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:+.2}%", r.total_return * dec!(100)), Style::default().fg(ret_color).add_modifier(Modifier::BOLD)),
                    Span::raw("   "),
                    Span::styled("年化:  ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:+.2}%", r.annualized_return * dec!(100)), Style::default().fg(ret_color)),
                ]),
                Line::from(vec![
                    Span::styled("最大回撤: ", Style::default().fg(Color::Gray)),
                    Span::styled(format!("{:.2}%", r.max_drawdown * dec!(100)), Style::default().fg(Color::Green)),
                    Span::raw("   "),
                    Span::styled("胜率:  ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{:.1}%", r.win_rate * dec!(100))),
                ]),
                Line::from(vec![
                    Span::styled("Sharpe:   ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{:.3}", r.sharpe_ratio)),
                    Span::raw("   "),
                    Span::styled("交易次数: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("{}", r.total_trades)),
                ]),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("初始: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("¥{:.0}", r.initial_cash)),
                    Span::raw("   "),
                    Span::styled("最终: ", Style::default().fg(Color::Gray)),
                    Span::raw(format!("¥{:.0}", r.final_equity)),
                ]),
            ]
        }
    };

    let para = Paragraph::new(content)
        .block(Block::default().title(" 回测结果 ").borders(Borders::ALL));
    f.render_widget(para, area);
}

fn render_trades(f: &mut Frame, bs: &BacktestState, area: Rect) {
    let items: Vec<ListItem> = match &bs.result {
        None => vec![ListItem::new(" 暂无交易记录")],
        Some(r) => {
            let header = ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{:<12} {:<5} {:<10} {:<7} {:<12} {}", "日期", "操作", "价格", "数量", "金额", "盈亏"),
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
                ),
            ]));
            let mut items = vec![header];
            let trades = &r.trades;
            let skip = bs.trade_scroll.min(trades.len().saturating_sub(1));
            for t in trades.iter().skip(skip) {
                let (action_str, action_color) = match t.action {
                    TradeAction::Buy  => ("买入", Color::Red),
                    TradeAction::Sell => ("卖出", Color::Green),
                };
                let pnl_str = t.pnl
                    .map(|p| format!("{:+.0}", p))
                    .unwrap_or_default();
                let pnl_color = t.pnl
                    .map(|p| if p >= dec!(0) { Color::Red } else { Color::Green })
                    .unwrap_or(Color::White);
                items.push(ListItem::new(Line::from(vec![
                    Span::raw(format!("{:<12} ", t.date)),
                    Span::styled(format!("{:<5} ", action_str), Style::default().fg(action_color)),
                    Span::raw(format!("{:<10.2} {:<7} {:<12.0} ", t.price, t.quantity, t.amount)),
                    Span::styled(pnl_str, Style::default().fg(pnl_color)),
                ])));
            }
            items
        }
    };

    let list = List::new(items)
        .block(Block::default()
            .title(" 交易记录  ↑/↓ 滚动 ")
            .borders(Borders::ALL));
    f.render_widget(list, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppScreen, State};
    use fa_core::{Market, Symbol};
    use ratatui::{backend::TestBackend, Terminal};

    fn make_state_on_backtest() -> State {
        let mut s = State::default();
        let sym = Symbol::new("AAPL", Market::USStock);
        s.screen = AppScreen::Backtest(BacktestState::new(sym));
        s
    }

    #[test]
    fn test_render_backtest_idle_does_not_panic() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = make_state_on_backtest();
        terminal.draw(|f| {
            if let AppScreen::Backtest(ref bs) = state.screen {
                render(f, bs, f.area());
            }
        }).unwrap();
    }

    #[test]
    fn test_render_backtest_shows_strategy_name() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = make_state_on_backtest();
        terminal.draw(|f| {
            if let AppScreen::Backtest(ref bs) = state.screen {
                render(f, bs, f.area());
            }
        }).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("MA") || content.contains("均线") || content.contains("回测"),
                "expected strategy name or '回测' in output");
    }
}
