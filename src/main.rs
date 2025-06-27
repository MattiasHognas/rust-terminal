use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use serde::Deserialize;
use std::{
    error::Error,
    fs::File,
    io::{self, Read},
    sync::mpsc::{channel, Receiver},
    time::{Duration, Instant},
};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
    Terminal,
};
use reqwest::blocking;
use serde_json::Value;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
enum Align {
    Right,
    Below,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TableSource {
    Static {
        data: Vec<Vec<String>>,
    },
    File {
        path: String,
        refresh_secs: u64,
        mapping: Option<Vec<String>>,
    },
    Url {
        url: String,
        refresh_secs: u64,
        mapping: Option<Vec<String>>,
    },
}

#[derive(Deserialize, Debug, Clone)]
struct TableConfig {
    title: String,
    headers: Vec<String>,
    columnweight: Option<Vec<u16>>,
    align: Align,
    maxwidth: Option<u16>,
    maxheight: Option<usize>,
    source: TableSource,
}

struct RuntimeTable {
    config: TableConfig,
    last_update: Instant,
    data: Vec<Vec<String>>,
    last_error: Option<String>,
    retry_count: u32,
    backoff_until: Instant,
}

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_SECS: u64 = 5;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config_path = "tables.json";
    let table_configs = read_config(config_path)?;
    let mut runtime_tables: Vec<RuntimeTable> = table_configs
        .into_iter()
        .map(|cfg| {
            let (data, err) = match load_table_data(&cfg.source) {
                Ok(d) => (d, None),
                Err(e) => (vec![vec!["".to_string()]], Some(e)),
            };
            RuntimeTable {
                config: cfg,
                last_update: Instant::now(),
                data,
                last_error: err,
                retry_count: 0,
                backoff_until: Instant::now(),
            }
        })
        .collect();

    let (_watcher, rx) = setup_watcher(config_path);
    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(vec![Constraint::Length(10); runtime_tables.len()])
                .split(f.size());

            for (i, table) in runtime_tables.iter().enumerate() {
                render_table(f, chunks[i], &table.config, &table.data, &table.last_error);
            }
        })?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        if let Ok(_) = rx.try_recv() {
            let updated_configs = read_config(config_path)?;
            runtime_tables = updated_configs
                .into_iter()
                .map(|cfg| {
                    let (data, err) = match load_table_data(&cfg.source) {
                        Ok(d) => (d, None),
                        Err(e) => (vec![vec!["".to_string()]], Some(e)),
                    };
                    RuntimeTable {
                        config: cfg,
                        last_update: Instant::now(),
                        data,
                        last_error: err,
                        retry_count: 0,
                        backoff_until: Instant::now(),
                    }
                })
                .collect();
        }

        for table in runtime_tables.iter_mut() {
            match &table.config.source {
                TableSource::File { refresh_secs, .. }
                | TableSource::Url { refresh_secs, .. } => {
                    if Instant::now() < table.backoff_until {
                        continue;
                    }

                    if table.last_update.elapsed().as_secs() >= *refresh_secs {
                        match load_table_data(&table.config.source) {
                            Ok(data) => {
                                table.data = data;
                                table.last_error = None;
                                table.retry_count = 0;
                                table.backoff_until = Instant::now();
                            }
                            Err(e) => {
                                table.last_error = Some(e.clone());
                                table.retry_count += 1;
                                if table.retry_count >= MAX_RETRIES {
                                    let delay = INITIAL_BACKOFF_SECS
                                        * (1 << (table.retry_count - MAX_RETRIES));
                                    table.backoff_until =
                                        Instant::now() + Duration::from_secs(delay);
                                }
                            }
                        }

                        table.last_update = Instant::now();
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn read_config(path: &str) -> Result<Vec<TableConfig>, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Vec<TableConfig> = serde_json::from_str(&contents)?;
    Ok(config)
}

fn setup_watcher(path: &str) -> (RecommendedWatcher, Receiver<()>) {
    let (tx, rx) = channel();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if matches!(event.kind, EventKind::Modify(_)) {
                let _ = tx.send(());
            }
        }
    })
    .expect("Failed to create watcher");

    watcher
        .watch(std::path::Path::new(path), RecursiveMode::NonRecursive)
        .expect("Failed to watch config file");

    (watcher, rx)
}

fn load_table_data(source: &TableSource) -> Result<Vec<Vec<String>>, String> {
    match source {
        TableSource::Static { data } => Ok(data.clone()),
        TableSource::File { path, mapping, .. } => {
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("File error: {}", e))?;
            parse_mapped_json(&text, mapping.clone())
        }
        TableSource::Url { url, mapping, .. } => {
            let text = blocking::get(url)
                .and_then(|r| r.text())
                .map_err(|e| format!("HTTP error: {}", e))?;
            parse_mapped_json(&text, mapping.clone())
        }
    }
}

fn parse_mapped_json(text: &str, mapping: Option<Vec<String>>) -> Result<Vec<Vec<String>>, String> {
    let json: Value = serde_json::from_str(text).map_err(|e| format!("JSON error: {}", e))?;
    let array = json.as_array().ok_or("Expected array of objects")?;

    match mapping {
        Some(keys) => Ok(array
            .iter()
            .map(|obj| {
                keys.iter()
                    .map(|k| {
                        obj.get(k)
                            .map(|v| format_value(v))
                            .unwrap_or_else(|| "null".to_string())
                    })
                    .collect()
            })
            .collect()),
        None => Err("Missing mapping".to_string()),
    }
}

fn format_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() > max {
        let mut cut = text.chars().take(max.saturating_sub(3)).collect::<String>();
        cut.push_str("...");
        cut
    } else {
        text.to_string()
    }
}

fn render_table<B: tui::backend::Backend>(
    f: &mut tui::Frame<B>,
    area: tui::layout::Rect,
    config: &TableConfig,
    data: &[Vec<String>],
    error: &Option<String>,
) {
    let max_width = config.maxwidth.unwrap_or(20);
    let max_rows = config.maxheight.unwrap_or(data.len());

    let header_cells = config
        .headers
        .iter()
        .map(|h| Cell::from(truncate(h, max_width as usize)).style(Style::default().add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).bottom_margin(1);

    let rows = data.iter().take(max_rows).map(|row| {
        let cells = row.iter().map(|cell| Cell::from(truncate(cell, max_width as usize)));
        Row::new(cells)
    });

    let column_widths: Vec<Constraint> = if let Some(weights) = &config.columnweight {
        let sum: u16 = weights.iter().copied().sum();
        weights
            .iter()
            .map(|w| Constraint::Ratio(*w as u32, sum as u32))
            .collect()
    } else {
        vec![Constraint::Length(max_width); config.headers.len()]
    };

    let block_title = if let Some(e) = error {
        format!("{} (ERROR: {})", config.title, e)
    } else {
        config.title.clone()
    };

    let table = Table::new(rows)
        .header(header)
        .block(
            Block::default()
                .title(block_title)
                .borders(Borders::ALL)
                .style(if error.is_some() {
                    Style::default().fg(tui::style::Color::Red)
                } else {
                    Style::default()
                }),
        )
        .widths(&column_widths);

    f.render_widget(table, area);
}
