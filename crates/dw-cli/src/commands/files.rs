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

    eprintln!("Uploading {}...", args.path.display());
    let file = client.upload_file(path, "batch").await?;
    eprintln!("Uploaded: {}", file.id);
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

    let output_path = args.output_file.as_deref().unwrap_or(&args.path);
    jsonl::transform_file(&args.path, output_path, &transforms).await?;

    eprintln!("Prepared: {}", output_path.display());
    Ok(())
}
