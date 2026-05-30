// crates/fa-tui/src/ui/watchlist.rs
use unicode_width::UnicodeWidthStr;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use crate::app::{FocusedPanel, State};

pub fn render(f: &mut Frame, state: &State, area: Rect) {
    let focused = state.focused_panel == FocusedPanel::Watchlist;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = state.watchlist.iter().map(|sym| {
        let quote = state.quotes.get(&sym.code);
        let (price_str, change_str, color) = if let Some(q) = quote {
            let color = if q.is_positive() { Color::Green } else { Color::Red };
            (
                format!("{:.2}", q.price),
                q.change_display(),
                color,
            )
        } else {
            ("--".into(), "".into(), Color::Gray)
        };

        let name_str = sym.name.as_deref().unwrap_or("");
        let truncated: String = name_str.chars().take(4).collect();
        let display_w = UnicodeWidthStr::width(truncated.as_str());
        let pad = 8usize.saturating_sub(display_w);
        let line = Line::from(vec![
            Span::raw(format!("{:<6}", sym.display_code())),
            Span::raw(format!(" {}{}", truncated, " ".repeat(pad))),  // unicode-aware padding
            Span::styled(format!("{:>8}", price_str), Style::default().fg(color)),
            Span::styled(format!("  {:>12}", change_str), Style::default().fg(color)),
        ]);
        ListItem::new(line)
    }).collect();

    let block_title = if state.is_add_active {
        format!(" Add: {}█ ", state.add_input)
    } else {
        " Watchlist ".to_string()
    };

    let list = List::new(items)
        .block(Block::default().title(block_title).borders(Borders::ALL).border_style(border_style))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("► ");

    let mut list_state = ListState::default();
    if !state.watchlist.is_empty() {
        list_state.select(Some(state.selected_watchlist));
    }

    f.render_stateful_widget(list, area, &mut list_state);

    if state.is_add_active && !state.search_results.is_empty() {
        render_search_overlay(f, state, area);
    }
}

fn render_search_overlay(f: &mut Frame, state: &State, area: Rect) {
    let items: Vec<ListItem> = state.search_results.iter().enumerate().map(|(i, s)| {
        let style = if i == state.search_selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        ListItem::new(Line::from(vec![
            Span::raw(format!("{:<8}", s.code)),
            Span::raw(format!("  {}", s.name)),
        ])).style(style)
    }).collect();

    let max_items = items.len().min(8) as u16;
    let available_height = area.height.saturating_sub(4);
    let overlay_height = (max_items + 2).min(available_height);
    if overlay_height == 0 {
        return;
    }
    let overlay_area = Rect {
        x: area.x + 1,
        y: area.y + 2,
        width: area.width.saturating_sub(2),
        height: overlay_height,
    };

    let list = List::new(items)
        .block(Block::default().title(" 搜索结果 ").borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));

    f.render_widget(ratatui::widgets::Clear, overlay_area);
    f.render_widget(list, overlay_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::State;
    use fa_core::{Market, Symbol};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_empty_watchlist() {
        let backend = TestBackend::new(60, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = State::default();

        terminal.draw(|f| {
            render(f, &state, f.area());
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Watchlist"));
    }

    #[test]
    fn test_render_with_symbols() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = State::default();
        state.watchlist.push(Symbol::new("AAPL", Market::USStock));

        terminal.draw(|f| {
            render(f, &state, f.area());
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("AAPL"));
    }

    #[test]
    fn test_render_shows_add_prompt_when_active() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = State::default();
        state.is_add_active = true;
        state.add_input = "TSLA".into();

        terminal.draw(|f| {
            render(f, &state, f.area());
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("TSLA"), "should show add input in title");
        assert!(content.contains("Add:"), "should show 'Add:' label");
    }

    #[test]
    fn test_render_with_symbol_name() {
        let backend = TestBackend::new(80, 15);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = State::default();
        let mut sym = Symbol::new("sh600519", Market::AShare);
        sym.name = Some("贵州茅台".into());
        state.watchlist.push(sym);

        terminal.draw(|f| {
            render(f, &state, f.area());
        }).unwrap();

        let buf = terminal.backend().buffer().clone();
        let content: String = buf.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("600519"), "should show display code");
        // Wide CJK chars have a space inserted after each in terminal buffer cells
        assert!("贵州茅台".chars().all(|c| content.contains(c)), "should show stock name");
    }
}
