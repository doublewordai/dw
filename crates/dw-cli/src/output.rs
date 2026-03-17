use comfy_table::{ContentArrangement, Table};
use std::io::IsTerminal;

/// Output format for CLI results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Plain,
}

impl OutputFormat {
    /// Determine the default output format based on whether stdout is a TTY.
    pub fn default_for_stdout() -> Self {
        if std::io::stdout().is_terminal() {
            OutputFormat::Table
        } else {
            OutputFormat::Json
        }
    }

    #[allow(dead_code)]
    pub fn from_str_or_default(s: Option<&str>) -> Self {
        match s {
            Some("json") => OutputFormat::Json,
            Some("table") => OutputFormat::Table,
            Some("plain") => OutputFormat::Plain,
            _ => Self::default_for_stdout(),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "plain" => Ok(OutputFormat::Plain),
            _ => Err(format!(
                "Invalid output format: '{}'. Use table, json, or plain.",
                s
            )),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Plain => write!(f, "plain"),
        }
    }
}

/// Trait for types that can be displayed in multiple output formats.
pub trait Displayable {
    fn table_headers() -> Vec<&'static str>;
    fn to_table_row(&self) -> Vec<String>;
    #[allow(dead_code)]
    fn to_json(&self) -> serde_json::Value;
    fn to_plain(&self) -> String;
}

/// Print a list of displayable items in the requested format.
pub fn print_list<T: Displayable + serde::Serialize>(items: &[T], format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            if items.is_empty() {
                println!("No results.");
                return;
            }
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(T::table_headers());
            for item in items {
                table.add_row(item.to_table_row());
            }
            println!("{table}");
        }
        OutputFormat::Json => {
            for item in items {
                println!("{}", serde_json::to_string(&item).unwrap_or_default());
            }
        }
        OutputFormat::Plain => {
            for item in items {
                println!("{}", item.to_plain());
            }
        }
    }
}

/// Print a single displayable item in the requested format.
pub fn print_item<T: Displayable + serde::Serialize>(item: &T, format: OutputFormat) {
    match format {
        OutputFormat::Table => {
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(T::table_headers());
            table.add_row(item.to_table_row());
            println!("{table}");
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&item).unwrap_or_default()
            );
        }
        OutputFormat::Plain => {
            println!("{}", item.to_plain());
        }
    }
}

/// Format a unix timestamp as a human-readable string.
pub fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// Format bytes as human-readable size.
pub fn format_bytes(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = 1024 * 1024;
    const GB: i64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
