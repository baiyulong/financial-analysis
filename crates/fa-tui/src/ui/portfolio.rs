// crates/fa-tui/src/ui/portfolio.rs
use crate::app::{FocusedPanel, State};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use rust_decimal::Decimal;

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let focused = state.focused_panel == FocusedPanel::Portfolio;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let prices: std::collections::HashMap<String, Decimal> = state
        .quotes
        .iter()
        .map(|(k, q)| (k.clone(), q.price))
        .collect();

    let mut items: Vec<ListItem> = state
        .portfolio
        .positions
        .iter()
        .map(|pos| {
            let price = prices
                .get(&pos.symbol.code)
                .copied()
                .unwrap_or(Decimal::ZERO);
            let mv = pos.market_value(price);
            let pnl = pos.pnl(price);
            let pct = pos.pnl_pct(price);
            let color = if pnl >= Decimal::ZERO {
                Color::Green
            } else {
                Color::Red
            };
            let sign = if pnl >= Decimal::ZERO { "+" } else { "" };

            let line = Line::from(vec![
                Span::raw(format!("{:<8} x{}", pos.symbol.code, pos.quantity)),
                Span::raw(format!("  {:>10.2}", pos.cost_basis)),
                Span::raw(format!("  {:>10.2}", mv)),
                Span::styled(
                    format!("  {}{:.2}({}{:.1}%)", sign, pnl, sign, pct),
                    Style::default().fg(color),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    // Summary row
    let total_cost = state.portfolio.total_cost();
    let total_mv = state.portfolio.total_market_value(&prices);
    let total_pnl = total_mv - total_cost;
    let pnl_color = if total_pnl >= Decimal::ZERO {
        Color::Green
    } else {
        Color::Red
    };
    let sign = if total_pnl >= Decimal::ZERO { "+" } else { "" };

    items.push(ListItem::new(Line::from(vec![Span::styled(
        format!(
            "─── Total: {:.2}  MV:{:.2}  PnL:{}{:.2}",
            total_cost, total_mv, sign, total_pnl
        ),
        Style::default().fg(pnl_color).add_modifier(Modifier::BOLD),
    )])));

    let list = List::new(items)
        .block(
            Block::default()
                .title(state.strings().portfolio_title)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = ListState::default();
    if !state.portfolio.positions.is_empty() {
        list_state.select(Some(state.selected_portfolio));
    }

    f.render_stateful_widget(list, area, &mut list_state);
}
