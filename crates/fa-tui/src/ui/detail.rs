// crates/fa-tui/src/ui/detail.rs
use crate::app::State;
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let s = state.strings();
    let content = if let Some(sym) = state.selected_symbol() {
        if let Some(q) = state.quotes.get(&sym.code) {
            let fallback_code = sym.display_code();
            let display_name = q
                .name
                .as_deref()
                .or(sym.name.as_deref())
                .unwrap_or(fallback_code.as_str());
            vec![
                Line::from(format!("  {}  {}", display_name, sym.display_code(),)),
                Line::from(format!("  {}  {}", sym.market, q.change_display())),
                Line::from(format!(
                    "  {}: {:.2}   {}: {}   {}: {}   {}: {}",
                    s.detail_latest,
                    q.price,
                    s.detail_open,
                    q.open.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    s.detail_high,
                    q.high.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                    s.detail_low,
                    q.low.map(|v| format!("{:.2}", v)).unwrap_or("--".into()),
                )),
                Line::from(format!(
                    "  {}: {}",
                    s.detail_volume,
                    q.volume
                        .map(|v| {
                            if v >= 100_000_000 {
                                format!("{:.2}{}", v as f64 / 1e8, s.detail_vol_unit_yi)
                            } else if v >= 10_000 {
                                format!("{:.2}{}", v as f64 / 1e4, s.detail_vol_unit_wan)
                            } else {
                                format!("{}", v)
                            }
                        })
                        .unwrap_or("--".into()),
                )),
            ]
        } else {
            let label = sym
                .name
                .as_deref()
                .map(|n| n.to_string())
                .unwrap_or_else(|| sym.display_code());
            vec![Line::from(format!("  {}  {}", label, s.detail_loading))]
        }
    } else {
        vec![Line::from(s.detail_select_hint)]
    };

    let para = Paragraph::new(content).block(
        Block::default().title(s.detail_title).borders(Borders::ALL),
    );
    f.render_widget(para, area);
}
