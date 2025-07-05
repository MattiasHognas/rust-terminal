#[path = "../src/config.rs"] mod config;
#[path = "../src/render.rs"] mod render;

#[cfg(test)]
mod tests {
    use crate::config::{AppConfig, TableConfig, TableSource, Design, StyleElement};
    use crate::render::{render_app};
    use ratatui::{backend::TestBackend, Terminal};
    use insta::assert_snapshot;

    #[test]
    fn test_render_app() {
        let backend: TestBackend = TestBackend::new(50, 10);
        let mut terminal: Terminal<TestBackend> = Terminal::new(backend).expect("Failed to create terminal");

        let config = AppConfig {
            tables: vec![
                vec![
                    TableConfig {
                        id: "table1".to_string(),
                        table_header: Some("Table header 1".to_string()),
                        column_headers: vec!["Column header 1".to_string(), "Column header 2".to_string()],
                        column_ratios: vec![50, 50],
                        max_cell_height: 3,
                        design: Some(Design {
                            border: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            header: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            column: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            cell: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                        }),
                        source: TableSource::Static {
                            data: vec![vec!["Cell value 1".to_string(), "Cell value 2".to_string()]],
                        },
                    },
                    TableConfig {
                        id: "table2".to_string(),
                        table_header: Some("Table header 2".to_string()),
                        column_headers: vec!["Column header 1".to_string(), "Column header 2".to_string(), "Column header 3".to_string()],
                        column_ratios: vec![33, 33, 33],
                        max_cell_height: 3,
                        design: Some(Design {
                            border: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            header: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            column: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            cell: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                        }),
                        source: TableSource::Static {
                            data: vec![vec!["Cell value 1".to_string(), "Cell value 2".to_string(), "Cell value 3".to_string()]],
                        },
                    },
                ],
                vec![
                    TableConfig {
                        id: "table3".to_string(),
                        table_header: Some("Table header 3".to_string()),
                        column_headers: vec!["Column header 1".to_string()],
                        column_ratios: vec![100],
                        max_cell_height: 3,
                        design: Some(Design {
                            border: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            header: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            column: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                            cell: Some(StyleElement { color: Some("#ffffff".to_string()) }),
                        }),
                        source: TableSource::Static {
                            data: vec![vec!["Cell value 1".to_string()]],
                        },
                    },
                ],
            ]
        };

        render_app(&mut terminal, &config).expect("Failed to render app");
        // terminal.flush().expect("Failed to flush terminal");
        assert_snapshot!(terminal.backend());
    }
}