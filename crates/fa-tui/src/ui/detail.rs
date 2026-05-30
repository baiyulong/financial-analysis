// crates/fa-tui/src/ui/detail.rs
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::app::State;

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let content = if let Some(sym) = state.selected_symbol() {
        if let Some(q) = state.quotes.get(&sym.code) {
            vec![
                Line::from(format!(
                    "  {}  {}",
                    q.name.as_deref().unwrap_or(&sym.code),
                    sym.display_code(),
                )),
                Line::from(format!(
                    "  {}  {}",
                    sym.market, q.change_display()
                )),
                Line::from(format!(
                    "  最新: {:.2}   开: {}   高: {}   低: {}",
                    q.price,
                    q.open.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.high.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    q.low.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                )),
                Line::from(format!(
                    "  成交量: {}",
                    q.volume.map(|v| {
                        if v >= 100_000_000 { format!("{:.2}亿手", v as f64 / 1e8) }
                        else if v >= 10_000 { format!("{:.2}万手", v as f64 / 1e4) }
                        else { format!("{}", v) }
                    }).unwrap_or("--".into()),
                )),
            ]
        } else {
            vec![Line::from(format!("  {}  加载中...", sym.name.as_deref().unwrap_or(&sym.code)))]
        }
    } else {
        vec![Line::from("Select a symbol to view details")]
    };

    let para = Paragraph::new(content)
        .block(Block::default().title(" Detail ").borders(Borders::ALL));
    f.render_widget(para, area);
}
