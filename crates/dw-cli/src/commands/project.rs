use std::collections::BTreeMap;
use std::process::Command;

use serde::Deserialize;

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
}

#[derive(Debug, Deserialize)]
pub struct StepDef {
    pub description: Option<String>,
    pub run: String,
}

/// Load dw.toml from the current directory (or parent directories).
fn load_manifest() -> anyhow::Result<ProjectManifest> {
    let path = find_manifest()?;
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Could not read {}: {}", path.display(), e))?;
    let manifest: ProjectManifest =
        toml::from_str(&contents).map_err(|e| anyhow::anyhow!("Invalid dw.toml: {}", e))?;
    Ok(manifest)
}

/// Search for dw.toml in the current directory and parents.
fn find_manifest() -> anyhow::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("dw.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            anyhow::bail!(
                "No dw.toml found. Run this command from a project directory \
                 (cloned via `dw examples clone <name>`)."
            );
        }
    }
}

/// Run the project setup command.
pub fn setup() -> anyhow::Result<()> {
    let manifest = load_manifest()?;
    let setup_cmd = manifest
        .project
        .as_ref()
        .and_then(|p| p.setup.as_deref())
        .unwrap_or("uv sync");

    eprintln!("Running setup: {}", setup_cmd);
    run_shell_command(setup_cmd, &[])
}

/// Run a named step from dw.toml.
pub fn run(step: &str, extra_args: &[String]) -> anyhow::Result<()> {
    let manifest = load_manifest()?;
    let step_def = manifest.steps.get(step).ok_or_else(|| {
        let available: Vec<_> = manifest.steps.keys().map(|k| k.as_str()).collect();
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

    run_shell_command(&step_def.run, extra_args)
}

/// Show project info and available steps.
pub fn info() -> anyhow::Result<()> {
    let manifest = load_manifest()?;

    if let Some(ref project) = manifest.project {
        if let Some(ref name) = project.name {
            println!("Project: {}", name);
        }
        if let Some(ref setup) = project.setup {
            println!("Setup:   {}", setup);
        }
        println!();
    }

    if manifest.steps.is_empty() {
        println!("No steps defined in dw.toml.");
    } else {
        println!("Steps:");
        for (name, step) in &manifest.steps {
            let desc = step.description.as_deref().unwrap_or(&step.run);
            println!("  {:<20} {}", name, desc);
        }
        println!();
        println!("Run with: dw project run <step> [args...]");
    }

    Ok(())
}

/// Execute a shell command, appending extra args.
fn run_shell_command(cmd: &str, extra_args: &[String]) -> anyhow::Result<()> {
    let full_cmd = if extra_args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, extra_args.join(" "))
    };

    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &full_cmd]).status()
    } else {
        Command::new("sh").args(["-c", &full_cmd]).status()
    }
    .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

    if !status.success() {
        anyhow::bail!(
            "Command failed with exit code {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
