use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub tables: Vec<Vec<TableConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TableConfig {
#[allow(dead_code)]
    pub id: String,
#[deny(dead_code)]
    pub table_header: Option<String>,
    pub column_headers: Vec<String>,
    pub column_ratios: Vec<u16>,
    pub max_cell_height: usize,
    pub source: TableSource,
    pub design: Option<Design>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
#[allow(dead_code)]
pub enum TableSource {
    Static { data: Vec<Vec<String>> },
    File { path: String },
    Http { url: String, refresh_seconds: Option<u64> },
}
#[deny(dead_code)]

#[derive(Debug, Deserialize, Clone)]
pub struct Design {
    pub border: Option<StyleElement>,
    pub header: Option<StyleElement>,
    pub column: Option<StyleElement>,
    pub cell: Option<StyleElement>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StyleElement {
    pub color: Option<String>,
}

impl AppConfig {
    #[allow(dead_code)]
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
    #[deny(dead_code)]
        let data = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&data)?;
        Ok(config)
    }
}