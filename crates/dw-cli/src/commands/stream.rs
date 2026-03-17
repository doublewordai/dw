use dw_client::DwClient;
use dw_client::types::batches::CreateBatchRequest;

use crate::cli::StreamArgs;
use crate::commands::batches;
use crate::jsonl;

/// Upload, create batch, watch progress (stderr), and stream results to stdout.
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

        // Watch until completion
        batches::watch_batch(client, &batch.id).await?;

        // Stream results to stdout
        let results = client.get_batch_results(&batch.id).await?;
        use std::io::Write;
        std::io::stdout().write_all(&results)?;
    }

    Ok(())
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
