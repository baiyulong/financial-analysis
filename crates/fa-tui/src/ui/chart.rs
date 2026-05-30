use fa_core::Period;
use fa_indicator::sma;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};
use rust_decimal::prelude::ToPrimitive;
use crate::app::ChartState;

struct KlineChart<'a> {
    visible: &'a [fa_core::OHLCV],
    y_min: f64,
    y_max: f64,
    bar_w: u16,
    cursor_in_view: usize,
    /// Pre-sliced to visible range: ma_data[k].1[i] corresponds to visible[i]
    ma_data: Vec<(Color, Vec<Option<rust_decimal::Decimal>>)>,
}

impl Widget for KlineChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let h = inner.height;
        let w = inner.width;

        // ── Pass 1: Candlestick bars ──────────────────────────────────────
        for (i, bar) in self.visible.iter().enumerate() {
            let x_off = (i as u16) * self.bar_w;
            if x_off >= w {
                break;
            }

            let open     = bar.open.to_f64().unwrap_or(self.y_min);
            let close    = bar.close.to_f64().unwrap_or(self.y_min);
            let body_top = open.max(close);
            let body_bot = open.min(close);
            let high     = bar.high.to_f64().unwrap_or(body_top);
            let low      = bar.low.to_f64().unwrap_or(body_bot);
            let color    = if close >= open { Color::Red } else { Color::Green };

            let high_row     = price_to_row(high,     self.y_min, self.y_max, h);
            let low_row      = price_to_row(low,      self.y_min, self.y_max, h);
            let top_body_row = price_to_row(body_top, self.y_min, self.y_max, h);
            let bot_body_row = price_to_row(body_bot, self.y_min, self.y_max, h);

            let center_col = inner.x + x_off + self.bar_w / 2;
            // Body occupies columns [x_off+1 .. x_off+bar_w-2], 1-cell margin each side
            let body_left  = inner.x + x_off + 1;
            let body_right = (inner.x + x_off + self.bar_w).saturating_sub(2);
            let max_col    = inner.x + w - 1;

            // Upper wick (high → top of body)
            for r in high_row..top_body_row {
                if center_col <= max_col {
                    buf[(center_col, inner.y + r)].set_char('│').set_fg(color);
                }
            }

            // Body (top_body_row ..= bot_body_row)
            for r in top_body_row..=bot_body_row {
                let y = inner.y + r;
                for col in body_left..=body_right.min(max_col) {
                    buf[(col, y)].set_char('█').set_fg(color);
                }
                // When bar_w <= 2 the body margins may leave center_col uncovered
                if (center_col < body_left || center_col > body_right) && center_col <= max_col {
                    buf[(center_col, y)].set_char('█').set_fg(color);
                }
            }

            // Lower wick (below body → low)
            if bot_body_row < low_row {
                for r in (bot_body_row + 1)..=low_row {
                    if center_col <= max_col {
                        buf[(center_col, inner.y + r)].set_char('│').set_fg(color);
                    }
                }
            }
        }

        // ── Pass 2: MA overlay (─ dot at price row for each bar) ─────────
        for (color, vis_ma) in &self.ma_data {
            for (i, v) in vis_ma.iter().enumerate() {
                if let Some(price) = v.and_then(|d| d.to_f64()) {
                    let x_off = (i as u16) * self.bar_w;
                    if x_off >= w {
                        break;
                    }
                    let center_col = inner.x + x_off + self.bar_w / 2;
                    let row = price_to_row(price, self.y_min, self.y_max, h);
                    if center_col < inner.x + w {
                        buf[(center_col, inner.y + row)].set_char('─').set_fg(*color);
                    }
                }
            }
        }

        // ── Pass 3: Cursor column highlight (DarkGray bg on center col) ──
        if self.cursor_in_view < self.visible.len() {
            let x_off = (self.cursor_in_view as u16) * self.bar_w;
            if x_off < w {
                let center_col = inner.x + x_off + self.bar_w / 2;
                if center_col < inner.x + w {
                    for r in 0..h {
                        buf[(center_col, inner.y + r)].set_bg(Color::DarkGray);
                    }
                }
            }
        }
    }
}

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

/// Maps a price value to a buffer row index within [0, height).
/// y_max → row 0 (top of chart), y_min → row height-1 (bottom).
fn price_to_row(price: f64, y_min: f64, y_max: f64, height: u16) -> u16 {
    if y_max <= y_min || height == 0 {
        return 0;
    }
    let ratio = (y_max - price) / (y_max - y_min);
    let row = (ratio * height as f64) as u16;
    row.min(height.saturating_sub(1))
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

    // Inner width (minus 2 border chars) determines how many bars fit
    let inner_w = area.width.saturating_sub(2) as usize;
    let bar_w = cs.bar_width as usize;
    let max_visible = (inner_w / bar_w).max(1);

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

    let ma_configs: &[(usize, Color)] = &[(5, Color::Yellow), (10, Color::Cyan), (20, Color::Magenta)];
    let cursor_in_view = cs.cursor.saturating_sub(start);

    // Pre-slice MA to visible window to keep Widget stateless
    let ma_data: Vec<(Color, Vec<Option<rust_decimal::Decimal>>)> = ma_configs.iter()
        .filter(|&&(period, _)| cs.ma_periods.contains(&period))
        .map(|&(period, color)| {
            let all_ma = sma(&cs.data, period);
            // sma() always returns data.len() elements, so start..end is always valid
            let vis_ma = all_ma[start..end].to_vec();
            (color, vis_ma)
        })
        .collect();

    f.render_widget(
        KlineChart { visible, y_min, y_max, bar_w: cs.bar_width, cursor_in_view, ma_data },
        area,
    );
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
    fn test_price_to_row() {
        assert_eq!(price_to_row(150.0, 100.0, 200.0, 10), 5);
        assert_eq!(price_to_row(200.0, 100.0, 200.0, 10), 0);
        assert_eq!(price_to_row(100.0, 100.0, 200.0, 10), 9);
        assert_eq!(price_to_row(250.0, 100.0, 200.0, 10), 0);
        assert_eq!(price_to_row(100.0, 100.0, 100.0, 10), 0);
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
