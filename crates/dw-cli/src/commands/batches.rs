use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use dw_client::DwClient;
use dw_client::types::batches::{BatchResponse, CreateBatchRequest, ListBatchesParams};

use crate::cli::{BatchCreateArgs, BatchRunArgs};
use crate::jsonl;
use crate::output::{Displayable, OutputFormat, format_timestamp, print_item, print_list};

impl Displayable for BatchResponse {
    fn table_headers() -> Vec<&'static str> {
        vec!["ID", "Status", "Endpoint", "Progress", "Created"]
    }

    fn to_table_row(&self) -> Vec<String> {
        let progress = self
            .request_counts
            .as_ref()
            .map(|rc| format!("{}/{}", rc.completed + rc.failed, rc.total))
            .unwrap_or_else(|| "-".to_string());

        vec![
            self.id.clone(),
            self.status.clone(),
            self.endpoint.clone(),
            progress,
            format_timestamp(self.created_at),
        ]
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn to_plain(&self) -> String {
        format!("{}\t{}", self.id, self.status)
    }
}

pub async fn create(
    client: &DwClient,
    args: &BatchCreateArgs,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let metadata = if args.metadata.is_empty() {
        None
    } else {
        let mut map = HashMap::new();
        for kv in &args.metadata {
            if let Some((k, v)) = kv.split_once('=') {
                map.insert(k.to_string(), v.to_string());
            }
        }
        Some(map)
    };

    let request = CreateBatchRequest {
        input_file_id: args.file.clone(),
        endpoint: "/v1/chat/completions".to_string(),
        completion_window: args.completion_window.clone(),
        metadata,
    };

    let batch = client.create_batch(&request).await?;
    eprintln!("Created batch: {}", batch.id);
    print_item(&batch, format);
    Ok(())
}

pub async fn list(
    client: &DwClient,
    limit: i64,
    active_first: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let params = ListBatchesParams {
        limit: Some(limit),
        active_first: if active_first { Some(true) } else { None },
        ..Default::default()
    };
    let response = client.list_batches(&params).await?;
    print_list(&response.data, format);
    Ok(())
}

pub async fn get(client: &DwClient, batch_id: &str, format: OutputFormat) -> anyhow::Result<()> {
    let batch = client.get_batch(batch_id).await?;
    print_item(&batch, format);
    Ok(())
}

pub async fn cancel(client: &DwClient, batch_id: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        eprint!("Cancel batch {}? [y/N] ", batch_id);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }
    let batch = client.cancel_batch(batch_id).await?;
    eprintln!("Cancelling batch {} (status: {}).", batch_id, batch.status);
    Ok(())
}

pub async fn retry(client: &DwClient, batch_id: &str, format: OutputFormat) -> anyhow::Result<()> {
    let batch = client.retry_batch(batch_id).await?;
    eprintln!("Retrying batch: {}", batch.id);
    print_item(&batch, format);
    Ok(())
}

pub async fn results(
    client: &DwClient,
    batch_id: &str,
    output_file: Option<&Path>,
) -> anyhow::Result<()> {
    let bytes = client.get_batch_results(batch_id).await?;

    if let Some(path) = output_file {
        tokio::fs::write(path, &bytes).await?;
        eprintln!("Results written to {}", path.display());
    } else {
        use std::io::Write;
        std::io::stdout().write_all(&bytes)?;
    }
    Ok(())
}

/// Upload file(s) and create batch(es) in one step.
pub async fn run(
    client: &DwClient,
    args: &BatchRunArgs,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let paths = collect_jsonl_paths(&args.path)?;

    if paths.is_empty() {
        anyhow::bail!("No .jsonl files found at {}", args.path.display());
    }

    for path in &paths {
        // Apply model override if specified
        let upload_path = if args.model.is_some() {
            let transforms = jsonl::Transforms {
                model: args.model.clone(),
                ..Default::default()
            };
            Some(jsonl::transform_to_temp(path, &transforms).await?)
        } else {
            None
        };

        let actual_path = upload_path.as_deref().unwrap_or(path);

        eprintln!("Uploading {}...", path.display());
        let file = client.upload_file(actual_path, "batch").await?;
        eprintln!("Uploaded: {} ({})", file.id, file.filename);

        let request = CreateBatchRequest {
            input_file_id: file.id.clone(),
            endpoint: "/v1/chat/completions".to_string(),
            completion_window: args.completion_window.clone(),
            metadata: None,
        };

        let batch = client.create_batch(&request).await?;
        eprintln!("Created batch: {}", batch.id);

        if args.watch {
            watch_batch(client, &batch.id).await?;
        } else {
            print_item(&batch, format);
        }
    }
    Ok(())
}

/// Watch a batch until completion, printing progress.
pub async fn watch_batch(client: &DwClient, batch_id: &str) -> anyhow::Result<()> {
    use std::io::Write;

    eprintln!("Watching batch {}...", batch_id);

    loop {
        let batch = client.get_batch(batch_id).await?;

        let progress = batch
            .request_counts
            .as_ref()
            .map(|rc| {
                let done = rc.completed + rc.failed;
                let pct = if rc.total > 0 {
                    (done as f64 / rc.total as f64) * 100.0
                } else {
                    0.0
                };
                format!(
                    "{}/{} ({:.0}%) — {} completed, {} failed",
                    done, rc.total, pct, rc.completed, rc.failed
                )
            })
            .unwrap_or_else(|| "waiting...".to_string());

        eprint!("\r\x1b[K  [{}] {}", batch.status, progress);
        std::io::stderr().flush()?;

        if batch.is_terminal() {
            eprintln!();
            eprintln!("Batch {} {}.", batch_id, batch.status);
            if batch.status == "completed" {
                return Ok(());
            } else {
                anyhow::bail!("Batch {} ended with status: {}", batch_id, batch.status);
            }
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Collect .jsonl file paths from a file or directory.
fn collect_jsonl_paths(path: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    if path.is_file() {
        Ok(vec![path.to_path_buf()])
    } else if path.is_dir() {
        let mut paths: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "jsonl") {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();
        paths.sort();
        Ok(paths)
    } else {
        anyhow::bail!("Path does not exist: {}", path.display());
    }
}
