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

use std::io::{BufRead, Write as IoWrite};

/// Show stats for a local JSONL file (streaming, bounded memory).
pub fn stats(path: &std::path::Path, format: OutputFormat) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    let mut line_count: usize = 0;
    let mut models: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut total_prompt_chars: usize = 0;
    let mut errors: usize = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        line_count += 1;
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(val) => {
                if let Some(model) = val
                    .get("body")
                    .and_then(|b| b.get("model"))
                    .and_then(|m| m.as_str())
                {
                    *models.entry(model.to_string()).or_insert(0) += 1;
                }
                if let Some(messages) = val
                    .get("body")
                    .and_then(|b| b.get("messages"))
                    .and_then(|m| m.as_array())
                {
                    for msg in messages {
                        if let Some(content) = msg.get("content") {
                            if let Some(s) = content.as_str() {
                                // String content
                                total_prompt_chars += s.len();
                            } else if let Some(parts) = content.as_array() {
                                // Multimodal array content — sum text parts only
                                for part in parts {
                                    if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                        total_prompt_chars += text.len();
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => errors += 1,
        }
    }

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

/// Extract a random sample using reservoir sampling (single pass, O(k) memory).
pub fn sample(
    path: &std::path::Path,
    count: usize,
    output: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    use rand::Rng;

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut rng = rand::rng();

    // Reservoir sampling: keep exactly `count` lines in memory
    if count == 0 {
        anyhow::bail!("Sample count must be at least 1.");
    }

    let mut reservoir: Vec<String> = Vec::with_capacity(count);
    let mut total: usize = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        if reservoir.len() < count {
            reservoir.push(line);
        } else {
            let j = rng.random_range(0..total);
            if j < count {
                reservoir[j] = line;
            }
        }
    }

    if reservoir.is_empty() {
        anyhow::bail!("File is empty: {}", path.display());
    }

    let mut writer: Box<dyn IoWrite> = if let Some(out_path) = output {
        Box::new(std::io::BufWriter::new(std::fs::File::create(out_path)?))
    } else {
        Box::new(std::io::BufWriter::new(std::io::stdout().lock()))
    };

    for line in &reservoir {
        writeln!(writer, "{}", line)?;
    }

    if let Some(out_path) = output {
        eprintln!(
            "Sampled {} of {} lines → {}",
            reservoir.len(),
            total,
            out_path.display()
        );
    }

    Ok(())
}

/// Merge multiple JSONL files into one (streaming).
pub fn merge(paths: &[std::path::PathBuf], output: Option<&std::path::Path>) -> anyhow::Result<()> {
    let mut writer: Box<dyn IoWrite> = if let Some(out_path) = output {
        Box::new(std::io::BufWriter::new(std::fs::File::create(out_path)?))
    } else {
        Box::new(std::io::BufWriter::new(std::io::stdout().lock()))
    };

    let mut total_lines = 0;
    for path in paths {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
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

/// Split a JSONL file into chunks (streaming, one chunk file at a time).
pub fn split(
    path: &std::path::Path,
    chunk_size: usize,
    output_dir: Option<&std::path::Path>,
) -> anyhow::Result<()> {
    if chunk_size == 0 {
        anyhow::bail!("Chunk size must be at least 1.");
    }

    let dir = output_dir.unwrap_or_else(|| {
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        if parent.as_os_str().is_empty() {
            std::path::Path::new(".")
        } else {
            parent
        }
    });
    std::fs::create_dir_all(dir)?;

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("chunk");

    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);

    let mut chunk_idx: usize = 0;
    let mut lines_in_chunk: usize = 0;
    let mut total_lines: usize = 0;
    let mut writer: Option<std::io::BufWriter<std::fs::File>> = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Start a new chunk file if needed
        if writer.is_none() || lines_in_chunk >= chunk_size {
            // Close previous chunk
            if let Some(ref mut w) = writer {
                w.flush()?;
                eprintln!(
                    "  {}-{:03}.jsonl ({} lines)",
                    stem, chunk_idx, lines_in_chunk
                );
            }
            chunk_idx += 1;
            lines_in_chunk = 0;
            let chunk_path = dir.join(format!("{}-{:03}.jsonl", stem, chunk_idx));
            writer = Some(std::io::BufWriter::new(std::fs::File::create(chunk_path)?));
        }

        writeln!(writer.as_mut().unwrap(), "{}", line)?;
        lines_in_chunk += 1;
        total_lines += 1;
    }

    // Flush final chunk
    if let Some(ref mut w) = writer {
        w.flush()?;
        eprintln!(
            "  {}-{:03}.jsonl ({} lines)",
            stem, chunk_idx, lines_in_chunk
        );
    }

    if total_lines == 0 {
        anyhow::bail!("File is empty: {}", path.display());
    }

    eprintln!(
        "Split {} lines into {} chunks of up to {}",
        total_lines, chunk_idx, chunk_size
    );

    Ok(())
}

/// Compare two JSONL result files by custom_id.
/// Both files are streamed line-by-line. A SHA-256 digest of each response's content
/// is stored per custom_id for memory-efficient comparison.
pub fn diff(a: &std::path::Path, b: &std::path::Path, format: OutputFormat) -> anyhow::Result<()> {
    use sha2::{Digest, Sha256};

    /// SHA-256 hash of the response content for reliable comparison.
    fn content_hash(val: &serde_json::Value) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(extract_content(val).as_bytes());
        hasher.finalize().into()
    }

    // Parse file into a map of custom_id → content SHA-256 hash.
    let parse_file =
        |path: &std::path::Path| -> anyhow::Result<std::collections::HashMap<String, [u8; 32]>> {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            let mut map = std::collections::HashMap::new();
            let mut parse_errors = 0usize;
            let mut missing_id = 0usize;
            let mut duplicates = 0usize;

            for line in reader.lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<serde_json::Value>(&line) {
                    Ok(val) => {
                        if let Some(id) = val.get("custom_id").and_then(|c| c.as_str()) {
                            if map.insert(id.to_string(), content_hash(&val)).is_some() {
                                duplicates += 1;
                            }
                        } else {
                            missing_id += 1;
                        }
                    }
                    Err(_) => parse_errors += 1,
                }
            }

            if parse_errors > 0 || missing_id > 0 || duplicates > 0 {
                eprintln!(
                    "Warning: {} ({} parse errors, {} missing custom_id, {} duplicate IDs)",
                    path.display(),
                    parse_errors,
                    missing_id,
                    duplicates
                );
            }
            Ok(map)
        };

    let map_a = parse_file(a)?;
    let map_b = parse_file(b)?;

    // Compute counts without allocating key vectors
    let mut only_a_count = 0usize;
    let mut common_count = 0usize;
    let mut same = 0usize;
    let mut different = 0usize;

    for (id, content_a) in &map_a {
        if let Some(content_b) = map_b.get(id) {
            common_count += 1;
            if content_a == content_b {
                same += 1;
            } else {
                different += 1;
            }
        } else {
            only_a_count += 1;
        }
    }

    let only_b_count = map_b.keys().filter(|k| !map_a.contains_key(*k)).count();

    match format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "file_a": a.display().to_string(),
                "file_b": b.display().to_string(),
                "entries_a": map_a.len(),
                "entries_b": map_b.len(),
                "common": common_count,
                "identical_responses": same,
                "different_responses": different,
                "only_in_a": only_a_count,
                "only_in_b": only_b_count,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Plain => {
            println!(
                "{}\t{}\t{}\t{}\t{}",
                common_count, same, different, only_a_count, only_b_count
            );
        }
        OutputFormat::Table => {
            println!("Comparing:");
            println!("  A: {} ({} entries)", a.display(), map_a.len());
            println!("  B: {} ({} entries)", b.display(), map_b.len());
            println!();
            println!("  Common IDs:          {}", common_count);
            println!("  Identical responses:  {}", same);
            println!("  Different responses:  {}", different);
            if only_a_count > 0 {
                println!("  Only in A:           {}", only_a_count);
            }
            if only_b_count > 0 {
                println!("  Only in B:           {}", only_b_count);
            }
        }
    }

    Ok(())
}

/// Extract response content as a canonical string for hashing.
/// String content is returned as-is; array/multimodal content is serialized to JSON.
fn extract_content(val: &serde_json::Value) -> String {
    let content = val
        .get("response")
        .and_then(|r| r.get("body"))
        .and_then(|b| b.get("choices"))
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"));

    match content {
        Some(c) if c.is_string() => c.as_str().unwrap_or("").to_string(),
        Some(c) => {
            // Multimodal or non-string content: use canonical JSON representation
            serde_json::to_string(c).unwrap_or_default()
        }
        None => String::new(),
    }
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
