use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};
use crate::config::{AppConfig, TableConfig, TableSource};

pub fn render_app<B: Backend>(terminal: &mut Terminal<B>, config: &AppConfig) -> Result<(), std::io::Error> {
    terminal.draw(|f| {
        let total_area = f.area();

        let row_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(100 / config.tables.len() as u16); config.tables.len()])
            .split(total_area);

        for (row_idx, row) in config.tables.iter().enumerate() {
            let col_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(100 / row.len() as u16); row.len()])
                .split(row_chunks[row_idx]);

            for (col_idx, table) in row.iter().enumerate() {
                render_table::<B>(f, col_chunks[col_idx], table);
            }
        }
    })?;

    Ok(())
}

fn render_table<B: Backend>(f: &mut Frame<'_>, area: Rect, config: &TableConfig) {
    let headers = Row::new(config.column_headers.clone())
        .style(color_from_design(config, "header"));

    let column_constraints: Vec<Constraint> = config
        .column_ratios
        .iter()
        .map(|p| Constraint::Percentage(*p))
        .collect();

    let rows = match &config.source {
        TableSource::Static { data } => data
            .iter()
            .map(|row| {
                let cells = row.iter().map(|cell| {
                    let wrapped = wrap_and_truncate(cell, config.max_cell_height);
                    Cell::from(wrapped).style(color_from_design(config, "cell"))
                });
                Row::new(cells)
            })
            .collect(),
        _ => vec![],
    };

    let mut table = Table::new(rows, column_constraints)
        .header(headers);

    if let Some(title) = &config.table_header {
        let block = Block::default()
            .title(title.as_str())
            .borders(Borders::ALL)
            .border_style(color_from_design(config, "border"));
        table = table.block(block);
    }

    f.render_widget(table, area);
}

fn color_from_design(config: &TableConfig, element: &str) -> Style {
    use ratatui::style::Style;

    let color = match (element, &config.design) {
        ("border", Some(d)) => &d.border,
        ("header", Some(d)) => &d.header,
        ("column", Some(d)) => &d.column,
        ("cell", Some(d)) => &d.cell,
        _ => &None,
    };

    if let Some(style_elem) = color {
        if let Some(hex) = &style_elem.color {
            if let Ok(parsed) = parse_hex_color(hex) {
                return Style::default().fg(parsed);
            }
        }
    }

    Style::default()
}

fn parse_hex_color(hex: &str) -> Result<Color, ()> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return Err(()); }
    if let Ok(rgb) = u32::from_str_radix(hex, 16) {
        let r = ((rgb >> 16) & 0xFF) as u8;
        let g = ((rgb >> 8) & 0xFF) as u8;
        let b = (rgb & 0xFF) as u8;
        return Ok(Color::Rgb(r, g, b));
    }
    Err(())
}

fn wrap_and_truncate(text: &str, max_lines: usize) -> String {
    let lines: Vec<String> = text
        .lines()
        .flat_map(|line| textwrap::wrap(line, 30)) // fixed width wrap
        .map(|w| w.into_owned())
        .collect();

    if lines.len() > max_lines {
        let mut truncated = lines[..max_lines].to_vec();
        if let Some(last) = truncated.last_mut() {
            *last = format!("{}...", last.trim_end_matches('.'));
        }
        truncated.join("\n")
    } else {
        lines.join("\n")
    }
}