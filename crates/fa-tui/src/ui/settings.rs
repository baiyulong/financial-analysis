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

    let s: &'static crate::i18n::Strings = match ss.language {
        crate::i18n::Language::Zh => &crate::i18n::ZH,
        crate::i18n::Language::En => &crate::i18n::EN,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(s.settings_title);
    let inner = block.inner(area);
    let outer = Paragraph::new(vec![Line::raw("")]).block(block);
    f.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // rows[0]: gap
            Constraint::Length(1), // rows[1]: provider
            Constraint::Length(1), // rows[2]: gap
            Constraint::Length(1), // rows[3]: URL
            Constraint::Length(1), // rows[4]: gap
            Constraint::Length(1), // rows[5]: language
            Constraint::Min(0),    // rows[6]: spacer
            Constraint::Length(1), // rows[7]: help
        ])
        .split(inner);

    let active_button = Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
    let inactive_button = Style::default().fg(Color::Gray);
    let focused_prefix = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::White);

    let provider_label = if ss.focused_field == 0 {
        format!("> {} ", s.settings_data_source)
    } else {
        format!("  {} ", s.settings_data_source)
    };
    let provider_line = Line::from(vec![
        Span::styled(
            provider_label,
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
        Span::raw(" "),
        Span::styled(
            "[ZhituAPI]",
            if ss.provider == DataSourceKind::Zhitu {
                active_button
            } else {
                inactive_button
            },
        ),
    ]);

    let url_line = if ss.provider == DataSourceKind::AkShare {
        let url_label = if ss.focused_field == 1 {
            format!("> {} ", s.settings_akshare_url)
        } else {
            format!("  {} ", s.settings_akshare_url)
        };
        let mut spans = vec![Span::styled(
            url_label,
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
        let url_label = if ss.focused_field == 1 {
            format!("> {} ", s.settings_akshare_url)
        } else {
            format!("  {} ", s.settings_akshare_url)
        };
        Line::from(vec![
            Span::styled(url_label, Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{} {}", ss.akshare_url, s.settings_akshare_only),
                Style::default().fg(Color::DarkGray),
            ),
        ])
    };

    let lang_line = Line::from(vec![
        Span::styled(
            if ss.focused_field == 2 {
                format!("> {} ", s.settings_language)
            } else {
                format!("  {} ", s.settings_language)
            },
            if ss.focused_field == 2 { focused_prefix } else { label_style },
        ),
        Span::styled(
            format!("[{}]", s.settings_lang_zh),
            if ss.language == crate::i18n::Language::Zh { active_button } else { inactive_button },
        ),
        Span::raw(" "),
        Span::styled(
            format!("[{}]", s.settings_lang_en),
            if ss.language == crate::i18n::Language::En { active_button } else { inactive_button },
        ),
    ]);

    let help = Paragraph::new(Line::from(s.settings_help))
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(Paragraph::new(provider_line), rows[1]);
    f.render_widget(Paragraph::new(url_line), rows[3]);
    f.render_widget(Paragraph::new(lang_line), rows[5]);
    f.render_widget(help, rows[7]);
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

    #[test]
    fn test_draw_settings_shows_language_row() {
        use crate::i18n::Language;
        let ss = SettingsState::new(DataSourceKind::Sina, "http://127.0.0.1:8080".into(), Language::Zh);
        let buf = render_buffer(&ss);
        let content = buffer_text(&buf);
        assert!(content.contains("Language"), "language row must appear");
        assert!("中文".chars().all(|c| content.contains(c)), "should show Chinese option");
        assert!(content.contains("English"), "should show English option");
    }
}
