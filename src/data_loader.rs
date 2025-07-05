use crate::config::{AppConfig, TableSource};
use std::fs;
use std::error::Error;
use std::collections::HashMap;
use std::time::{Instant};
use once_cell::sync::Lazy;
use std::sync::Mutex;

// Global cache to track last refresh time per table ID
static LAST_REFRESH: Lazy<Mutex<HashMap<String, Instant>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn load_all_table_data(config: &mut AppConfig) -> Result<(), Box<dyn Error>> {
    for row in &mut config.tables {
        for table in row {
            match &table.source {
                TableSource::Static { .. } => {
                    // Nothing to load
                }

                TableSource::File { path } => {
                    let raw = fs::read_to_string(path)?;
                    let data: Vec<Vec<String>> = serde_json::from_str(&raw)?;
                    table.source = TableSource::Static { data };
                }

                TableSource::Http { url, refresh_seconds } => {
                    let should_refresh = {
                        let map = LAST_REFRESH.lock().unwrap();
                        let now = Instant::now();
                        match map.get(&table.id) {
                            Some(&last) => {
                                let delay = refresh_seconds.unwrap_or(0);
                                now.duration_since(last).as_secs() >= delay
                            }
                            None => true,
                        }
                    };

                    if should_refresh {
                        let raw = reqwest::blocking::get(url)?.text()?;
                        let data: Vec<Vec<String>> = serde_json::from_str(&raw)?;
                        table.source = TableSource::Static { data };

                        let mut map = LAST_REFRESH.lock().unwrap();
                        map.insert(table.id.clone(), Instant::now());
                    }
                }
            }
        }
    }

    Ok(())
}
