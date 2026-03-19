use std::time::Duration;

use dw_client::DwClient;
use dw_client::types::batches::CreateBatchRequest;

use crate::cli::StreamArgs;
use crate::jsonl;

/// Upload, create batch, and stream results to stdout as they complete.
///
/// Progress goes to stderr, results to stdout. Results are streamed
/// incrementally — they appear as soon as individual requests complete,
/// not after the whole batch finishes.
pub async fn run(client: &DwClient, args: &StreamArgs) -> anyhow::Result<()> {
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

        // Upload with spinner
        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message(format!("Uploading {}...", path.display()));
        spinner.enable_steady_tick(Duration::from_millis(100));
        let file = client.upload_file(actual_path, "batch").await?;
        spinner.finish_with_message(format!("Uploaded {} ({})", file.id, file.filename));

        // Create batch with spinner
        let request = CreateBatchRequest {
            input_file_id: file.id.clone(),
            endpoint: "/v1/chat/completions".to_string(),
            completion_window: args.completion_window.clone(),
            metadata: None,
        };

        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_style(
            indicatif::ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        spinner.set_message("Creating batch...");
        spinner.enable_steady_tick(Duration::from_millis(100));
        let batch = client.create_batch(&request).await?;
        spinner.finish_with_message(format!("Created batch: {}", batch.id));

        // Stream results incrementally as they complete
        stream_results(client, &batch.id).await?;
    }

    Ok(())
}

/// Poll for completed results and stream them to stdout as they arrive.
/// Shows a progress bar on stderr while waiting.
async fn stream_results(client: &DwClient, batch_id: &str) -> anyhow::Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::Write;

    let bar = ProgressBar::new(0);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("  {msg} [{bar:30.green/dim}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("█▓░"),
    );
    bar.set_message(format!("{} — streaming", batch_id));

    let mut cursor: usize = 0;
    let page_size: usize = 100;

    loop {
        // Fetch the next page of completed results
        let page = client
            .get_batch_results_page(batch_id, cursor, page_size, Some("completed"))
            .await?;

        // Write any new results to stdout immediately
        if !page.body.is_empty() {
            std::io::stdout().write_all(page.body.as_bytes())?;
            std::io::stdout().flush()?;
            cursor = page.last_line;
        }

        // Update progress bar from batch status
        let batch = client.get_batch(batch_id).await?;
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
            // Fetch any remaining results we haven't consumed yet
            loop {
                let final_page = client
                    .get_batch_results_page(batch_id, cursor, page_size, Some("completed"))
                    .await?;
                if !final_page.body.is_empty() {
                    std::io::stdout().write_all(final_page.body.as_bytes())?;
                    cursor = final_page.last_line;
                }
                if !final_page.incomplete {
                    break;
                }
            }
            std::io::stdout().flush()?;

            if batch.status == "completed" {
                bar.finish_with_message(format!("{} — completed", batch_id));
            } else {
                bar.abandon_with_message(format!("{} — {}", batch_id, batch.status));
            }
            return Ok(());
        }

        // Poll interval — don't hammer the API
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
