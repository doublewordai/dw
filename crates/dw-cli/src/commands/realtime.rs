use dw_client::DwClient;
use std::io::{IsTerminal, Read, Write};
use std::path::Path;

use crate::cli::RealtimeArgs;

/// Send a one-shot chat completion request, streaming tokens to stdout.
pub async fn run(client: &DwClient, args: &RealtimeArgs) -> anyhow::Result<()> {
    // Determine prompt: from arg, or from stdin if piped
    let prompt = if let Some(ref p) = args.prompt {
        p.clone()
    } else if !std::io::stdin().is_terminal() {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        input
    } else {
        anyhow::bail!("No prompt provided. Usage: dw realtime <model> \"<prompt>\"");
    };

    // Build messages
    let mut messages = Vec::new();
    if let Some(ref system) = args.system {
        messages.push(serde_json::json!({"role": "system", "content": system}));
    }
    messages.push(serde_json::json!({"role": "user", "content": prompt}));

    let mut body = serde_json::json!({
        "model": args.model,
        "messages": messages,
        "stream": !args.no_stream,
    });

    if let Some(max_tokens) = args.max_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }
    if let Some(temperature) = args.temperature {
        body["temperature"] = serde_json::json!(temperature);
    }

    let request = client
        .post(dw_client::ApiSurface::Ai, "/v1/chat/completions")?
        .json(&body);

    if args.no_stream {
        // Non-streaming: wait for full response
        let response: serde_json::Value = client.send(request).await?;

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("");

        write_output(content, args.output_file.as_deref())?;

        if args.usage
            && let Some(usage) = response.get("usage")
        {
            eprintln!(
                "\nTokens: {} input, {} output",
                usage["prompt_tokens"], usage["completion_tokens"]
            );
        }
    } else {
        // Streaming: process SSE events incrementally as bytes arrive
        use futures_util::StreamExt;

        let response = request.send().await?;

        if !response.status().is_success() {
            let err = dw_client::DwError::from_response(response).await;
            return Err(err.into());
        }

        let mut writer: Box<dyn Write> = if let Some(ref path) = args.output_file {
            Box::new(std::fs::File::create(path)?)
        } else {
            Box::new(std::io::stdout())
        };

        let mut total_content = String::new();
        let mut line_buf = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);

            // SSE data comes as "data: {...}\n\n" — may be split across chunks
            line_buf.push_str(&text);

            // Process all complete lines in the buffer
            while let Some(newline_pos) = line_buf.find('\n') {
                let line = line_buf[..newline_pos].trim().to_string();
                line_buf = line_buf[newline_pos + 1..].to_string();

                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ")
                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data)
                {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        write!(writer, "{}", content)?;
                        writer.flush()?;
                        total_content.push_str(content);
                    }

                    // Check for usage in final chunk
                    if args.usage
                        && let Some(usage) = parsed.get("usage")
                        && usage["prompt_tokens"].is_number()
                    {
                        eprintln!(
                            "\nTokens: {} input, {} output",
                            usage["prompt_tokens"], usage["completion_tokens"]
                        );
                    }
                }
            }
        }

        // Ensure newline at end
        if !total_content.ends_with('\n') {
            writeln!(writer)?;
        }
    }

    Ok(())
}

fn write_output(content: &str, output_file: Option<&Path>) -> anyhow::Result<()> {
    if let Some(path) = output_file {
        std::fs::write(path, content)?;
        eprintln!("Written to {}", path.display());
    } else {
        print!("{}", content);
        if !content.ends_with('\n') {
            println!();
        }
    }
    Ok(())
}
