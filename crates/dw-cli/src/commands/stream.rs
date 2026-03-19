use std::io::Write;
use std::time::Duration;

use dw_client::DwClient;
use dw_client::types::batches::CreateBatchRequest;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::cli::StreamArgs;
use crate::jsonl;

/// Upload, create batches, and stream all results to stdout as they complete.
///
/// For multiple files: each batch starts streaming as soon as it's created —
/// uploads and streaming run concurrently (pipelined). Results from all
/// batches interleave into a single stdout output.
pub async fn run(client: &DwClient, args: &StreamArgs) -> anyhow::Result<()> {
    let paths = collect_jsonl_paths(&args.path)?;

    if paths.is_empty() {
        anyhow::bail!("No .jsonl files found at {}", args.path.display());
    }

    if paths.len() == 1 {
        // Single file: simple sequential flow
        let batch_id = upload_and_create(client, &paths[0], args, None).await?;
        return stream_single(client, &batch_id).await;
    }

    // Multiple files: pipeline uploads with concurrent streaming.
    let multi = MultiProgress::new();
    let stdout = std::sync::Arc::new(tokio::sync::Mutex::new(std::io::stdout()));
    let mut stream_handles: Vec<tokio::task::JoinHandle<anyhow::Result<()>>> = Vec::new();

    for path in &paths {
        let batch_id = upload_and_create(client, path, args, Some(&multi)).await?;

        // Spawn streaming immediately — runs while we upload the next file
        let client = client.clone();
        let mp = multi.clone();
        let stdout = stdout.clone();
        stream_handles.push(tokio::spawn(async move {
            stream_with_multi(&client, &batch_id, &mp, &stdout).await
        }));
    }

    // Wait for all streams to complete
    let mut had_failure = false;
    for handle in stream_handles {
        if let Err(e) = handle.await? {
            eprintln!("Error: {}", e);
            had_failure = true;
        }
    }
    stdout.lock().await.flush()?;

    if had_failure {
        anyhow::bail!("One or more batches failed");
    }
    Ok(())
}

/// Upload a file and create a batch, returning the batch ID.
async fn upload_and_create(
    client: &DwClient,
    path: &std::path::Path,
    args: &StreamArgs,
    multi: Option<&MultiProgress>,
) -> anyhow::Result<String> {
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

    let spinner = if let Some(mp) = multi {
        mp.add(ProgressBar::new_spinner())
    } else {
        ProgressBar::new_spinner()
    };
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Uploading {}...", path.display()));
    spinner.enable_steady_tick(Duration::from_millis(100));
    let file = client.upload_file(actual_path, "batch").await?;
    spinner.finish_with_message(format!("Uploaded {} ({})", file.id, file.filename));

    let request = CreateBatchRequest {
        input_file_id: file.id.clone(),
        endpoint: "/v1/chat/completions".to_string(),
        completion_window: args.completion_window.clone(),
        metadata: None,
    };
    let spinner = if let Some(mp) = multi {
        mp.add(ProgressBar::new_spinner())
    } else {
        ProgressBar::new_spinner()
    };
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Creating batch...");
    spinner.enable_steady_tick(Duration::from_millis(100));
    let batch = client.create_batch(&request).await?;
    spinner.finish_with_message(format!("Created batch: {}", batch.id));

    Ok(batch.id)
}

/// Stream results from a single batch with a standalone progress bar.
async fn stream_single(client: &DwClient, batch_id: &str) -> anyhow::Result<()> {
    let bar = ProgressBar::new(0);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("  {msg} [{bar:30.green/dim}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("█▓░"),
    );
    bar.set_message(format!("{} — streaming", batch_id));

    stream_loop(
        client,
        batch_id,
        &bar,
        &tokio::sync::Mutex::new(std::io::stdout()),
    )
    .await
}

/// Stream results from a batch within a MultiProgress group, writing to shared stdout.
async fn stream_with_multi(
    client: &DwClient,
    batch_id: &str,
    multi: &MultiProgress,
    stdout: &tokio::sync::Mutex<std::io::Stdout>,
) -> anyhow::Result<()> {
    let bar = multi.add(ProgressBar::new(0));
    bar.set_style(
        ProgressStyle::default_bar()
            .template("  {msg} [{bar:30.green/dim}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("█▓░"),
    );
    bar.set_message(format!("{} — streaming", batch_id));

    stream_loop(client, batch_id, &bar, stdout).await
}

/// Core streaming loop: poll for completed results and write to stdout.
/// Retries transient network errors up to 3 times with exponential backoff.
async fn stream_loop(
    client: &DwClient,
    batch_id: &str,
    bar: &ProgressBar,
    stdout: &tokio::sync::Mutex<std::io::Stdout>,
) -> anyhow::Result<()> {
    let mut cursor: usize = 0;
    let page_size: usize = 100;
    let mut consecutive_errors: u32 = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        // Fetch results page — retry on transient errors
        let page = match client
            .get_batch_results_page(batch_id, cursor, page_size, Some("completed"))
            .await
        {
            Ok(p) => p,
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors > MAX_RETRIES {
                    bar.abandon_with_message(format!("{} — connection lost", batch_id));
                    anyhow::bail!("Lost connection after {} retries: {}", MAX_RETRIES, e);
                }
                let delay = 2u64.pow(consecutive_errors);
                bar.set_message(format!(
                    "{} — retrying ({}/{})",
                    batch_id, consecutive_errors, MAX_RETRIES
                ));
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
        };

        if !page.body.is_empty() {
            let mut out = stdout.lock().await;
            out.write_all(page.body.as_bytes())?;
            out.flush()?;
            drop(out);
            cursor = page.last_line;
        }

        // Fetch batch status — retry on transient errors
        let batch = match client.get_batch(batch_id).await {
            Ok(b) => {
                consecutive_errors = 0;
                b
            }
            Err(e) => {
                consecutive_errors += 1;
                if consecutive_errors > MAX_RETRIES {
                    bar.abandon_with_message(format!("{} — connection lost", batch_id));
                    anyhow::bail!("Lost connection after {} retries: {}", MAX_RETRIES, e);
                }
                let delay = 2u64.pow(consecutive_errors);
                bar.set_message(format!(
                    "{} — retrying ({}/{})",
                    batch_id, consecutive_errors, MAX_RETRIES
                ));
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
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
        }

        if batch.is_terminal() {
            // Drain remaining results
            loop {
                let final_page = client
                    .get_batch_results_page(batch_id, cursor, page_size, Some("completed"))
                    .await?;
                if !final_page.body.is_empty() {
                    let mut out = stdout.lock().await;
                    out.write_all(final_page.body.as_bytes())?;
                    drop(out);
                    cursor = final_page.last_line;
                }
                if !final_page.incomplete {
                    break;
                }
            }
            stdout.lock().await.flush()?;

            if batch.status == "completed" {
                bar.finish_with_message(format!("{} — completed", batch_id));
            } else {
                bar.abandon_with_message(format!("{} — {}", batch_id, batch.status));
            }
            return Ok(());
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

fn collect_jsonl_paths(path: &std::path::Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
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
