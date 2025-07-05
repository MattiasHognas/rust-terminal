mod config;
mod data_loader;
mod render;
mod watcher;

use config::AppConfig;
use data_loader::load_all_table_data;
use render::render_app;
use std::{sync::{Arc, Mutex}};
use crossterm::terminal::{enable_raw_mode};
use crossterm::event::{self, Event as CEvent, KeyCode};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::stdout;
use std::sync::mpsc::channel;
use watcher::setup_watcher;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = "tables.json";
    let app_config = AppConfig::load_from_file(config_path)?;

    enable_raw_mode()?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let config_arc = Arc::new(Mutex::new(app_config));
    let config_clone = config_arc.clone();

    let (tx, rx) = channel();
    let _watcher = setup_watcher(config_path, tx)?;

    loop {
        if event::poll(std::time::Duration::from_millis(500))? {
            match event::read()? {
                CEvent::Resize(_, _) => {
                    let guard = config_arc.lock().unwrap();
                    render_app(&mut terminal, &guard)?;
                }
                CEvent::Key(key) if key.code == KeyCode::Char('q') => break Ok(()),
                _ => {}
            }
        }

        if let Ok(_event) = rx.try_recv() {
            let new_config = AppConfig::load_from_file(config_path)?;
            *config_clone.lock().unwrap() = new_config;
        }

        {
            let mut guard = config_arc.lock().unwrap();
            load_all_table_data(&mut guard)?;
            render_app(&mut terminal, &guard)?;
        }
    }

    // disable_raw_mode()?; // unreachable
}
