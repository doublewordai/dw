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
pub async fn run(
    client: &DwClient,
    args: &StreamArgs,
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
    let paths = collect_jsonl_paths(&args.path)?;

    if paths.is_empty() {
        anyhow::bail!("No .jsonl files found at {}", args.path.display());
    }

    if paths.len() == 1 {
        // Single file: simple sequential flow
        let batch_id = upload_and_create(client, &paths[0], args, None).await?;
        return stream_single(client, &batch_id, poll_interval_secs, max_retries).await;
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
            stream_with_multi(
                &client,
                &batch_id,
                &mp,
                &stdout,
                poll_interval_secs,
                max_retries,
            )
            .await
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
async fn stream_single(
    client: &DwClient,
    batch_id: &str,
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
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
        poll_interval_secs,
        max_retries,
    )
    .await
}

/// Stream results from a batch within a MultiProgress group, writing to shared stdout.
async fn stream_with_multi(
    client: &DwClient,
    batch_id: &str,
    multi: &MultiProgress,
    stdout: &tokio::sync::Mutex<std::io::Stdout>,
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
    let bar = multi.add(ProgressBar::new(0));
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
        stdout,
        poll_interval_secs,
        max_retries,
    )
    .await
}

/// Core streaming loop: poll batch status and stream results from the output file.
///
/// Uses the file content endpoint with byte offsets (like autobatcher) — this gives
/// stable pagination without duplicates, unlike skip-based pagination on a growing
/// result set.
async fn stream_loop(
    client: &DwClient,
    batch_id: &str,
    bar: &ProgressBar,
    stdout: &tokio::sync::Mutex<std::io::Stdout>,
    poll_interval_secs: u64,
    max_retries: u32,
) -> anyhow::Result<()> {
    let mut offset: usize = 0;
    let mut consecutive_errors: u32 = 0;
    let max_retries = max_retries.min(10);

    loop {
        // 1. Get batch status
        let batch = match client.get_batch_once(batch_id).await {
            Ok(b) => {
                consecutive_errors = 0;
                b
            }
            Err(e) if e.is_transient() => {
                consecutive_errors += 1;
                if consecutive_errors > max_retries {
                    bar.abandon_with_message(format!("{} — connection lost", batch_id));
                    anyhow::bail!("Lost connection after {} retries: {}", max_retries, e);
                }
                let delay = transient_delay(&e, consecutive_errors);
                bar.set_message(format!(
                    "{} — retrying ({}/{})",
                    batch_id, consecutive_errors, max_retries
                ));
                tokio::time::sleep(Duration::from_secs(delay)).await;
                continue;
            }
            Err(e) => {
                bar.abandon_with_message(format!("{} — error", batch_id));
                return Err(e.into());
            }
        };

        // Update progress bar
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

        // 2. Stream results from the output file (if available)
        if let Some(ref output_file_id) = batch.output_file_id {
            match client.get_file_content_stream(output_file_id, offset).await {
                Ok(dw_client::types::files::FileContentChunk::Data {
                    body, next_offset, ..
                }) => {
                    consecutive_errors = 0;
                    if !body.is_empty() {
                        let mut out = stdout.lock().await;
                        out.write_all(body.as_bytes())?;
                        out.flush()?;
                        drop(out);
                        offset = next_offset;
                    }
                }
                Ok(dw_client::types::files::FileContentChunk::NotReady) => {
                    // Output file not created yet — normal during early polling
                    consecutive_errors = 0;
                }
                Err(e) if e.is_transient() => {
                    consecutive_errors += 1;
                    if consecutive_errors > max_retries {
                        bar.abandon_with_message(format!("{} — connection lost", batch_id));
                        anyhow::bail!("Lost connection after {} retries: {}", max_retries, e);
                    }
                    let delay = transient_delay(&e, consecutive_errors);
                    bar.set_message(format!(
                        "{} — retrying ({}/{})",
                        batch_id, consecutive_errors, max_retries
                    ));
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    continue;
                }
                Err(e) => {
                    bar.abandon_with_message(format!("{} — error", batch_id));
                    return Err(e.into());
                }
            }
        }

        // 3. Check if batch is done
        if batch.is_terminal() {
            // Final drain — fetch any remaining content from the output file
            if let Some(ref output_file_id) = batch.output_file_id {
                let mut drain_errors: u32 = 0;
                loop {
                    match client.get_file_content_stream(output_file_id, offset).await {
                        Ok(dw_client::types::files::FileContentChunk::Data {
                            body,
                            next_offset,
                            incomplete,
                        }) => {
                            drain_errors = 0;
                            if !body.is_empty() {
                                let mut out = stdout.lock().await;
                                out.write_all(body.as_bytes())?;
                                drop(out);
                                offset = next_offset;
                            }
                            if !incomplete {
                                break;
                            }
                            // Incomplete but no new data — brief pause to avoid tight loop
                            if body.is_empty() {
                                tokio::time::sleep(Duration::from_millis(500)).await;
                            }
                        }
                        Ok(dw_client::types::files::FileContentChunk::NotReady) => {
                            // Batch is terminal but file doesn't exist — no results to drain
                            break;
                        }
                        Err(e) if e.is_transient() => {
                            drain_errors += 1;
                            if drain_errors > max_retries {
                                bar.abandon_with_message(format!("{} — drain failed", batch_id));
                                anyhow::bail!(
                                    "Failed to drain results after {} retries: {}",
                                    max_retries,
                                    e
                                );
                            }
                            let delay = transient_delay(&e, drain_errors);
                            tokio::time::sleep(Duration::from_secs(delay)).await;
                            continue;
                        }
                        Err(e) => {
                            bar.abandon_with_message(format!("{} — error", batch_id));
                            return Err(e.into());
                        }
                    }
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

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
    }
}

/// Compute retry delay: use server's retry_after for rate limits, else exponential backoff.
fn transient_delay(e: &dw_client::DwError, attempt: u32) -> u64 {
    if let dw_client::DwError::RateLimited {
        retry_after: Some(secs),
    } = e
    {
        *secs
    } else {
        2u64.saturating_pow(attempt).min(60)
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
