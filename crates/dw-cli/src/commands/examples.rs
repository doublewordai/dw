use serde::Deserialize;
use std::path::Path;

const EXAMPLES_REPO: &str = "doublewordai/use-cases";
const GITHUB_API_BASE: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
struct ExampleManifest {
    name: String,
    description: String,
    #[serde(default)]
    cost: Option<String>,
}

/// Hardcoded manifest for offline use. This will be replaced by a fetched manifest.
fn builtin_manifest() -> Vec<ExampleManifest> {
    vec![
        ExampleManifest {
            name: "async-agents".into(),
            description: "Deep research with recursive agent trees".into(),
            cost: Some("$0.34 for 47 agents".into()),
        },
        ExampleManifest {
            name: "synthetic-data-generation".into(),
            description: "Generate training data with quality filtering".into(),
            cost: Some("$3.21 for 10K samples".into()),
        },
        ExampleManifest {
            name: "data-processing-pipelines".into(),
            description: "Clean and enrich messy records".into(),
            cost: Some("$0.80 for 50K records".into()),
        },
        ExampleManifest {
            name: "embeddings".into(),
            description: "Semantic search over document corpus".into(),
            cost: Some("$0.03 for 1.6M tokens".into()),
        },
        ExampleManifest {
            name: "model-evals".into(),
            description: "Benchmark models on GSM8K".into(),
            cost: Some("$0.21 for 1,319 questions".into()),
        },
        ExampleManifest {
            name: "bug-detection-ensemble".into(),
            description: "Classify security vulnerabilities".into(),
            cost: Some("$0.40 for 4,642 samples".into()),
        },
        ExampleManifest {
            name: "dataset-compilation".into(),
            description: "Build company datasets via search + LLM".into(),
            cost: Some("$1.05 for 188 companies".into()),
        },
        ExampleManifest {
            name: "structured-extraction".into(),
            description: "Extract fields from scanned receipts".into(),
            cost: Some("$0.12 for 626 receipts".into()),
        },
        ExampleManifest {
            name: "image-summarization".into(),
            description: "Caption images for social media".into(),
            cost: Some("$0.10 for 1,000 images".into()),
        },
    ]
}

pub fn list() {
    let examples = builtin_manifest();

    println!("{:<30} {:<50} COST", "NAME", "DESCRIPTION");
    println!("{}", "-".repeat(95));
    for ex in &examples {
        println!(
            "{:<30} {:<50} {}",
            ex.name,
            ex.description,
            ex.cost.as_deref().unwrap_or("-")
        );
    }
    println!();
    println!("Clone an example: dw examples clone <name>");
}

pub async fn clone_example(name: &str, dir: Option<&Path>) -> anyhow::Result<()> {
    // Validate example exists
    let examples = builtin_manifest();
    if !examples.iter().any(|e| e.name == name) {
        let available: Vec<_> = examples.iter().map(|e| e.name.as_str()).collect();
        anyhow::bail!(
            "Unknown example: '{}'. Available: {}",
            name,
            available.join(", ")
        );
    }

    let target_dir = dir
        .map(|d| d.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from(name));

    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists.", target_dir.display());
    }

    // Try downloading tarball from GitHub
    eprintln!("Downloading {}...", name);

    let tarball_url = format!("{}/repos/{}/tarball/main", GITHUB_API_BASE, EXAMPLES_REPO);

    let client = reqwest::Client::new();
    let response = client
        .get(&tarball_url)
        .header("User-Agent", "dw-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            let bytes = resp.bytes().await?;
            extract_example_from_tarball(&bytes, name, &target_dir)?;
            eprintln!("Cloned '{}' to {}/", name, target_dir.display());
            eprintln!("Get started:");
            eprintln!("  cd {}", target_dir.display());
            eprintln!("  uv sync");
            eprintln!("  export DOUBLEWORD_API_KEY=\"your-key\"");
            Ok(())
        }
        _ => {
            // Fallback to git clone
            clone_with_git(name, &target_dir).await
        }
    }
}

fn extract_example_from_tarball(
    tarball: &[u8],
    example_name: &str,
    target_dir: &Path,
) -> anyhow::Result<()> {
    use std::io::Read;

    let decoder = flate2::read::GzDecoder::new(tarball);
    let mut archive = tar::Archive::new(decoder);

    std::fs::create_dir_all(target_dir)?;

    let prefix_pattern = format!("/{}/", example_name);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let path_str = path.to_string_lossy();

        // Find entries matching the example directory
        if let Some(idx) = path_str.find(&prefix_pattern) {
            let relative = &path_str[(idx + prefix_pattern.len())..];
            if relative.is_empty() {
                continue;
            }

            let target = target_dir.join(relative);
            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&target)?;
            } else {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut content = Vec::new();
                entry.read_to_end(&mut content)?;
                std::fs::write(&target, &content)?;
            }
        }
    }

    Ok(())
}

async fn clone_with_git(name: &str, target_dir: &Path) -> anyhow::Result<()> {
    eprintln!("Falling back to git clone...");

    let repo_url = format!("https://github.com/{}.git", EXAMPLES_REPO);

    let status = tokio::process::Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--filter=blob:none",
            "--sparse",
            &repo_url,
            &target_dir.to_string_lossy(),
        ])
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("git clone failed. Ensure git is installed.");
    }

    let status = tokio::process::Command::new("git")
        .args(["sparse-checkout", "set", name])
        .current_dir(target_dir)
        .status()
        .await?;

    if !status.success() {
        anyhow::bail!("git sparse-checkout failed.");
    }

    eprintln!("Cloned '{}' to {}/", name, target_dir.display());
    Ok(())
}
