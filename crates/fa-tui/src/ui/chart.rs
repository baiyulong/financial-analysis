use fa_core::Period;
use fa_indicator::sma;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Rectangle},
        Block, Borders, Paragraph,
    },
    Frame,
};
use rust_decimal::prelude::ToPrimitive;
use crate::app::ChartState;

pub fn render(f: &mut Frame, cs: &ChartState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // 标题栏
            Constraint::Min(0),     // 图表区
            Constraint::Length(1),  // 光标信息
        ])
        .split(area);

    render_titlebar(f, cs, chunks[0]);
    render_chart(f, cs, chunks[1]);
    render_cursor_info(f, cs, chunks[2]);
}

fn period_label(p: &Period) -> &'static str {
    match p {
        Period::Day1   => "1D",
        Period::Week1  => "5D",
        Period::Month1 => "1M",
        Period::Month3 => "3M",
        Period::Month6 => "6M",
        Period::Year1  => "1Y",
        Period::Year5  => "5Y",
    }
}

fn render_titlebar(f: &mut Frame, cs: &ChartState, area: Rect) {
    let text = format!(
        " {}  {}  | [1]1D [5]5D [m]1M [q]3M [y]1Y  [←→]光标  [[]缩放  [Esc]返回",
        cs.symbol.display_code(),
        period_label(&cs.period),
    );
    f.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray)),
        area,
    );
}

fn render_chart(f: &mut Frame, cs: &ChartState, area: Rect) {
    if cs.loading || cs.data.is_empty() {
        let msg = if cs.loading { "Loading data..." } else { "No data available" };
        f.render_widget(
            Paragraph::new(msg)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let chart_inner_width = area.width.saturating_sub(8) as usize;
    let bar_w = cs.bar_width as usize;
    let max_visible = (chart_inner_width / bar_w).max(1);

    let half = max_visible / 2;
    let end = (cs.cursor + half + 1).min(cs.data.len());
    let start = end.saturating_sub(max_visible);
    let end = (start + max_visible).min(cs.data.len());
    let visible = &cs.data[start..end];

    let price_min = visible.iter()
        .filter_map(|b| b.low.to_f64())
        .fold(f64::MAX, f64::min);
    let price_max = visible.iter()
        .filter_map(|b| b.high.to_f64())
        .fold(f64::MIN, f64::max);

    let padding = ((price_max - price_min) * 0.05).max(0.01);
    let y_min = price_min - padding;
    let y_max = price_max + padding;
    let x_max = (visible.len() * bar_w) as f64;

    let ma_configs: &[(usize, Color)] = &[(5, Color::Yellow), (10, Color::Cyan), (20, Color::Magenta)];
    let cursor_in_view = cs.cursor.saturating_sub(start);

    // Pre-compute MA values outside the paint closure to avoid per-frame allocation
    let ma_data: Vec<(Color, Vec<Option<rust_decimal::Decimal>>)> = ma_configs.iter()
        .filter(|&&(period, _)| cs.ma_periods.contains(&period))
        .map(|&(period, color)| (color, sma(&cs.data, period)))
        .collect();

    let canvas = Canvas::default()
        .block(Block::default().borders(Borders::ALL))
        .x_bounds([0.0, x_max])
        .y_bounds([y_min, y_max])
        .paint(|ctx| {
            // Pass 1: K-line bodies and wicks
            for (i, bar) in visible.iter().enumerate() {
                let x_center = i as f64 * bar_w as f64 + bar_w as f64 / 2.0;
                let open  = bar.open.to_f64().unwrap_or(y_min);
                let close = bar.close.to_f64().unwrap_or(y_min);
                let body_top = open.max(close);
                let body_bot = open.min(close);
                let high  = bar.high.to_f64().unwrap_or(body_top);
                let low   = bar.low.to_f64().unwrap_or(body_bot);
                let color = if close >= open { Color::Red } else { Color::Green };

                ctx.draw(&CanvasLine { x1: x_center, y1: body_top, x2: x_center, y2: high, color });
                ctx.draw(&CanvasLine { x1: x_center, y1: low, x2: x_center, y2: body_bot, color });
                ctx.draw(&Rectangle {
                    x: i as f64 * bar_w as f64,
                    y: body_bot,
                    width: (bar_w as f64 - 0.5).max(0.5),
                    height: (body_top - body_bot).max(0.05 * (y_max - y_min)),
                    color,
                });
            }

            // Pass 2: MA overlay lines
            for (color, all_ma) in &ma_data {
                if all_ma.len() < end { continue; } // skip this MA only, not the closure
                let vis_ma = &all_ma[start..end];
                let mut prev: Option<(f64, f64)> = None;
                for (i, v) in vis_ma.iter().enumerate() {
                    if let Some(y) = v.and_then(|d| d.to_f64()) {
                        let x = i as f64 * bar_w as f64 + bar_w as f64 / 2.0;
                        if let Some((px, py)) = prev {
                            ctx.draw(&CanvasLine { x1: px, y1: py, x2: x, y2: y, color: *color });
                        }
                        prev = Some((x, y));
                    } else {
                        prev = None;
                    }
                }
            }

            // Pass 3: Cursor crosshair — drawn last so it appears on top
            if cursor_in_view < visible.len() {
                let x_center = cursor_in_view as f64 * bar_w as f64 + bar_w as f64 / 2.0;
                ctx.draw(&CanvasLine {
                    x1: x_center, y1: y_min,
                    x2: x_center, y2: y_max,
                    color: Color::White,
                });
            }
        });

    f.render_widget(canvas, area);
}

fn render_cursor_info(f: &mut Frame, cs: &ChartState, area: Rect) {
    let text = if let Some(bar) = cs.current_bar() {
        let change_pct = if !bar.open.is_zero() {
            (bar.close - bar.open) / bar.open * rust_decimal::Decimal::from(100)
        } else {
            rust_decimal::Decimal::ZERO
        };
        let sign = if bar.close >= bar.open { "+" } else { "" };
        format!(
            " {}  开:{:.2}  高:{:.2}  低:{:.2}  收:{:.2}  {}{:.2}%",
            bar.timestamp.format("%Y-%m-%d"),
            bar.open, bar.high, bar.low, bar.close,
            sign, change_pct,
        )
    } else {
        " 无数据".to_string()
    };

    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Gray)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ChartState;
    use fa_core::{Market, Period, Symbol, OHLCV};
    use ratatui::{backend::TestBackend, Terminal};
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn make_chart_state_loading() -> ChartState {
        ChartState::new(Symbol::new("AAPL", Market::USStock), Period::Month1)
    }

    fn make_chart_state_with_data() -> ChartState {
        let sym = Symbol::new("AAPL", Market::USStock);
        let bar = |o: i64, h: i64, l: i64, c: i64| OHLCV {
            symbol: sym.clone(),
            timestamp: Utc::now(),
            open:  Decimal::from(o),
            high:  Decimal::from(h),
            low:   Decimal::from(l),
            close: Decimal::from(c),
            volume: 1000,
        };
        ChartState {
            symbol: sym.clone(),
            period: Period::Month1,
            data: vec![
                bar(100, 110, 90, 105),
                bar(105, 115, 95,  98),
                bar( 98, 108, 88, 102),
            ],
            cursor: 2,
            bar_width: 3,
            ma_periods: vec![],
            loading: false,
        }
    }

    #[test]
    fn test_render_loading_state() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let cs = make_chart_state_loading();
        terminal.draw(|f| render(f, &cs, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL") || content.contains("Loading"));
    }

    #[test]
    fn test_render_with_data_no_panic() {
        let backend = TestBackend::new(120, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        let cs = make_chart_state_with_data();
        terminal.draw(|f| render(f, &cs, f.area())).unwrap();
        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL"));
    }
}
