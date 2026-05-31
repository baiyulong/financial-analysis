use crate::app::{DataSourceKind, SettingsState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw_settings(f: &mut Frame, area: Rect, ss: &SettingsState) {
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("⚙ 系统设置 (按 Esc 取消 / Enter 保存)");
    let inner = block.inner(area);
    let outer = Paragraph::new(vec![Line::raw("")]).block(block);
    f.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    let active_button = Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
    let inactive_button = Style::default().fg(Color::Gray);
    let focused_prefix = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::White);

    let provider_line = Line::from(vec![
        Span::styled(
            if ss.focused_field == 0 {
                "> 数据源      "
            } else {
                "  数据源      "
            },
            if ss.focused_field == 0 {
                focused_prefix
            } else {
                label_style
            },
        ),
        Span::styled(
            "[Sina]",
            if ss.provider == DataSourceKind::Sina {
                active_button
            } else {
                inactive_button
            },
        ),
        Span::raw(" "),
        Span::styled(
            "[AkShare]",
            if ss.provider == DataSourceKind::AkShare {
                active_button
            } else {
                inactive_button
            },
        ),
    ]);

    let url_line = if ss.provider == DataSourceKind::AkShare {
        let mut spans = vec![Span::styled(
            if ss.focused_field == 1 {
                "> AkShare URL "
            } else {
                "  AkShare URL "
            },
            if ss.focused_field == 1 {
                focused_prefix
            } else {
                label_style
            },
        )];
        spans.push(Span::raw(ss.akshare_url.as_str()));
        if ss.editing_url {
            spans.push(Span::styled(
                "_",
                Style::default()
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD | Modifier::SLOW_BLINK),
            ));
        }
        Line::from(spans)
    } else {
        Line::from(vec![
            Span::styled(
                if ss.focused_field == 1 {
                    "> AkShare URL "
                } else {
                    "  AkShare URL "
                },
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{} (仅 AkShare 使用)", ss.akshare_url),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    };

    let help = Paragraph::new(Line::from("↑↓ 切换 | Space 选择 | Enter 保存 | Esc 取消"))
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(Paragraph::new(provider_line), rows[1]);
    f.render_widget(Paragraph::new(url_line), rows[3]);
    f.render_widget(help, rows[6]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, buffer::Buffer, Terminal};

    fn render_buffer(ss: &SettingsState) -> Buffer {
        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_settings(f, f.area(), ss)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buf: &Buffer) -> String {
        buf.content.iter().map(|cell| cell.symbol()).collect()
    }

    fn find_text_start(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
        for y in 0..buf.area.height {
            let row: String = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
            if let Some(idx) = row.find(needle) {
                return Some((idx as u16, y));
            }
        }
        None
    }

    #[test]
    fn test_draw_settings_shows_title_help_and_selected_provider() {
        let ss = SettingsState::new(DataSourceKind::AkShare, "http://127.0.0.1:8080".into(), crate::i18n::Language::default());
        let buf = render_buffer(&ss);
        let content = buffer_text(&buf);

        assert!("系统设置".chars().all(|c| content.contains(c)));
        for hint in ["Space", "Enter", "Esc"] {
            assert!(content.contains(hint));
        }
        assert!(content.contains("[Sina]"));
        assert!(content.contains("[AkShare]"));

        let (x, y) = find_text_start(&buf, "[AkShare]").expect("AkShare button should render");
        let cell = &buf[(x, y)];
        assert!(cell.modifier.contains(Modifier::REVERSED));
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_draw_settings_greys_out_url_for_sina() {
        let ss = SettingsState::new(DataSourceKind::Sina, "http://127.0.0.1:8080".into(), crate::i18n::Language::default());
        let buf = render_buffer(&ss);
        let content = buffer_text(&buf);

        assert!(content.contains("AkShare"));
        for ch in ['仅', '使', '用'] {
            assert!(content.contains(ch));
        }
        let (x, y) = find_text_start(&buf, "http://127.0.0.1:8080")
            .expect("URL should render in disabled row");
        assert_eq!(buf[(x, y)].fg, Color::DarkGray);
    }

    #[test]
    fn test_draw_settings_shows_cursor_while_editing_url() {
        let mut ss = SettingsState::new(DataSourceKind::AkShare, "http://127.0.0.1:8080".into(), crate::i18n::Language::default());
        ss.focused_field = 1;
        ss.editing_url = true;

        let buf = render_buffer(&ss);
        let content = buffer_text(&buf);

        assert!(content.contains("http://127.0.0.1:8080_"));
    }
}
