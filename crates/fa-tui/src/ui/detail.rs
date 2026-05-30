// crates/fa-tui/src/ui/detail.rs
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use rust_decimal::prelude::ToPrimitive;
use crate::app::State;

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let content = if let Some(sym) = state.selected_symbol() {
        if let Some(q) = state.quotes.get(&sym.code) {
            vec![
                Line::from(format!(
                    "{}  {}  Latest: {:.2}  {}",
                    sym.code, sym.market, q.price, q.change_display()
                )),
                Line::from(format!(
                    "Open: {}  High: {}  Low: {}  Vol: {}",
                    q.open.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.high.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.low.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.volume.map(|v| {
                        if v >= 1_000_000 { format!("{:.1}M", v as f64 / 1e6) }
                        else { format!("{}", v) }
                    }).unwrap_or("--".into()),
                )),
                Line::from(format!(
                    "52W High/Low: {} / {}  Mkt Cap: {}  PE: {}",
                    q.week_52_high.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.week_52_low.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.market_cap.and_then(|v| v.to_f64()).map(|v| {
                        if v >= 1e12 { format!("{:.2}T", v / 1e12) }
                        else { format!("{:.2}B", v / 1e9) }
                    }).unwrap_or("--".into()),
                    q.pe_ratio.map(|v| format!("{:.1}", v)).unwrap_or("--".into()),
                )),
            ]
        } else {
            vec![Line::from(format!("{} — Loading...", sym.code))]
        }
    } else {
        vec![Line::from("Select a symbol to view details")]
    };

    let para = Paragraph::new(content)
        .block(Block::default().title(" Detail ").borders(Borders::ALL));
    f.render_widget(para, area);
}
