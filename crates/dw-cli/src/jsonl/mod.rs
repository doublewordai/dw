use serde_json::Value;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// JSONL validation error.
pub struct ValidationError {
    pub line: usize,
    pub message: String,
}

/// Transforms to apply to a JSONL file.
#[derive(Debug, Default)]
pub struct Transforms {
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
    pub top_p: Option<f64>,
    pub set_fields: Vec<String>,
    pub add_lines: Vec<String>,
    pub remove_lines: Option<String>,
    pub encode_images: bool,
}

/// Validate a JSONL file, returning any errors found.
pub fn validate_file(path: &Path) -> anyhow::Result<Vec<ValidationError>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut errors = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        let line_num = i + 1;
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                errors.push(ValidationError {
                    line: line_num,
                    message: format!("Could not read line: {}", e),
                });
                continue;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let obj: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                errors.push(ValidationError {
                    line: line_num,
                    message: format!("Invalid JSON: {}", e),
                });
                continue;
            }
        };

        // Check required fields
        if obj.get("custom_id").is_none() {
            errors.push(ValidationError {
                line: line_num,
                message: "Missing required field: custom_id".to_string(),
            });
        }
        if obj.get("method").is_none() {
            errors.push(ValidationError {
                line: line_num,
                message: "Missing required field: method".to_string(),
            });
        }
        if obj.get("url").is_none() {
            errors.push(ValidationError {
                line: line_num,
                message: "Missing required field: url".to_string(),
            });
        }
        if obj.get("body").is_none() {
            errors.push(ValidationError {
                line: line_num,
                message: "Missing required field: body".to_string(),
            });
        } else if let Some(body) = obj.get("body")
            && body.get("model").is_none()
        {
            errors.push(ValidationError {
                line: line_num,
                message: "Missing required field: body.model".to_string(),
            });
        }
    }

    Ok(errors)
}

/// Transform a JSONL file and write to output path.
pub async fn transform_file(
    input: &Path,
    output: &Path,
    transforms: &Transforms,
) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(input)?;
    let transformed = apply_transforms(&content, transforms).await?;

    // If writing in-place, create a backup first
    if input == output {
        let backup = input.with_extension("jsonl.bak");
        std::fs::copy(input, &backup)?;
    }

    std::fs::write(output, transformed)?;
    Ok(())
}

/// Transform a JSONL file to a temporary file, returning the temp path.
pub async fn transform_to_temp(input: &Path, transforms: &Transforms) -> anyhow::Result<PathBuf> {
    let content = std::fs::read_to_string(input)?;
    let transformed = apply_transforms(&content, transforms).await?;

    let temp_dir = std::env::temp_dir();
    let file_name = input.file_name().unwrap_or_default().to_string_lossy();
    let temp_path = temp_dir.join(format!("dw-{}", file_name));
    std::fs::write(&temp_path, transformed)?;
    Ok(temp_path)
}

async fn apply_transforms(content: &str, transforms: &Transforms) -> anyhow::Result<String> {
    let mut output = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut obj: Value = serde_json::from_str(trimmed)?;

        // Check remove_lines filter
        if let Some(ref pattern) = transforms.remove_lines
            && let Some(custom_id) = obj.get("custom_id").and_then(|v| v.as_str())
            && custom_id.contains(pattern)
        {
            continue;
        }

        // Apply model override
        if let Some(ref model) = transforms.model
            && let Some(body) = obj.get_mut("body")
        {
            body["model"] = Value::String(model.clone());
        }

        // Apply temperature
        if let Some(temp) = transforms.temperature
            && let Some(body) = obj.get_mut("body")
        {
            body["temperature"] = serde_json::json!(temp);
        }

        // Apply max_tokens
        if let Some(max) = transforms.max_tokens
            && let Some(body) = obj.get_mut("body")
        {
            body["max_tokens"] = serde_json::json!(max);
        }

        // Apply top_p
        if let Some(top_p) = transforms.top_p
            && let Some(body) = obj.get_mut("body")
        {
            body["top_p"] = serde_json::json!(top_p);
        }

        // Apply arbitrary set fields
        for field_spec in &transforms.set_fields {
            if let Some((key_path, value_str)) = field_spec.split_once('=') {
                let value = parse_json_value(value_str);
                set_nested_field(&mut obj, key_path, value);
            }
        }

        // Apply image encoding
        if transforms.encode_images {
            encode_images_in_entry(&mut obj).await?;
        }

        writeln!(output, "{}", serde_json::to_string(&obj)?)?;
    }

    // Add new lines
    for add_line in &transforms.add_lines {
        // Validate it's valid JSON
        let _: Value = serde_json::from_str(add_line)
            .map_err(|e| anyhow::anyhow!("Invalid JSON in --add-line: {}", e))?;
        writeln!(output, "{}", add_line.trim())?;
    }

    Ok(String::from_utf8(output)?)
}

/// Set a nested field using dot-notation (e.g., "body.stream" = false).
fn set_nested_field(obj: &mut Value, path: &str, value: Value) {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = obj;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part: set the value
            if let Some(map) = current.as_object_mut() {
                map.insert(part.to_string(), value.clone());
            }
        } else {
            // Intermediate part: navigate or create
            if !current.get(*part).is_some_and(|v| v.is_object()) {
                current[*part] = serde_json::json!({});
            }
            current = &mut current[*part];
        }
    }
}

/// Parse a string value into a JSON value (bool, number, or string).
fn parse_json_value(s: &str) -> Value {
    if s == "true" {
        Value::Bool(true)
    } else if s == "false" {
        Value::Bool(false)
    } else if s == "null" {
        Value::Null
    } else if let Ok(n) = s.parse::<i64>() {
        Value::Number(n.into())
    } else if let Ok(n) = s.parse::<f64>() {
        serde_json::json!(n)
    } else {
        // Try as JSON first, fallback to string
        serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.to_string()))
    }
}

/// Encode local image references in a JSONL entry to base64 data URIs.
async fn encode_images_in_entry(obj: &mut Value) -> anyhow::Result<()> {
    if let Some(body) = obj.get_mut("body")
        && let Some(messages) = body.get_mut("messages")
        && let Some(messages_arr) = messages.as_array_mut()
    {
        for message in messages_arr {
            if let Some(content) = message.get_mut("content")
                && let Some(content_arr) = content.as_array_mut()
            {
                for part in content_arr {
                    if part.get("type").and_then(|t| t.as_str()) == Some("image_url")
                        && let Some(image_url) = part.get_mut("image_url")
                        && let Some(url) = image_url.get("url").and_then(|u| u.as_str())
                        && let Some(data_uri) = encode_image_to_data_uri(url).await?
                    {
                        image_url["url"] = Value::String(data_uri);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Encode an image (local path or URL) to a base64 data URI.
/// Returns None if the URL is already a data URI or a remote URL we shouldn't download.
async fn encode_image_to_data_uri(url: &str) -> anyhow::Result<Option<String>> {
    use base64::Engine;

    if url.starts_with("data:") {
        // Already a data URI
        return Ok(None);
    }

    let (bytes, mime_type) = if url.starts_with("http://") || url.starts_with("https://") {
        // Download from URL
        let response = reqwest::get(url).await?;
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let bytes = response.bytes().await?;
        (bytes.to_vec(), content_type)
    } else {
        // Local file path
        let path = if let Some(stripped) = url.strip_prefix("file://") {
            Path::new(stripped)
        } else {
            Path::new(url)
        };

        let bytes = std::fs::read(path)
            .map_err(|e| anyhow::anyhow!("Could not read image '{}': {}", url, e))?;

        let mime = match path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("svg") => "image/svg+xml",
            _ => "image/png",
        };
        (bytes, mime.to_string())
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Some(format!("data:{};base64,{}", mime_type, encoded)))
}
