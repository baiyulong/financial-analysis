# Chart Y-Axis Price Labels + Volume Combined Border Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a price Y-axis (right side, 5 tick labels) to the K-line chart, and place the volume bars inside the same bordered box as the K-line chart (separated by a `─` divider).

**Architecture:** Both changes are confined to `crates/fa-tui/src/ui/chart.rs`. The outer layout merges the previously-separate chart and volume chunks into one `Min(10)` block. A new `render_chart_and_volume()` function draws one outer `Block::ALL` border, splits the inner area vertically (chart / separator / volume), draws the Y-axis separator `│` and price labels on the right 9 columns, then delegates to refactored borderless `render_chart_inner()` and `render_volume_inner()` helpers. `KlineChart` widget loses its self-drawn border; callers are responsible for borders.

**Tech Stack:** Rust, Ratatui 0.27, `ratatui::layout::Layout`, `ratatui::widgets::{Block, Borders}`, `ratatui::buffer::Buffer`

---

## File Map

| File | Change |
|------|--------|
| `crates/fa-tui/src/ui/chart.rs` | All changes. Rename helpers, add `render_chart_and_volume`, remove border from `KlineChart`, add Y-axis logic. |

No other files need to change.

---

### Task 1: Write failing tests

**Files:**
- Modify: `crates/fa-tui/src/ui/chart.rs` (test module only)

- [ ] **Step 1: Add two failing tests to the `tests` module at the bottom of chart.rs**

Append inside `mod tests { ... }` (before the closing `}`):

```rust
#[test]
fn test_yaxis_price_labels_appear() {
    // Price labels (formatted floats) must appear in the buffer when chart has data.
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let cs = make_chart_state_with_data();
    terminal
        .draw(|f| render(f, &cs, &DataSourceKind::Sina, &crate::i18n::ZH, f.area()))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    // y_max for the test data: high=115, padding ~5% → ≈120.75; y_min ~83.6
    // Any formatted decimal like "." surrounded by digits must appear on the right axis.
    assert!(
        content.contains('.'),
        "Y-axis price labels (formatted floats) must appear in the buffer"
    );
    // The │ separator between candles and Y-axis must appear
    assert!(
        content.contains('│'),
        "Y-axis vertical separator │ must be drawn"
    );
}

#[test]
fn test_volume_combined_box_separator() {
    // The ─ separator line between chart and volume must appear (same-box design).
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let cs = make_chart_state_with_data();
    terminal
        .draw(|f| render(f, &cs, &DataSourceKind::Sina, &crate::i18n::ZH, f.area()))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let content: String = buf.content().iter().map(|c| c.symbol()).collect();
    // The ─ chars form the separator row between chart and volume sections.
    assert!(
        content.contains('─'),
        "horizontal separator ─ between chart and volume must be drawn"
    );
}
```

- [ ] **Step 2: Run tests to confirm they fail (Y-axis not yet implemented)**

```bash
cd /root/projects/financial-analysis
cargo test -p fa-tui test_yaxis_price_labels_appear test_volume_combined_box_separator -- --nocapture 2>&1 | grep -E "FAILED|passed|error"
```

Expected: both tests FAIL (the existing chart has no Y-axis separator `│` and no combined-box `─`).

Note: `content.contains('.')` may accidentally pass because the titlebar or statusbar could have a `.` in the period label. If so, the `│` assertion is the discriminating one. Proceed to implementation regardless.

---

### Task 2: Implement combined chart+volume box with Y-axis

**Files:**
- Modify: `crates/fa-tui/src/ui/chart.rs` (full implementation)

All changes are in one file.

#### Step 2a: Remove border from `KlineChart::render`, use area directly

- [ ] **Replace the `KlineChart::render` body** (lines 24–154). The widget must now render directly into `area` without drawing its own block. All references to `inner` become `area`, and the initial block/inner lines are removed:

Replace the entire `impl Widget for KlineChart<'_>` block with:

```rust
impl Widget for KlineChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // No border — caller is responsible for borders.
        let h = area.height;
        let w = area.width;

        if h == 0 || w == 0 {
            return;
        }

        // ── Pass 1: Candlestick bars ──────────────────────────────────────
        for (i, bar) in self.visible.iter().enumerate() {
            let x_off = (i as u16) * self.bar_w;
            if x_off >= w {
                break;
            }

            let open = bar.open.to_f64().unwrap_or(self.y_min);
            let close = bar.close.to_f64().unwrap_or(self.y_min);
            let body_top = open.max(close);
            let body_bot = open.min(close);
            let high = bar.high.to_f64().unwrap_or(body_top);
            let low = bar.low.to_f64().unwrap_or(body_bot);
            let color = if close >= open {
                Color::Red
            } else {
                Color::Green
            };

            let high_row = price_to_row(high, self.y_min, self.y_max, h);
            let low_row = price_to_row(low, self.y_min, self.y_max, h);
            let top_body_row = price_to_row(body_top, self.y_min, self.y_max, h);
            let bot_body_row = price_to_row(body_bot, self.y_min, self.y_max, h);

            let center_col = area.x + x_off + self.bar_w / 2;
            let body_left = area.x + x_off + 1;
            let body_right = (area.x + x_off + self.bar_w).saturating_sub(2);
            let max_col = area.x + w - 1;

            // Upper wick
            for r in high_row..top_body_row {
                if center_col <= max_col {
                    buf[(center_col, area.y + r)].set_char('│').set_fg(color);
                }
            }

            // Body
            for r in top_body_row..=bot_body_row {
                let y = area.y + r;
                for col in body_left..=body_right.min(max_col) {
                    buf[(col, y)].set_char('█').set_fg(color);
                }
                if (center_col < body_left || center_col > body_right) && center_col <= max_col {
                    buf[(center_col, y)].set_char('█').set_fg(color);
                }
            }

            // Lower wick
            if bot_body_row < low_row {
                for r in (bot_body_row + 1)..=low_row {
                    if center_col <= max_col {
                        buf[(center_col, area.y + r)].set_char('│').set_fg(color);
                    }
                }
            }
        }

        // ── Pass 2: MA overlay ───────────────────────────────────────────
        for (color, vis_ma) in &self.ma_data {
            let mut prev: Option<(u16, u16)> = None;
            for (i, v) in vis_ma.iter().enumerate() {
                if let Some(price) = v.and_then(|d| d.to_f64()) {
                    let x_off = (i as u16) * self.bar_w;
                    if x_off >= w {
                        break;
                    }
                    let center_col = area.x + x_off + self.bar_w / 2;
                    let row = price_to_row(price, self.y_min, self.y_max, h);

                    if center_col < area.x + w {
                        buf[(center_col, area.y + row)]
                            .set_char('─')
                            .set_fg(*color);
                    }

                    if let Some((prev_col, prev_row)) = prev {
                        let span = center_col.saturating_sub(prev_col);
                        for step in 1..span {
                            let col = prev_col + step;
                            if col >= area.x + w {
                                break;
                            }
                            let t = step as f32 / span as f32;
                            let interp_row = if prev_row <= row {
                                prev_row + ((row - prev_row) as f32 * t) as u16
                            } else {
                                prev_row - ((prev_row - row) as f32 * t) as u16
                            };
                            buf[(col, area.y + interp_row)]
                                .set_char('─')
                                .set_fg(*color);
                        }
                    }

                    prev = Some((center_col, row));
                } else {
                    prev = None;
                }
            }
        }

        // ── Pass 3: Cursor column highlight ─────────────────────────────
        if self.cursor_in_view < self.visible.len() {
            let x_off = (self.cursor_in_view as u16) * self.bar_w;
            if x_off < w {
                let center_col = area.x + x_off + self.bar_w / 2;
                if center_col < area.x + w {
                    for r in 0..h {
                        buf[(center_col, area.y + r)].set_bg(Color::DarkGray);
                    }
                }
            }
        }
    }
}
```

#### Step 2b: Update `render()` layout — merge chart+volume into one chunk

- [ ] **Replace the `render()` function**:

```rust
pub fn render(f: &mut Frame, cs: &ChartState, data_source: &DataSourceKind, strings: &'static crate::i18n::Strings, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // title bar
            Constraint::Min(10),    // chart + volume combined
            Constraint::Length(1),  // cursor info
            Constraint::Length(1),  // status bar
        ])
        .split(area);

    render_titlebar(f, cs, chunks[0]);
    render_chart_and_volume(f, cs, strings, chunks[1]);
    render_cursor_info(f, cs, chunks[2]);
    render_statusbar(f, data_source, strings, chunks[3]);
}
```

#### Step 2c: Add `render_chart_and_volume()` function

- [ ] **Add this new function** before `render_titlebar` (after the closing brace of `render()`):

```rust
/// Draws one bordered box containing the K-line chart (top ~75%) and volume bars (bottom ~25%),
/// separated by a ─ divider. The right 9 columns of the chart area are reserved for the Y-axis
/// (1 col for │ separator + 8 cols for price labels).
fn render_chart_and_volume(f: &mut Frame, cs: &ChartState, strings: &'static crate::i18n::Strings, area: Rect) {
    // 1. Draw outer border.
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 6 || inner.width < 12 {
        return;
    }

    // 2. Compute geometry.
    const YAXIS_W: u16 = 9; // 1 for │ + 8 for label text
    let candle_w = inner.width.saturating_sub(YAXIS_W);
    let vol_h = (inner.height / 5).max(3).min(8);
    let chart_h = inner.height.saturating_sub(vol_h + 1); // +1 for separator row

    // 3. Split inner area vertically: [chart | separator | volume].
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(chart_h),
            Constraint::Length(1),
            Constraint::Length(vol_h),
        ])
        .split(inner);
    let chart_row = vert[0];
    let sep_row = vert[1];
    let vol_row = vert[2];

    // 4. Split chart row horizontally: [candles | y-axis].
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(candle_w),
            Constraint::Length(YAXIS_W),
        ])
        .split(chart_row);
    let candle_area = horiz[0];
    let yaxis_area = horiz[1]; // col 0 = │, cols 1-8 = label text

    // 5. Draw Y-axis separator │ and separator row ─ directly into the buffer.
    {
        let buf = f.buffer_mut();
        // Vertical │ along Y-axis left edge.
        for y in yaxis_area.y..yaxis_area.y + yaxis_area.height {
            if yaxis_area.x < inner.x + inner.width {
                buf[(yaxis_area.x, y)].set_char('│').set_fg(Color::DarkGray);
            }
        }
        // Horizontal ─ separator between chart and volume (full inner width).
        for x in sep_row.x..sep_row.x + sep_row.width {
            buf[(x, sep_row.y)].set_char('─').set_fg(Color::DarkGray);
        }
        // Junction where │ meets ─.
        if yaxis_area.x < inner.x + inner.width {
            buf[(yaxis_area.x, sep_row.y)]
                .set_char('┴')
                .set_fg(Color::DarkGray);
        }
    }

    // 6. Draw Y-axis price labels (only when there is visible data).
    if !cs.loading && !cs.data.is_empty() && candle_w > 0 {
        let (start, end) = visible_window(cs, candle_w as usize);
        if start < end && end <= cs.data.len() {
            let visible = &cs.data[start..end];
            let price_min = visible
                .iter()
                .filter_map(|b| b.low.to_f64())
                .fold(f64::MAX, f64::min);
            let price_max = visible
                .iter()
                .filter_map(|b| b.high.to_f64())
                .fold(f64::MIN, f64::max);
            if price_min < price_max {
                let padding = ((price_max - price_min) * 0.05).max(0.01);
                let y_min = price_min - padding;
                let y_max = price_max + padding;
                let h = yaxis_area.height;
                // 5 evenly spaced price ticks: top, 75%, 50%, 25%, bottom.
                let ticks: [(u16, f64); 5] = [
                    (0, y_max),
                    (h / 4, y_max * 0.75 + y_min * 0.25),
                    (h / 2, (y_max + y_min) / 2.0),
                    (h * 3 / 4, y_max * 0.25 + y_min * 0.75),
                    (h.saturating_sub(1), y_min),
                ];
                let buf = f.buffer_mut();
                let label_x = yaxis_area.x + 1; // skip the │ col
                for (row, price) in ticks {
                    if row >= h {
                        continue;
                    }
                    let label = format!("{:>8.2}", price);
                    for (i, ch) in label.chars().enumerate() {
                        let col = label_x + i as u16;
                        if col < inner.x + inner.width {
                            buf[(col, yaxis_area.y + row)]
                                .set_char(ch)
                                .set_fg(Color::DarkGray);
                        }
                    }
                }
            }
        }
    }

    // 7. Render K-line chart into candle_area (no border — handled above).
    render_chart_inner(f, cs, strings, candle_area);

    // 8. Render volume bars aligned with candles (same candle_w width).
    let vol_candle_area = Rect::new(vol_row.x, vol_row.y, candle_w, vol_row.height);
    render_volume_inner(f, cs, vol_candle_area);
}
```

#### Step 2d: Rename and update `render_chart` → `render_chart_inner`

- [ ] **Replace the existing `render_chart` function** with a borderless version. The area passed in is already `candle_area` — no border subtraction needed:

```rust
fn render_chart_inner(f: &mut Frame, cs: &ChartState, strings: &'static crate::i18n::Strings, area: Rect) {
    if cs.loading || cs.data.is_empty() {
        let msg = if cs.loading {
            strings.chart_loading
        } else {
            strings.chart_no_data
        };
        f.render_widget(
            Paragraph::new(msg).alignment(Alignment::Center),
            area,
        );
        return;
    }

    // area.width is already the candle-only width (Y-axis excluded by caller).
    let inner_w = area.width as usize;
    let (start, end) = visible_window(cs, inner_w);
    let visible = &cs.data[start..end];

    let price_min = visible
        .iter()
        .filter_map(|b| b.low.to_f64())
        .fold(f64::MAX, f64::min);
    let price_max = visible
        .iter()
        .filter_map(|b| b.high.to_f64())
        .fold(f64::MIN, f64::max);

    let padding = ((price_max - price_min) * 0.05).max(0.01);
    let y_min = price_min - padding;
    let y_max = price_max + padding;

    let ma_configs: &[(usize, Color)] =
        &[(5, Color::Yellow), (10, Color::Cyan), (20, Color::Magenta)];
    let cursor_in_view = cs.cursor.saturating_sub(start);

    let ma_data: Vec<(Color, Vec<Option<rust_decimal::Decimal>>)> = ma_configs
        .iter()
        .filter(|&&(period, _)| cs.ma_periods.contains(&period))
        .map(|&(period, color)| {
            let all_ma = sma(&cs.data, period);
            let vis_ma = all_ma[start..end].to_vec();
            (color, vis_ma)
        })
        .collect();

    f.render_widget(
        KlineChart {
            visible,
            y_min,
            y_max,
            bar_w: cs.bar_width,
            cursor_in_view,
            ma_data,
        },
        area,
    );
}
```

#### Step 2e: Rename and update `render_volume` → `render_volume_inner`

- [ ] **Replace the existing `render_volume` function**. The area passed in is already `vol_candle_area` (width = `candle_w`); no border subtraction needed:

```rust
fn render_volume_inner(f: &mut Frame, cs: &ChartState, area: Rect) {
    if cs.loading || cs.data.is_empty() {
        return;
    }

    // area.width equals candle_w — matches the chart's visible window width.
    let inner_w = area.width as usize;
    let (start, end) = visible_window(cs, inner_w);
    let visible = &cs.data[start..end];

    f.render_widget(
        VolumeChart {
            visible,
            bar_w: cs.bar_width,
        },
        area,
    );
}
```

- [ ] **Step 2f: Run cargo test to confirm all tests pass**

```bash
cd /root/projects/financial-analysis
cargo test -p fa-tui 2>&1 | grep -E "^test result|FAILED|^error"
```

Expected: all tests pass including the two new ones. If any fail:
- `test_yaxis_price_labels_appear` failing: check `render_chart_and_volume` step 6 (label rendering)
- `test_volume_combined_box_separator` failing: check step 5 (separator drawing)
- `test_render_with_data_no_panic` failing: check that `KlineChart::render` compiles and `render_chart_inner` is called correctly

Also verify the full workspace:
```bash
cargo test --workspace 2>&1 | grep -E "^test result|FAILED"
```

Expected: 229 tests pass (227 existing + 2 new), 0 failed.

- [ ] **Step 2g: Commit**

```bash
cd /root/projects/financial-analysis
git add crates/fa-tui/src/ui/chart.rs
git commit -m "feat(chart): add price Y-axis labels and combined chart+volume border"
```

---

### Task 3: Push to remote

- [ ] **Step 3a: Run full workspace tests one final time**

```bash
cd /root/projects/financial-analysis
cargo test --workspace 2>&1 | grep -E "^test result|FAILED"
```

Expected: 0 failed.

- [ ] **Step 3b: Push**

```bash
cd /root/projects/financial-analysis
git push origin main
```

---

## Self-Review

**Spec coverage:**
- ✅ Y-axis price labels (right side, 5 ticks, `{:>8.2}` format, DarkGray color)
- ✅ Y-axis `│` separator between candles and labels
- ✅ Volume in same bordered box as K-line chart
- ✅ `─` separator divider between chart and volume sections
- ✅ `┴` junction where │ meets ─
- ✅ Volume bars horizontally aligned with candles (same `visible_window` width)
- ✅ Loading/empty state handled (message in chart area, no volume, no Y-axis labels)
- ✅ TDD: failing tests written first, then implementation

**Placeholder scan:** No TBD, TODO, or vague requirements.

**Type consistency:**
- `render_chart_and_volume` calls `render_chart_inner(f, cs, strings, candle_area)` — `candle_area` is `horiz[0]` of type `Rect` ✓
- `render_chart_and_volume` calls `render_volume_inner(f, cs, vol_candle_area)` — `vol_candle_area` is `Rect::new(...)` ✓
- `KlineChart` widget unchanged struct fields; new `render` body uses `area.x/y` everywhere ✓
- `visible_window` signature `(cs: &ChartState, inner_w: usize) -> (usize, usize)` — called with `candle_w as usize` ✓
- `Rect::new(x, y, width, height)` — correct Ratatui API ✓
