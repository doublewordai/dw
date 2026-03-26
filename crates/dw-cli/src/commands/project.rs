use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Project manifest parsed from dw.toml.
#[derive(Debug, Deserialize)]
pub struct ProjectManifest {
    pub project: Option<ProjectInfo>,
    #[serde(default)]
    pub steps: BTreeMap<String, StepDef>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInfo {
    pub name: Option<String>,
    pub setup: Option<String>,
    /// Full workflow instructions shown by `dw project info`.
    #[serde(default)]
    pub workflow: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct StepDef {
    pub description: Option<String>,
    pub run: String,
}

const RUN_STATE_FILE: &str = ".dw-run.json";

/// Persistent run state written during `run-all`.
#[derive(Debug, Serialize, Deserialize)]
struct RunState {
    started_at: String,
    completed_steps: usize,
    total_steps: usize,
    steps: Vec<StepState>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StepState {
    index: usize,
    command: String,
    status: String, // "completed", "failed", "skipped"
    #[serde(skip_serializing_if = "Option::is_none")]
    batch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    at: Option<String>,
}

impl RunState {
    fn new(total: usize) -> Self {
        Self {
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_steps: 0,
            total_steps: total,
            steps: Vec::new(),
        }
    }

    fn save(&self, dir: &Path) -> anyhow::Result<()> {
        let path = dir.join(RUN_STATE_FILE);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load(dir: &Path) -> anyhow::Result<Self> {
        let path = dir.join(RUN_STATE_FILE);
        let contents = std::fs::read_to_string(&path)
            .map_err(|_| anyhow::anyhow!("No run state found. Run `dw project run-all` first."))?;
        Ok(serde_json::from_str(&contents)?)
    }

    fn record_step(&mut self, index: usize, command: &str, status: &str, batch_id: Option<String>) {
        self.steps.push(StepState {
            index,
            command: command.to_string(),
            status: status.to_string(),
            batch_id,
            at: Some(chrono::Utc::now().to_rfc3339()),
        });
        if status == "completed" {
            self.completed_steps = self.completed_steps.max(index);
        }
    }
}

/// Loaded manifest with its directory (for setting cwd).
struct LoadedManifest {
    manifest: ProjectManifest,
    dir: PathBuf,
}

/// Load dw.toml and return the manifest + its parent directory.
fn load_manifest() -> anyhow::Result<LoadedManifest> {
    let path = find_manifest()?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Could not read {}: {}", path.display(), e))?;
    let manifest: ProjectManifest =
        toml::from_str(&contents).map_err(|e| anyhow::anyhow!("Invalid dw.toml: {}", e))?;
    Ok(LoadedManifest { manifest, dir })
}

/// Search for dw.toml in the current directory and parents.
fn find_manifest() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("dw.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            anyhow::bail!(
                "No dw.toml found in this directory or any parent. \
                 Create a dw.toml or run from a project directory."
            );
        }
    }
}

/// Run the project setup command.
pub fn setup() -> anyhow::Result<()> {
    let loaded = load_manifest()?;
    let setup_cmd = loaded
        .manifest
        .project
        .as_ref()
        .and_then(|p| p.setup.as_deref())
        .unwrap_or("uv sync");

    eprintln!("Running setup: {}", setup_cmd);
    run_shell_command(setup_cmd, &[], &loaded.dir)
}

/// Run a named step from dw.toml.
pub fn run(step: &str, extra_args: &[String]) -> anyhow::Result<()> {
    let loaded = load_manifest()?;
    let step_def = loaded.manifest.steps.get(step).ok_or_else(|| {
        let available: Vec<_> = loaded.manifest.steps.keys().map(|k| k.as_str()).collect();
        anyhow::anyhow!(
            "Step '{}' not found in dw.toml. Available steps: {}",
            step,
            if available.is_empty() {
                "(none)".to_string()
            } else {
                available.join(", ")
            }
        )
    })?;

    if let Some(ref desc) = step_def.description {
        eprintln!("{}", desc);
    }

    run_shell_command(&step_def.run, extra_args, &loaded.dir)
}

/// Show project info and available steps.
pub fn info() -> anyhow::Result<()> {
    let loaded = load_manifest()?;

    if let Some(ref project) = loaded.manifest.project
        && let Some(ref name) = project.name
    {
        println!("Project: {}", name);
    }
    let setup_cmd = loaded
        .manifest
        .project
        .as_ref()
        .and_then(|p| p.setup.as_deref())
        .unwrap_or("uv sync");
    println!("Setup:   {}", setup_cmd);
    println!();

    if !loaded.manifest.steps.is_empty() {
        // Display steps in workflow order (fall back to alphabetical for unlisted steps)
        let workflow_order: Vec<&str> = loaded
            .manifest
            .project
            .as_ref()
            .and_then(|p| p.workflow.as_ref())
            .map(|w| {
                w.iter()
                    .filter_map(|line| {
                        // Extract step name from "dw project run <step> ..."
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4
                            && parts[0] == "dw"
                            && parts[1] == "project"
                            && parts[2] == "run"
                        {
                            Some(parts[3])
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Precompute position map for O(1) lookup during sort
        let pos_map: std::collections::HashMap<&str, usize> = workflow_order
            .iter()
            .enumerate()
            .map(|(i, name)| (*name, i))
            .collect();

        let mut ordered_steps: Vec<(&String, &StepDef)> = loaded.manifest.steps.iter().collect();
        ordered_steps.sort_by(|(a, _), (b, _)| {
            let pos_a = pos_map.get(a.as_str()).copied().unwrap_or(usize::MAX);
            let pos_b = pos_map.get(b.as_str()).copied().unwrap_or(usize::MAX);
            pos_a.cmp(&pos_b).then_with(|| a.cmp(b))
        });

        println!("Steps:");
        for (name, step) in &ordered_steps {
            let desc = step.description.as_deref().unwrap_or(&step.run);
            println!("  {:<20} {}", name, desc);
        }
        println!();
    }

    // Show workflow if defined and non-empty
    if let Some(ref project) = loaded.manifest.project
        && let Some(ref workflow) = project.workflow
        && !workflow.is_empty()
    {
        println!("Workflow:");
        for (i, step) in workflow.iter().enumerate() {
            println!("  {}. {}", i + 1, step);
        }
        println!();
    }

    Ok(())
}

/// Initialize a new project with scaffolding.
pub fn init(
    name: Option<&str>,
    template: Option<&str>,
    with_sdks: &[String],
) -> anyhow::Result<()> {
    use std::io::{IsTerminal, Write};

    let interactive = std::io::stdin().is_terminal();

    // Interactive name prompt if not provided
    let project_name = if let Some(n) = name {
        n.to_string()
    } else if !interactive {
        anyhow::bail!(
            "Project name is required in non-interactive mode. Usage: dw project init <name>"
        );
    } else {
        eprint!("Project name: ");
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            anyhow::bail!("Project name is required.");
        }
        trimmed
    };

    // Interactive template selection if not provided
    let template_name = if let Some(t) = template {
        t.to_string()
    } else if !interactive {
        "single-batch".to_string()
    } else {
        eprintln!("\nTemplate:");
        eprintln!("  1. single-batch  — prepare → submit → analyze (default)");
        eprintln!("  2. pipeline      — multi-stage with transform between batches");
        eprintln!("  3. shell         — bash scripts, no Python");
        eprintln!("  4. minimal       — just dw.toml and directories");
        eprint!("\nChoice [1]: ");
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        match input.trim() {
            "" | "1" | "single-batch" => "single-batch".to_string(),
            "2" | "pipeline" => "pipeline".to_string(),
            "3" | "shell" => "shell".to_string(),
            "4" | "minimal" => "minimal".to_string(),
            other => {
                eprintln!("Unknown template '{}', using single-batch.", other);
                "single-batch".to_string()
            }
        }
    };

    // Interactive SDK selection if no --with flags and using Python templates
    // Prompt for SDKs interactively if none specified via --with and using a Python template
    let effective_sdks: Vec<String> = if with_sdks.is_empty()
        && interactive
        && matches!(template_name.as_str(), "single-batch" | "pipeline")
    {
        eprintln!("\nOptional SDKs (enter numbers, comma-separated, or press Enter to skip):");
        eprintln!("  1. autobatcher  — automatic batching behind an async OpenAI client");
        eprintln!("  2. parfold      — LLM-powered data primitives (sort, filter, map)");
        eprint!("\nInclude [none]: ");
        std::io::stderr().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let mut sdks = Vec::new();
        for part in input.trim().split(',') {
            match part.trim() {
                "1" | "autobatcher" => sdks.push("autobatcher".to_string()),
                "2" | "parfold" => sdks.push("parfold".to_string()),
                "" => {}
                _ => {}
            }
        }
        sdks
    } else {
        with_sdks.to_vec()
    };

    // Validate name: only alphanumeric, hyphens, underscores, dots.
    // This is safe for: directory names, TOML values, Python package names,
    // shell command interpolation (via uv run {name}).
    if project_name.is_empty()
        || !project_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        anyhow::bail!(
            "Invalid project name '{}'. Use only letters, numbers, hyphens, underscores, and dots.",
            project_name
        );
    }

    let dir = Path::new(&project_name);
    if dir.exists() {
        anyhow::bail!("Directory '{}' already exists.", project_name);
    }

    // Create directories
    std::fs::create_dir_all(dir.join("batches"))?;
    std::fs::create_dir_all(dir.join("results"))?;

    // Generate files based on template
    match template_name.as_str() {
        "single-batch" => {
            generate_single_batch(dir, &project_name, &effective_sdks)?;
        }
        "pipeline" => {
            generate_pipeline(dir, &project_name, &effective_sdks)?;
        }
        "shell" => {
            generate_shell(dir, &project_name)?;
        }
        "minimal" => {
            generate_minimal(dir, &project_name)?;
        }
        _ => {
            generate_single_batch(dir, &project_name, &effective_sdks)?;
        }
    }

    eprintln!("\nCreated {}/", project_name);
    eprintln!("  dw.toml          Project manifest");
    match template_name.as_str() {
        "shell" => {
            eprintln!("  scripts/         Shell scripts");
        }
        "minimal" => {}
        _ => {
            eprintln!("  pyproject.toml   Python dependencies");
            eprintln!("  src/             Project source code");
        }
    }
    eprintln!("  batches/         Batch JSONL files");
    eprintln!("  results/         Results and analysis");
    eprintln!("\nGet started:");
    eprintln!("  cd {}", project_name);
    eprintln!("  dw project info");

    Ok(())
}

/// Run all workflow steps sequentially (skipping setup), with state tracking.
pub fn run_all(from: usize, continue_run: bool) -> anyhow::Result<()> {
    let loaded = load_manifest()?;
    let workflow = loaded
        .manifest
        .project
        .as_ref()
        .and_then(|p| p.workflow.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No workflow defined in dw.toml."))?;

    if workflow.is_empty() {
        anyhow::bail!("Workflow is empty in dw.toml.");
    }

    // Filter out setup steps
    let steps: Vec<(usize, &str)> = workflow
        .iter()
        .enumerate()
        .map(|(i, s)| (i + 1, s.trim()))
        .filter(|(_, s)| *s != "dw project setup" && !s.starts_with("dw project setup "))
        .collect();

    // Determine start point
    let start_from = if continue_run {
        let prev_state = RunState::load(&loaded.dir)?;
        let resume = prev_state.completed_steps + 1;
        eprintln!(
            "Resuming from step {} (previously completed {} of {} steps)",
            resume, prev_state.completed_steps, prev_state.total_steps
        );
        resume
    } else if from > 0 {
        from
    } else {
        anyhow::bail!("--from is 1-indexed. Use --from 1 to start from the beginning.");
    };

    // Initialize or load state
    let mut state = if continue_run {
        RunState::load(&loaded.dir)?
    } else {
        RunState::new(steps.len())
    };

    let mut executed = 0;
    let total_remaining = steps.iter().filter(|(idx, _)| *idx >= start_from).count();

    for (original_idx, step) in &steps {
        if *original_idx < start_from {
            continue;
        }

        executed += 1;
        eprintln!("\n[{}/{}] {}", executed, total_remaining, step);

        match run_shell_command(step, &[], &loaded.dir) {
            Ok(()) => {
                state.record_step(*original_idx, step, "completed", None);
                state.save(&loaded.dir)?;
            }
            Err(e) => {
                state.record_step(*original_idx, step, "failed", None);
                state.save(&loaded.dir)?;
                eprintln!(
                    "\nStep {} failed. Resume with: dw project run-all --continue",
                    original_idx
                );
                return Err(e);
            }
        }
    }

    eprintln!("\nAll steps completed.");
    Ok(())
}

/// Show current run status.
pub fn status() -> anyhow::Result<()> {
    let loaded = load_manifest()?;
    let state = RunState::load(&loaded.dir)?;

    println!("Run started: {}", state.started_at);
    println!(
        "Progress:    {}/{} steps completed",
        state.completed_steps, state.total_steps
    );
    println!();

    if state.steps.is_empty() {
        println!("No steps recorded yet.");
    } else {
        for step in &state.steps {
            let status_icon = match step.status.as_str() {
                "completed" => "✓",
                "failed" => "✗",
                "skipped" => "–",
                _ => "?",
            };
            let batch_info = step
                .batch_id
                .as_deref()
                .map(|id| format!("  batch: {}", id))
                .unwrap_or_default();
            println!(
                "  {} [{}] {}{}",
                status_icon, step.index, step.command, batch_info
            );
        }
    }

    // Show resume hint if incomplete
    if state.steps.iter().any(|s| s.status == "failed") {
        eprintln!("\nResume with: dw project run-all --continue");
    }

    Ok(())
}

/// Clean project artifacts.
pub fn clean() -> anyhow::Result<()> {
    let loaded = load_manifest()?;
    let dir = &loaded.dir;

    let mut removed = Vec::new();

    for subdir in &["batches", "results", "output"] {
        let path = dir.join(subdir);
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
            std::fs::create_dir_all(&path)?; // recreate empty
            removed.push(*subdir);
        }
    }

    let state_path = dir.join(RUN_STATE_FILE);
    if state_path.exists() {
        std::fs::remove_file(&state_path)?;
        removed.push(".dw-run.json");
    }

    if removed.is_empty() {
        eprintln!("Nothing to clean.");
    } else {
        eprintln!("Cleaned: {}", removed.join(", "));
    }

    Ok(())
}

// ===== Template generators =====

fn write_file(path: &Path, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

fn generate_minimal(dir: &Path, name: &str) -> anyhow::Result<()> {
    write_file(
        &dir.join("dw.toml"),
        &format!(
            r#"[project]
name = "{name}"
"#
        ),
    )
}

fn generate_shell(dir: &Path, name: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir.join("scripts"))?;

    write_file(
        &dir.join("dw.toml"),
        &format!(
            r#"[project]
name = "{name}"
setup = "echo 'No setup needed for shell projects'"
workflow = [
    "dw project run prepare",
    "dw files stats batches/batch.jsonl",
    "dw files prepare batches/batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "dw stream batches/batch.jsonl > results.jsonl",
    "dw project run analyze",
    "dw usage",
]

[steps.prepare]
description = "Generate batch JSONL"
run = "bash scripts/prepare.sh"

[steps.analyze]
description = "Analyze results"
run = "bash scripts/analyze.sh"
"#
        ),
    )?;

    write_file(
        &dir.join("scripts/prepare.sh"),
        r#"#!/bin/bash
# Generate batch-ready JSONL. Edit this to load your data and build prompts.
set -e

mkdir -p batches

cat > batches/batch.jsonl << 'JSONL'
{"custom_id":"hello-0","method":"POST","url":"/v1/chat/completions","body":{"model":"Qwen/Qwen3-VL-30B-A3B-Instruct-FP8","messages":[{"role":"user","content":"What is the capital of France?"}]}}
{"custom_id":"hello-1","method":"POST","url":"/v1/chat/completions","body":{"model":"Qwen/Qwen3-VL-30B-A3B-Instruct-FP8","messages":[{"role":"user","content":"What is the capital of Japan?"}]}}
{"custom_id":"hello-2","method":"POST","url":"/v1/chat/completions","body":{"model":"Qwen/Qwen3-VL-30B-A3B-Instruct-FP8","messages":[{"role":"user","content":"What is the capital of Brazil?"}]}}
{"custom_id":"hello-3","method":"POST","url":"/v1/chat/completions","body":{"model":"Qwen/Qwen3-VL-30B-A3B-Instruct-FP8","messages":[{"role":"user","content":"What is the capital of Australia?"}]}}
{"custom_id":"hello-4","method":"POST","url":"/v1/chat/completions","body":{"model":"Qwen/Qwen3-VL-30B-A3B-Instruct-FP8","messages":[{"role":"user","content":"What is the capital of Egypt?"}]}}
JSONL

echo "Created batches/batch.jsonl (5 requests)"
"#,
    )?;

    write_file(
        &dir.join("scripts/analyze.sh"),
        r#"#!/bin/bash
# Analyze batch results. Edit this to process your outputs.
set -e

if [ ! -f results.jsonl ]; then
    echo "Error: results.jsonl not found. Run the batch first."
    exit 1
fi

echo "Results:"
wc -l results.jsonl
echo ""
echo "First result:"
head -1 results.jsonl | python3 -m json.tool 2>/dev/null || head -1 results.jsonl
"#,
    )?;

    // Make scripts executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            dir.join("scripts/prepare.sh"),
            std::fs::Permissions::from_mode(0o755),
        )?;
        std::fs::set_permissions(
            dir.join("scripts/analyze.sh"),
            std::fs::Permissions::from_mode(0o755),
        )?;
    }

    Ok(())
}

fn generate_single_batch(dir: &Path, name: &str, with_sdks: &[String]) -> anyhow::Result<()> {
    write_file(
        &dir.join("dw.toml"),
        &format!(
            r#"[project]
name = "{name}"
setup = "uv sync"
workflow = [
    "dw project setup",
    "dw project run prepare",
    "dw files stats batches/batch.jsonl",
    "dw files prepare batches/batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "dw stream batches/batch.jsonl > results.jsonl",
    "dw project run analyze -- -r results.jsonl",
    "dw usage",
]

[steps.prepare]
description = "Generate batch JSONL from your data"
run = "uv run {name} prepare"

[steps.analyze]
description = "Analyze batch results"
run = "uv run {name} analyze"
"#
        ),
    )?;

    generate_pyproject(dir, name, with_sdks)?;
    generate_single_batch_cli(dir)?;

    Ok(())
}

fn generate_pipeline(dir: &Path, name: &str, with_sdks: &[String]) -> anyhow::Result<()> {
    write_file(
        &dir.join("dw.toml"),
        &format!(
            r#"[project]
name = "{name}"
setup = "uv sync"
workflow = [
    "dw project setup",
    "dw project run prepare-stage1",
    "dw files stats batches/stage1.jsonl",
    "dw files prepare batches/stage1.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "dw stream batches/stage1.jsonl > results/stage1.jsonl",
    "dw project run transform -- --input results/stage1.jsonl",
    "dw files prepare batches/stage2.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "dw stream batches/stage2.jsonl > results/stage2.jsonl",
    "dw project run analyze -- -r results/stage2.jsonl",
    "dw usage",
]

[steps.prepare-stage1]
description = "Generate batch JSONL for stage 1"
run = "uv run {name} prepare-stage1"

[steps.transform]
description = "Transform stage 1 results into stage 2 inputs"
run = "uv run {name} transform"

[steps.analyze]
description = "Analyze final results"
run = "uv run {name} analyze"
"#
        ),
    )?;

    generate_pyproject(dir, name, with_sdks)?;
    generate_pipeline_cli(dir)?;

    Ok(())
}

fn generate_pyproject(dir: &Path, name: &str, with_sdks: &[String]) -> anyhow::Result<()> {
    let mut deps = vec!["    \"click>=8.0\"".to_string()];
    for sdk in with_sdks {
        match sdk.as_str() {
            "autobatcher" => deps.push("    \"autobatcher>=0.1\"".to_string()),
            "parfold" => deps.push("    \"parfold>=0.1\"".to_string()),
            other => eprintln!("Warning: unknown SDK '{}', skipping.", other),
        }
    }

    write_file(
        &dir.join("pyproject.toml"),
        &format!(
            r#"[build-system]
requires = ["setuptools>=64"]
build-backend = "setuptools.build_meta"

[project]
name = "{name}"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
{deps}
]

[project.scripts]
{name} = "src.cli:main"

[tool.setuptools.packages.find]
where = ["."]
"#,
            deps = deps.join(",\n")
        ),
    )
}

fn generate_single_batch_cli(dir: &Path) -> anyhow::Result<()> {
    write_file(&dir.join("src/__init__.py"), "")?;

    write_file(
        &dir.join("src/cli.py"),
        r#"""Batch inference project. Edit prepare() and analyze() for your use case."""

import json
from pathlib import Path

import click


@click.group()
def cli():
    """Batch inference project."""
    pass


@cli.command()
@click.option("--output", "-o", default="batches/batch.jsonl", help="Output JSONL file")
def prepare(output):
    """Generate batch-ready JSONL. Edit this for your data and prompts."""
    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # TODO: Replace with your data loading and prompt building
    prompts = [
        "What is the capital of France?",
        "Explain quantum computing in one sentence.",
        "Write a haiku about batch inference.",
        "What is the largest ocean on Earth?",
        "Name three programming languages created in the 1990s.",
        "What causes rain?",
        "Summarize the plot of Romeo and Juliet in two sentences.",
        "What is the speed of light in km/s?",
        "Explain what an API is to a five-year-old.",
        "What year did the first iPhone launch?",
    ]

    with open(output_path, "w") as f:
        for i, prompt in enumerate(prompts):
            line = {
                "custom_id": f"request-{i:04d}",
                "method": "POST",
                "url": "/v1/chat/completions",
                "body": {
                    "model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
                    "messages": [{"role": "user", "content": prompt}],
                    "max_tokens": 256,
                },
            }
            f.write(json.dumps(line) + "\n")

    click.echo(f"Created {output_path} ({len(prompts)} requests)")


@cli.command()
@click.option("--results", "-r", required=True, help="Results JSONL file")
def analyze(results):
    """Analyze batch results. Edit this for your analysis logic."""
    results_path = Path(results)
    if not results_path.exists():
        raise click.ClickException(f"Results file not found: {results_path}")

    count = 0
    errors = 0
    with open(results_path) as f:
        for line in f:
            if not line.strip():
                continue
            obj = json.loads(line)
            # Handle both response formats (Doubleword and OpenAI)
            rb = obj.get("response_body") or obj.get("response", {}).get("body", {})
            choices = rb.get("choices", [])
            if choices:
                content = choices[0].get("message", {}).get("content", "")
                click.echo(f"[{obj['custom_id']}] {content[:100]}...")
                count += 1
            elif obj.get("error"):
                errors += 1

    click.echo(f"\n{count} results, {errors} errors")


def main():
    cli()
"#,
    )
}

fn generate_pipeline_cli(dir: &Path) -> anyhow::Result<()> {
    write_file(&dir.join("src/__init__.py"), "")?;

    write_file(
        &dir.join("src/cli.py"),
        r#"""Multi-stage batch pipeline. Edit the stages for your use case."""

import json
from pathlib import Path

import click


@click.group()
def cli():
    """Multi-stage batch inference pipeline."""
    pass


@cli.command("prepare-stage1")
@click.option("--output", "-o", default="batches/stage1.jsonl", help="Output JSONL file")
def prepare_stage1(output):
    """Generate batch JSONL for stage 1. Edit this for your data."""
    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # TODO: Replace with your data loading
    items = [
        "The history of artificial intelligence",
        "Climate change and its effects on agriculture",
        "The evolution of programming languages",
        "Space exploration milestones",
        "The future of renewable energy",
    ]

    with open(output_path, "w") as f:
        for i, item in enumerate(items):
            line = {
                "custom_id": f"stage1-{i:04d}",
                "method": "POST",
                "url": "/v1/chat/completions",
                "body": {
                    "model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
                    "messages": [
                        {"role": "user", "content": f"Write a brief summary about: {item}"}
                    ],
                    "max_tokens": 512,
                },
            }
            f.write(json.dumps(line) + "\n")

    click.echo(f"Created {output_path} ({len(items)} requests)")


@cli.command()
@click.option("--input", "-i", "input_path", required=True, help="Stage 1 results JSONL")
@click.option("--output", "-o", default="batches/stage2.jsonl", help="Output JSONL file")
def transform(input_path, output):
    """Transform stage 1 results into stage 2 inputs.

    This is where you read the previous stage's outputs and build
    new prompts for the next stage. Edit for your pipeline logic.
    """
    input_file = Path(input_path)
    output_path = Path(output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    results = []
    with open(input_file) as f:
        for line in f:
            if line.strip():
                results.append(json.loads(line))

    click.echo(f"Loaded {len(results)} stage 1 results")

    # TODO: Replace with your transformation logic
    with open(output_path, "w") as f:
        for i, result in enumerate(results):
            # Extract the summary from stage 1
            rb = result.get("response_body") or result.get("response", {}).get("body", {})
            choices = rb.get("choices", [])
            summary = choices[0]["message"]["content"] if choices else ""

            # Build stage 2 prompt using stage 1 output
            line = {
                "custom_id": f"stage2-{i:04d}",
                "method": "POST",
                "url": "/v1/chat/completions",
                "body": {
                    "model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
                    "messages": [
                        {
                            "role": "user",
                            "content": f"Given this summary:\n\n{summary}\n\n"
                            "List the 3 most important facts mentioned.",
                        }
                    ],
                    "max_tokens": 256,
                },
            }
            f.write(json.dumps(line) + "\n")

    click.echo(f"Created {output_path} ({len(results)} requests)")


@cli.command()
@click.option("--results", "-r", required=True, help="Final results JSONL file")
def analyze(results):
    """Analyze final pipeline results."""
    results_path = Path(results)
    if not results_path.exists():
        raise click.ClickException(f"Results file not found: {results_path}")

    count = 0
    with open(results_path) as f:
        for line in f:
            if not line.strip():
                continue
            obj = json.loads(line)
            rb = obj.get("response_body") or obj.get("response", {}).get("body", {})
            choices = rb.get("choices", [])
            if choices:
                content = choices[0].get("message", {}).get("content", "")
                click.echo(f"[{obj['custom_id']}] {content[:120]}...")
                count += 1

    click.echo(f"\n{count} final results")


def main():
    cli()
"#,
    )
}

/// Shell-escape a single argument (POSIX).
fn shell_escape_posix(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }
    if arg
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' || c == ':')
    {
        return arg.to_string();
    }
    format!("'{}'", arg.replace('\'', "'\\''"))
}

/// Execute a shell command in the manifest's directory.
/// Uses POSIX `sh`. On Windows, this requires that a `sh` binary is available
/// on PATH (for example from Git Bash or MSYS2), or that the CLI is run inside
/// a Linux/WSL environment where `sh` is present.
///
/// This helper does not inject credentials; it simply inherits the environment
/// of the parent process. Project steps that need API access should prefer
/// `dw` subcommands (which read credentials from `~/.dw/` automatically).
fn run_shell_command(cmd: &str, extra_args: &[String], cwd: &Path) -> anyhow::Result<()> {
    let full_cmd = if extra_args.is_empty() {
        cmd.to_string()
    } else {
        let escaped: Vec<String> = extra_args.iter().map(|a| shell_escape_posix(a)).collect();
        format!("{} {}", cmd, escaped.join(" "))
    };

    let status = Command::new("sh")
        .args(["-c", &full_cmd])
        .current_dir(cwd)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "'sh' not found. On Windows, install Git Bash, WSL, or MSYS2. \
                     On other systems, ensure /bin/sh is available."
                )
            } else {
                anyhow::anyhow!("Failed to execute command: {}", e)
            }
        })?;

    if !status.success() {
        anyhow::bail!(
            "Command failed with exit code {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
