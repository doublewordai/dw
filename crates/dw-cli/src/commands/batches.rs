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

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Creating batch...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    let batch = client.create_batch(&request).await?;
    spinner.finish_with_message(format!("Created batch: {}", batch.id));
    print_item(&batch, format);
    Ok(())
}

pub async fn list(
    client: &DwClient,
    limit: i64,
    after: Option<&str>,
    all: bool,
    active_first: bool,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let active_first_param = if active_first { Some(true) } else { None };

    if all {
        let mut all_batches = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = ListBatchesParams {
                limit: Some(100),
                after: cursor,
                active_first: active_first_param,
            };
            let response = client.list_batches(&params).await?;
            let has_more = response.has_more;
            let last_id = response.last_id.clone();
            all_batches.extend(response.data);
            if !has_more {
                break;
            }
            cursor = last_id;
        }
        print_list(&all_batches, format);
    } else {
        let params = ListBatchesParams {
            limit: Some(limit),
            after: after.map(|s| s.to_string()),
            active_first: active_first_param,
        };
        let response = client.list_batches(&params).await?;
        print_list(&response.data, format);
        if response.has_more
            && let Some(last_id) = &response.last_id
            && format != OutputFormat::Json
        {
            eprintln!(
                "More batches available. Next page: dw batches list --after {}",
                last_id
            );
        }
    }
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
    ids: &[String],
    from_file: Option<&Path>,
    output_file: Option<&Path>,
) -> anyhow::Result<()> {
    let batch_ids = resolve_batch_ids(ids, from_file).await?;

    if let Some(path) = output_file {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await?;
        }
        // Write to a temp file and rename on success to avoid partial output.
        // Batches are fetched sequentially to preserve output order; the typical
        // case is 1–10 IDs so concurrency adds complexity without meaningful gain.
        let tmp = path.with_extension("jsonl.tmp");
        let write_result = async {
            let file = tokio::fs::File::create(&tmp).await?;
            let mut writer = tokio::io::BufWriter::new(file);
            for batch_id in &batch_ids {
                let bytes = client.get_batch_results(batch_id).await?;
                tokio::io::AsyncWriteExt::write_all(&mut writer, &bytes).await?;
                if !bytes.ends_with(b"\n") {
                    tokio::io::AsyncWriteExt::write_all(&mut writer, b"\n").await?;
                }
            }
            tokio::io::AsyncWriteExt::flush(&mut writer).await?;
            anyhow::Ok(())
        }
        .await;
        if let Err(e) = write_result {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }
        // On Unix, rename overwrites atomically. On Windows it may fail
        // if the destination exists, so remove it first when present.
        if tokio::fs::try_exists(path).await.unwrap_or(false) {
            let _ = tokio::fs::remove_file(path).await;
        }
        if let Err(e) = tokio::fs::rename(&tmp, path).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e.into());
        }
        eprintln!(
            "Results written to {} ({} batch{})",
            path.display(),
            batch_ids.len(),
            if batch_ids.len() == 1 { "" } else { "es" }
        );
    } else {
        use std::io::Write;
        for batch_id in &batch_ids {
            let bytes = client.get_batch_results(batch_id).await?;
            // Lock stdout only for the write, not across await points
            let mut out = std::io::stdout().lock();
            out.write_all(&bytes)?;
            if !bytes.ends_with(b"\n") {
                out.write_all(b"\n")?;
            }
        }
    }
    Ok(())
}

/// Upload file(s) and create batch(es) in one step.
///
/// When `--watch` is used with multiple files, each batch starts being
/// watched as soon as it's created — uploads and watches run concurrently.
pub async fn run(
    client: &DwClient,
    args: &BatchRunArgs,
    format: OutputFormat,
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
    let paths = collect_jsonl_paths(&args.path)?;

    if paths.is_empty() {
        anyhow::bail!("No .jsonl files found at {}", args.path.display());
    }

    let multi = if args.watch && paths.len() > 1 {
        Some(indicatif::MultiProgress::new())
    } else {
        None
    };

    let mut watch_handles: Vec<tokio::task::JoinHandle<anyhow::Result<()>>> = Vec::new();
    let mut batch_ids: Vec<String> = Vec::new();

    for path in &paths {
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

        let spinner = if let Some(ref mp) = multi {
            mp.add(indicatif::ProgressBar::new_spinner())
        } else {
            indicatif::ProgressBar::new_spinner()
        };
        spinner.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message(format!("Uploading {}...", path.display()));
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        let file = client.upload_file(actual_path, "batch").await?;
        spinner.finish_with_message(format!("Uploaded {} ({})", file.id, file.filename));

        let request = CreateBatchRequest {
            input_file_id: file.id.clone(),
            endpoint: "/v1/chat/completions".to_string(),
            completion_window: args.completion_window.clone(),
            metadata: None,
        };

        let spinner = if let Some(ref mp) = multi {
            mp.add(indicatif::ProgressBar::new_spinner())
        } else {
            indicatif::ProgressBar::new_spinner()
        };
        spinner.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message("Creating batch...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        let batch = client.create_batch(&request).await?;
        spinner.finish_with_message(format!("Created batch: {}", batch.id));
        batch_ids.push(batch.id.clone());

        if args.watch {
            // Spawn watch immediately — runs concurrently while we upload the next file
            let client = client.clone();
            let batch_id = batch.id.clone();
            let multi_clone = multi.clone();
            watch_handles.push(tokio::spawn(async move {
                watch_single(
                    &client,
                    &batch_id,
                    multi_clone.as_ref(),
                    poll_interval_secs,
                    max_retries,
                )
                .await
            }));
        } else {
            print_item(&batch, format);
        }
    }

    // Write batch IDs to file if requested
    if let Some(ref id_path) = args.output_id {
        use std::io::Write;
        if let Some(parent) = id_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let mut f = std::fs::File::create(id_path)?;
        for id in &batch_ids {
            writeln!(f, "{}", id)?;
        }
    }

    // Wait for all watches to complete
    let mut had_failure = false;
    for handle in watch_handles {
        if let Err(e) = handle.await? {
            eprintln!("Error: {}", e);
            had_failure = true;
        }
    }
    if had_failure {
        anyhow::bail!("One or more batches failed");
    }

    Ok(())
}

/// Watch one or more batches until completion with parallel progress bars.
pub async fn watch_batches(
    client: &DwClient,
    batch_ids: &[String],
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
    if batch_ids.len() == 1 {
        return watch_single(client, &batch_ids[0], None, poll_interval_secs, max_retries).await;
    }

    let multi = indicatif::MultiProgress::new();
    let mut handles = Vec::new();

    for batch_id in batch_ids {
        let client = client.clone();
        let batch_id = batch_id.clone();
        let multi = multi.clone();
        handles.push(tokio::spawn(async move {
            watch_single(
                &client,
                &batch_id,
                Some(&multi),
                poll_interval_secs,
                max_retries,
            )
            .await
        }));
    }

    let mut had_failure = false;
    for handle in handles {
        if let Err(e) = handle.await? {
            eprintln!("Error: {}", e);
            had_failure = true;
        }
    }

    if had_failure {
        anyhow::bail!("One or more batches failed");
    }
    Ok(())
}

/// Watch a single batch with a progress bar. If `multi` is provided, the bar
/// is added to the multi-progress group; otherwise it's standalone.
pub async fn watch_single(
    client: &DwClient,
    batch_id: &str,
    multi: Option<&indicatif::MultiProgress>,
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};

    let style = ProgressStyle::default_bar()
        .template("  {msg} [{bar:30.green/dim}] {pos}/{len} ({percent}%)")
        .unwrap()
        .progress_chars("█▓░");

    let bar = if let Some(mp) = multi {
        mp.add(ProgressBar::new(0))
    } else {
        ProgressBar::new(0)
    };
    bar.set_style(style);
    bar.set_message(format!("{} — waiting", batch_id));

    let mut consecutive_errors: u32 = 0;
    let max_retries = max_retries.min(10);

    loop {
        let batch = match client.get_batch_once(batch_id).await {
            Ok(b) => {
                consecutive_errors = 0;
                b
            }
            Err(e) if e.is_transient() => {
                consecutive_errors += 1;
                if consecutive_errors > max_retries {
                    bar.abandon_with_message(format!("{} — connection lost", batch_id));
                    anyhow::bail!(
                        "Lost connection to server after {} retries: {}",
                        max_retries,
                        e
                    );
                }
                // Honor server-provided retry_after for rate limits, else backoff
                let delay = if let dw_client::DwError::RateLimited {
                    retry_after: Some(secs),
                } = &e
                {
                    *secs
                } else {
                    2u64.saturating_pow(consecutive_errors).min(60)
                };
                bar.set_message(format!(
                    "{} — retrying ({}/{})",
                    batch_id, consecutive_errors, max_retries
                ));
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
            Err(e) => {
                // Non-transient error (auth, 4xx, etc.) — fail immediately
                bar.abandon_with_message(format!("{} — error", batch_id));
                return Err(e.into());
            }
        };

        if let Some(rc) = &batch.request_counts {
            let done = (rc.completed + rc.failed) as u64;
            let total = rc.total as u64;

            if bar.length().unwrap_or(0) != total {
                bar.set_length(total);
            }
            bar.set_position(done);

            let failed_str = if rc.failed > 0 {
                format!(", {} failed", rc.failed)
            } else {
                String::new()
            };
            bar.set_message(format!("{} — {}{}", batch_id, batch.status, failed_str));
        } else {
            bar.set_message(format!("{} — {}", batch_id, batch.status));
        }

        if batch.is_terminal() {
            if batch.status == "completed" {
                bar.finish_with_message(format!("{} — completed", batch_id));
                return Ok(());
            } else {
                bar.abandon_with_message(format!("{} — {}", batch_id, batch.status));
                anyhow::bail!("Batch {} ended with status: {}", batch_id, batch.status);
            }
        }

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
    }
}

/// Show analytics for one or more batches.
pub async fn analytics(
    client: &DwClient,
    ids: &[String],
    from_file: Option<&Path>,
    format: crate::output::OutputFormat,
) -> anyhow::Result<()> {
    let batch_ids = resolve_batch_ids(ids, from_file).await?;
    let multi = batch_ids.len() > 1;
    for (i, batch_id) in batch_ids.iter().enumerate() {
        if multi && format == crate::output::OutputFormat::Table && i > 0 {
            println!();
        }
        if multi && format == crate::output::OutputFormat::Json {
            // NDJSON: one compact JSON object per line for multi-batch output
            let a = client.get_batch_analytics(batch_id).await?;
            println!("{}", serde_json::to_string(&a)?);
        } else if multi && format == crate::output::OutputFormat::Plain {
            // Prefix with batch ID so multi-batch rows are identifiable
            let a = client.get_batch_analytics(batch_id).await?;
            println!(
                "{}\t{}\t{}\t{}\t{}",
                batch_id,
                a.total_requests,
                a.total_tokens,
                a.avg_duration_ms.unwrap_or(0.0),
                a.total_cost.as_deref().unwrap_or("0")
            );
        } else {
            crate::commands::usage::batch_analytics(client, batch_id, format).await?;
        }
    }
    Ok(())
}

/// Resolve batch IDs from positional args and/or a file (one ID per line).
async fn resolve_batch_ids(
    ids: &[String],
    from_file: Option<&Path>,
) -> anyhow::Result<Vec<String>> {
    let mut result: Vec<String> = ids.to_vec();
    if let Some(path) = from_file {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path.display(), e))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                result.push(trimmed.to_string());
            }
        }
    }
    if result.is_empty() {
        anyhow::bail!("No batch IDs provided. Pass IDs as arguments or use --from-file <FILE>");
    }
    Ok(result)
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
