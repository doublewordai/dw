use std::path::Path;

use dw_client::DwClient;
use dw_client::types::files::FileResponse;

use crate::cli::{FilePrepareArgs, FileUploadArgs};
use crate::jsonl;
use crate::output::{
    Displayable, OutputFormat, format_bytes, format_timestamp, print_item, print_list,
};

impl Displayable for FileResponse {
    fn table_headers() -> Vec<&'static str> {
        vec!["ID", "Filename", "Size", "Purpose", "Created"]
    }

    fn to_table_row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.filename.clone(),
            format_bytes(self.bytes),
            self.purpose.clone(),
            format_timestamp(self.created_at),
        ]
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn to_plain(&self) -> String {
        format!(
            "{}\t{}\t{}",
            self.id,
            self.filename,
            format_bytes(self.bytes)
        )
    }
}

pub async fn upload(
    client: &DwClient,
    args: &FileUploadArgs,
    format: OutputFormat,
) -> anyhow::Result<()> {
    if !args.path.exists() {
        anyhow::bail!("Path does not exist: {}", args.path.display());
    }

    let upload_path = if args.model.is_some()
        || args.temperature.is_some()
        || args.max_tokens.is_some()
        || args.encode_images
    {
        // Apply transforms to a temp file
        let transforms = jsonl::Transforms {
            model: args.model.clone(),
            temperature: args.temperature,
            max_tokens: args.max_tokens,
            ..Default::default()
        };
        let temp = jsonl::transform_to_temp(&args.path, &transforms).await?;
        Some(temp)
    } else {
        None
    };

    let path = upload_path.as_deref().unwrap_or(&args.path);

    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message(format!("Uploading {}...", args.path.display()));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let file = client.upload_file(path, "batch").await?;

    spinner.finish_with_message(format!(
        "Uploaded {} ({})",
        file.id,
        format_bytes(file.bytes)
    ));
    print_item(&file, format);
    Ok(())
}

pub async fn list(
    client: &DwClient,
    limit: i64,
    after: Option<&str>,
    all: bool,
    purpose: &str,
    format: OutputFormat,
) -> anyhow::Result<()> {
    // "all" purpose means no filter; otherwise filter by purpose
    let purpose_filter = if purpose == "all" {
        None
    } else {
        Some(purpose.to_string())
    };

    if all {
        // Auto-paginate: fetch all files
        let mut all_files = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let params = dw_client::types::files::ListFilesParams {
                limit: Some(100),
                after: cursor,
                purpose: purpose_filter.clone(),
            };
            let response = client.list_files(&params).await?;
            let has_more = response.has_more;
            let last_id = response.last_id.clone();
            all_files.extend(response.data);
            if !has_more {
                break;
            }
            cursor = last_id;
        }
        print_list(&all_files, format);
    } else {
        let params = dw_client::types::files::ListFilesParams {
            limit: Some(limit),
            after: after.map(|s| s.to_string()),
            purpose: purpose_filter,
        };
        let response = client.list_files(&params).await?;
        print_list(&response.data, format);
        if response.has_more
            && let Some(last_id) = &response.last_id
            && format != OutputFormat::Json
        {
            eprintln!(
                "More files available. Next page: dw files list --after {}",
                last_id
            );
        }
    }
    Ok(())
}

pub async fn get(client: &DwClient, file_id: &str, format: OutputFormat) -> anyhow::Result<()> {
    let file = client.get_file(file_id).await?;
    print_item(&file, format);
    Ok(())
}

pub async fn delete(client: &DwClient, file_id: &str, yes: bool) -> anyhow::Result<()> {
    if !yes {
        eprint!("Delete file {}? [y/N] ", file_id);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Cancelled.");
            return Ok(());
        }
    }
    client.delete_file(file_id).await?;
    eprintln!("Deleted file {}.", file_id);
    Ok(())
}

pub async fn content(
    client: &DwClient,
    file_id: &str,
    output_file: Option<&Path>,
) -> anyhow::Result<()> {
    let bytes = client.get_file_content(file_id).await?;

    if let Some(path) = output_file {
        tokio::fs::write(path, &bytes).await?;
        eprintln!("Written to {}", path.display());
    } else {
        use std::io::Write;
        std::io::stdout().write_all(&bytes)?;
    }
    Ok(())
}

pub async fn cost_estimate(
    client: &DwClient,
    file_id: &str,
    completion_window: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let estimate = client
        .get_file_cost_estimate(file_id, completion_window)
        .await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&estimate)?);
        }
        _ => {
            println!(
                "Estimated cost: ${} ({} requests, ~{} input tokens, ~{} output tokens)",
                estimate.total_estimated_cost,
                estimate.total_requests,
                estimate.total_estimated_input_tokens,
                estimate.total_estimated_output_tokens,
            );
            for breakdown in &estimate.models {
                println!(
                    "  {} — {} requests — ${}",
                    breakdown.model, breakdown.request_count, breakdown.estimated_cost
                );
            }
        }
    }
    Ok(())
}

pub fn validate(path: &Path) -> anyhow::Result<()> {
    let errors = jsonl::validate_file(path)?;

    if errors.is_empty() {
        eprintln!("Valid JSONL file: {}", path.display());
    } else {
        eprintln!("Validation errors in {}:", path.display());
        for error in &errors {
            eprintln!("  Line {}: {}", error.line, error.message);
        }
        std::process::exit(1);
    }
    Ok(())
}

pub async fn prepare(args: &FilePrepareArgs) -> anyhow::Result<()> {
    let transforms = jsonl::Transforms {
        model: args.model.clone(),
        temperature: args.temperature,
        max_tokens: args.max_tokens,
        top_p: args.top_p,
        set_fields: args.set_fields.clone(),
        add_lines: args.add_lines.clone(),
        remove_lines: args.remove_lines.clone(),
        encode_images: args.encode_images,
    };

    if args.path.is_dir() {
        if args.output_file.is_some() {
            anyhow::bail!(
                "Cannot use --output-file with a directory. Files are modified in place."
            );
        }
        let mut entries: Vec<_> = std::fs::read_dir(&args.path)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        entries.sort_by_key(|e| e.file_name());

        if entries.is_empty() {
            anyhow::bail!("No .jsonl files found in {}", args.path.display());
        }

        for entry in &entries {
            let path = entry.path();
            jsonl::transform_file(&path, &path, &transforms).await?;
            eprintln!("  Prepared: {}", path.display());
        }
    } else {
        let output_path = args.output_file.as_deref().unwrap_or(&args.path);
        jsonl::transform_file(&args.path, output_path, &transforms).await?;
        eprintln!("Prepared: {}", output_path.display());
    }
    Ok(())
}

// ===== Local JSONL manipulation commands =====

/// Show stats for a local JSONL file.
pub fn stats(path: &std::path::Path, format: OutputFormat) -> anyhow::Result<()> {
    let contents = std::fs::read_to_string(path)?;
    let mut line_count: usize = 0;
    let mut models: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_prompt_chars: usize = 0;
    let mut errors: usize = 0;

    for line in contents.lines() {
        if line.trim().is_empty() {
            continue;
        }
        line_count += 1;
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(val) => {
                // Extract model from body.model
                if let Some(model) = val
                    .get("body")
                    .and_then(|b| b.get("model"))
                    .and_then(|m| m.as_str())
                {
                    *models.entry(model.to_string()).or_insert(0) += 1;
                }
                // Rough token estimate from message content length
                if let Some(messages) = val
                    .get("body")
                    .and_then(|b| b.get("messages"))
                    .and_then(|m| m.as_array())
                {
                    for msg in messages {
                        if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                            total_prompt_chars += content.len();
                        }
                    }
                }
            }
            Err(_) => errors += 1,
        }
    }

    // Rough token estimate: ~4 chars per token
    let estimated_tokens = total_prompt_chars / 4;

    match format {
        OutputFormat::Json => {
            let stats = serde_json::json!({
                "lines": line_count,
                "models": models,
                "estimated_input_tokens": estimated_tokens,
                "parse_errors": errors,
            });
            println!("{}", serde_json::to_string_pretty(&stats)?);
        }
        OutputFormat::Plain => {
            println!("{}\t{}\t{}", line_count, estimated_tokens, errors);
        }
        OutputFormat::Table => {
            println!("File: {}", path.display());
            println!("  Requests:              {}", line_count);
            println!(
                "  Estimated input tokens: ~{}",
                format_number(estimated_tokens)
            );
            if errors > 0 {
                println!("  Parse errors:          {}", errors);
            }
            if !models.is_empty() {
                println!("  Models:");
                for (model, count) in &models {
                    println!("    {:<40} {}", model, count);
                }
            } else {
                println!("  Models:                (none set — use `dw files prepare --model`)");
            }
        }
    }

    Ok(())
}

/// Extract a random sample from a JSONL file.
pub fn sample(
    path: &std::path::Path,
    count: usize,
    output: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use rand::seq::SliceRandom;

    let contents = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.is_empty() {
        anyhow::bail!("File is empty: {}", path.display());
    }

    let sample_size = count.min(lines.len());
    let mut rng = rand::rng();
    let mut indices: Vec<usize> = (0..lines.len()).collect();
    indices.shuffle(&mut rng);
    indices.truncate(sample_size);
    indices.sort(); // preserve original order

    let sampled: Vec<&str> = indices.iter().map(|&i| lines[i]).collect();
    let result = sampled.join("\n") + "\n";

    if let Some(out_path) = output {
        std::fs::write(out_path, &result)?;
        eprintln!(
            "Sampled {} of {} lines → {}",
            sample_size,
            lines.len(),
            out_path.display()
        );
    } else {
        print!("{}", result);
    }

    Ok(())
}

/// Merge multiple JSONL files into one.
pub fn merge(paths: &[std::path::PathBuf], output: Option<&std::path::Path>) -> anyhow::Result<()> {
    use std::io::Write;

    let mut writer: Box<dyn Write> = if let Some(out_path) = output {
        Box::new(std::fs::File::create(out_path)?)
    } else {
        Box::new(std::io::stdout())
    };

    let mut total_lines = 0;
    for path in paths {
        let contents = std::fs::read_to_string(path)?;
        for line in contents.lines() {
            if !line.trim().is_empty() {
                writeln!(writer, "{}", line)?;
                total_lines += 1;
            }
        }
    }

    if let Some(out_path) = output {
        eprintln!(
            "Merged {} files ({} lines) → {}",
            paths.len(),
            total_lines,
            out_path.display()
        );
    }

    Ok(())
}

/// Split a JSONL file into chunks.
pub fn split(
    path: &std::path::Path,
    chunk_size: usize,
    output_dir: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    if chunk_size == 0 {
        anyhow::bail!("Chunk size must be at least 1.");
    }

    let contents = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = contents.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.is_empty() {
        anyhow::bail!("File is empty: {}", path.display());
    }

    let dir = output_dir.unwrap_or_else(|| path.parent().unwrap_or(std::path::Path::new(".")));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("chunk");

    let num_chunks = lines.len().div_ceil(chunk_size);

    for (i, chunk) in lines.chunks(chunk_size).enumerate() {
        let chunk_path = dir.join(format!("{}-{:03}.jsonl", stem, i + 1));
        let chunk_content = chunk.join("\n") + "\n";
        std::fs::write(&chunk_path, chunk_content)?;
        eprintln!("  {} ({} lines)", chunk_path.display(), chunk.len());
    }

    eprintln!(
        "Split {} lines into {} chunks of up to {}",
        lines.len(),
        num_chunks,
        chunk_size
    );

    Ok(())
}

/// Compare two JSONL result files by custom_id.
pub fn diff(a: &std::path::Path, b: &std::path::Path, format: OutputFormat) -> anyhow::Result<()> {
    let parse_file = |path: &std::path::Path| -> anyhow::Result<std::collections::HashMap<String, serde_json::Value>> {
        let contents = std::fs::read_to_string(path)?;
        let mut map = std::collections::HashMap::new();
        for line in contents.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(id) = val.get("custom_id").and_then(|c| c.as_str()) {
                    map.insert(id.to_string(), val);
                }
        }
        Ok(map)
    };

    let map_a = parse_file(a)?;
    let map_b = parse_file(b)?;

    let only_a: Vec<_> = map_a.keys().filter(|k| !map_b.contains_key(*k)).collect();
    let only_b: Vec<_> = map_b.keys().filter(|k| !map_a.contains_key(*k)).collect();
    let common: Vec<_> = map_a.keys().filter(|k| map_b.contains_key(*k)).collect();

    // For common entries, extract the response content and compare
    let mut different = 0;
    let mut same = 0;

    let extract_content = |val: &serde_json::Value| -> Option<String> {
        val.get("response")
            .and_then(|r| r.get("body"))
            .and_then(|b| b.get("choices"))
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
    };

    for id in &common {
        let content_a = extract_content(&map_a[*id]);
        let content_b = extract_content(&map_b[*id]);
        if content_a == content_b {
            same += 1;
        } else {
            different += 1;
        }
    }

    match format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "file_a": a.display().to_string(),
                "file_b": b.display().to_string(),
                "entries_a": map_a.len(),
                "entries_b": map_b.len(),
                "common": common.len(),
                "identical_responses": same,
                "different_responses": different,
                "only_in_a": only_a.len(),
                "only_in_b": only_b.len(),
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Plain => {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                common.len(),
                same,
                different,
                only_a.len(),
                only_b.len()
            );
        }
        OutputFormat::Table => {
            println!("Comparing:");
            println!("  A: {} ({} entries)", a.display(), map_a.len());
            println!("  B: {} ({} entries)", b.display(), map_b.len());
            println!();
            println!("  Common IDs:          {}", common.len());
            println!("  Identical responses:  {}", same);
            println!("  Different responses:  {}", different);
            if !only_a.is_empty() {
                println!("  Only in A:           {}", only_a.len());
            }
            if !only_b.is_empty() {
                println!("  Only in B:           {}", only_b.len());
            }
        }
    }

    Ok(())
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
