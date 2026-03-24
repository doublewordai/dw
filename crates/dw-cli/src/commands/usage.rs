use crate::cli::UsageArgs;
use crate::output::{OutputFormat, truncate_timestamp};
use dw_client::DwClient;

/// Show usage summary.
pub async fn run(client: &DwClient, args: &UsageArgs, format: OutputFormat) -> anyhow::Result<()> {
    let usage = client
        .get_usage(args.since.as_deref(), args.until.as_deref())
        .await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&usage)?);
        }
        OutputFormat::Plain => {
            println!(
                "{}\t{}\t{}\t{}",
                usage.total_request_count,
                usage.total_input_tokens,
                usage.total_output_tokens,
                usage.total_cost
            );
        }
        OutputFormat::Table => {
            println!("Usage Summary");
            println!("─────────────────────────────────────────");
            println!("  Requests:       {}", usage.total_request_count);
            println!(
                "  Batches:        {} (avg {:.1} requests/batch)",
                usage.total_batch_count, usage.avg_requests_per_batch
            );
            println!(
                "  Input tokens:   {}",
                format_tokens(usage.total_input_tokens)
            );
            println!(
                "  Output tokens:  {}",
                format_tokens(usage.total_output_tokens)
            );
            println!("  Cost:           ${}", usage.total_cost);
            println!("  Realtime est:   ${}", usage.estimated_realtime_cost);

            if !usage.by_model.is_empty() {
                println!();
                println!("By Model");
                println!("─────────────────────────────────────────");

                let mut table = comfy_table::Table::new();
                table.set_header(vec![
                    "Model",
                    "Requests",
                    "Input Tokens",
                    "Output Tokens",
                    "Cost",
                ]);

                for entry in &usage.by_model {
                    table.add_row(vec![
                        entry.model.clone(),
                        entry.request_count.to_string(),
                        format_tokens(entry.input_tokens),
                        format_tokens(entry.output_tokens),
                        format!("${}", entry.cost),
                    ]);
                }

                println!("{}", table);
            }
        }
    }

    Ok(())
}

/// Show batch analytics.
pub async fn batch_analytics(
    client: &DwClient,
    batch_id: &str,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let analytics = client.get_batch_analytics(batch_id).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&analytics)?);
        }
        OutputFormat::Plain => {
            println!(
                "{}\t{}\t{}\t{}",
                analytics.total_requests,
                analytics.total_tokens,
                analytics.avg_duration_ms.unwrap_or(0.0),
                analytics.total_cost.as_deref().unwrap_or("0")
            );
        }
        OutputFormat::Table => {
            println!("Batch Analytics: {}", batch_id);
            println!("─────────────────────────────────────────");
            println!("  Requests:          {}", analytics.total_requests);
            println!(
                "  Prompt tokens:     {}",
                format_tokens(analytics.total_prompt_tokens)
            );
            println!(
                "  Completion tokens: {}",
                format_tokens(analytics.total_completion_tokens)
            );
            println!(
                "  Total tokens:      {}",
                format_tokens(analytics.total_tokens)
            );
            if let Some(ms) = analytics.avg_duration_ms {
                println!("  Avg latency:       {:.0}ms", ms);
            }
            if let Some(ms) = analytics.avg_ttfb_ms {
                println!("  Avg TTFB:          {:.0}ms", ms);
            }
            if let Some(ref cost) = analytics.total_cost {
                println!("  Cost:              ${}", cost);
            }
        }
    }

    Ok(())
}

/// List recent requests.
pub async fn list_requests(
    client: &DwClient,
    args: &crate::cli::RequestsArgs,
    format: OutputFormat,
) -> anyhow::Result<()> {
    use dw_client::types::usage::ListRequestsParams;

    let params = ListRequestsParams {
        limit: args.limit,
        skip: args.skip,
        model: args.model.clone(),
        since: args.since.clone(),
        until: args.until.clone(),
        batch_id: args.batch_id.clone(),
        status_code: args.status,
    };

    let response = client.list_requests(&params).await?;

    match format {
        OutputFormat::Json => {
            for entry in &response.entries {
                println!("{}", serde_json::to_string(entry)?);
            }
        }
        OutputFormat::Plain => {
            for entry in &response.entries {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    entry.timestamp,
                    entry.model.as_deref().unwrap_or("-"),
                    entry
                        .status_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    entry
                        .total_tokens
                        .map(format_tokens)
                        .unwrap_or_else(|| "-".to_string()),
                    entry
                        .duration_ms
                        .map(|d| format!("{}ms", d))
                        .unwrap_or_else(|| "-".to_string()),
                );
            }
        }
        OutputFormat::Table => {
            if response.entries.is_empty() {
                eprintln!("No requests found.");
                return Ok(());
            }

            let mut table = comfy_table::Table::new();
            table.set_header(vec![
                "Timestamp",
                "Model",
                "Status",
                "Tokens",
                "Latency",
                "Batch",
            ]);

            for entry in &response.entries {
                table.add_row(vec![
                    truncate_timestamp(&entry.timestamp),
                    entry.model.as_deref().unwrap_or("-").to_string(),
                    entry
                        .status_code
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    entry
                        .total_tokens
                        .map(format_tokens)
                        .unwrap_or_else(|| "-".to_string()),
                    entry
                        .duration_ms
                        .map(|d| format!("{}ms", d))
                        .unwrap_or_else(|| "-".to_string()),
                    entry
                        .fusillade_batch_id
                        .as_deref()
                        .map(|id| id.get(..8).unwrap_or(id).to_string())
                        .unwrap_or_else(|| "-".to_string()),
                ]);
            }

            println!("{}", table);

            if response.entries.len() as u64 == params.limit {
                eprintln!(
                    "\nMore results available. Next page: dw requests --skip {}",
                    params.skip + params.limit
                );
            }
        }
    }

    Ok(())
}

fn format_tokens(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
