use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
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

    if let Some(ref project) = loaded.manifest.project {
        if let Some(ref name) = project.name {
            println!("Project: {}", name);
        }
        if let Some(ref setup) = project.setup {
            println!("Setup:   {}", setup);
        }
        println!();
    }

    if loaded.manifest.steps.is_empty() {
        println!("No steps defined in dw.toml.");
    } else {
        println!("Steps:");
        for (name, step) in &loaded.manifest.steps {
            let desc = step.description.as_deref().unwrap_or(&step.run);
            println!("  {:<20} {}", name, desc);
        }
        println!();
        println!("Run with: dw project run <step> [args...]");
    }

    Ok(())
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

/// Shell-escape a single argument (Windows cmd.exe).
fn shell_escape_windows(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }
    if arg
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/' || c == ':')
    {
        return arg.to_string();
    }
    // Wrap in double quotes, escape embedded double quotes
    format!("\"{}\"", arg.replace('"', "\\\""))
}

/// Execute a shell command in the manifest's directory, appending escaped extra args.
fn run_shell_command(cmd: &str, extra_args: &[String], cwd: &Path) -> anyhow::Result<()> {
    let (full_cmd, shell, shell_flag) = if cfg!(target_os = "windows") {
        let escaped: Vec<String> = extra_args.iter().map(|a| shell_escape_windows(a)).collect();
        let full = if escaped.is_empty() {
            cmd.to_string()
        } else {
            format!("{} {}", cmd, escaped.join(" "))
        };
        (full, "cmd", "/C")
    } else {
        let escaped: Vec<String> = extra_args.iter().map(|a| shell_escape_posix(a)).collect();
        let full = if escaped.is_empty() {
            cmd.to_string()
        } else {
            format!("{} {}", cmd, escaped.join(" "))
        };
        (full, "sh", "-c")
    };

    let status = Command::new(shell)
        .args([shell_flag, &full_cmd])
        .current_dir(cwd)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

    if !status.success() {
        anyhow::bail!(
            "Command failed with exit code {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
