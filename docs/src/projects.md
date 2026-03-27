# Project System

The project system lets you define multi-step workflows in a `dw.toml` manifest file and run them with `dw project` commands.

## Creating a Project

### From a Template

```bash
dw project init my-project
```

Interactive prompts let you choose a template:

| Template | Description |
|----------|-------------|
| `single-batch` | Prepare, submit, and analyze a single batch |
| `pipeline` | Multi-stage pipeline (prepare -> run stages -> analyze) |
| `shell` | Shell-script steps instead of Python |
| `minimal` | Just a dw.toml with no scaffolding |

Or specify directly:

```bash
dw project init my-project --template pipeline
```

### From an Example

```bash
dw examples clone model-evals
cd model-evals
```

### Manually

Create a `dw.toml` in your project directory.

## The dw.toml Manifest

```toml
[project]
name = "my-project"
setup = "uv sync"
workflow = [
    "dw project setup",
    "dw project run prepare -- -n 100",
    "dw files stats output/batch.jsonl",
    "dw files prepare output/batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "dw stream output/batch.jsonl > results.jsonl",
    "dw project run analyze -- -r results.jsonl",
    "dw usage",
]

[steps.prepare]
description = "Download dataset and generate batch JSONL"
run = "uv run my-project prepare"

[steps.analyze]
description = "Score results against ground truth"
run = "uv run my-project score"
```

### Fields

| Field | Description |
|-------|-------------|
| `project.name` | Project name |
| `project.setup` | Command to run on `dw project setup` (e.g., `uv sync`) |
| `project.workflow` | Ordered list of commands for `dw project run-all` and `dw project info` |
| `steps.<name>.run` | Shell command to execute for this step |
| `steps.<name>.description` | Human-readable description shown in `dw project info` |

## Running Steps

### Setup

Install dependencies:

```bash
dw project setup
```

Runs the `project.setup` command from `dw.toml`.

### Individual Steps

```bash
dw project run prepare
```

Pass extra arguments after `--`:

```bash
dw project run prepare -- -n 100 --output custom.jsonl
```

### Full Workflow

Run all workflow steps sequentially:

```bash
dw project run-all
```

This executes every command in the `workflow` array, skipping `dw project setup` (run it separately first).

### Resume After Failure

If a step fails, fix the issue and continue from where you left off:

```bash
dw project run-all --continue
```

Or start from a specific step:

```bash
dw project run-all --from 3
```

## Inspecting State

### Project Info

```bash
dw project info
```

Shows available steps, descriptions, and the full workflow.

### Run Status

```bash
dw project status
```

Shows the current run state: which steps have completed, which failed, and where the run left off.

## Cleanup

```bash
dw project clean
```

Removes `batches/`, `results/`, and the run state file (`.dw-run.json`).

## Workflow Comments

Lines in the workflow starting with `#` are comments. They are displayed in `dw project info` but skipped during `dw project run-all` and excluded from step numbering for `--from`:

```toml
workflow = [
    "dw project setup",
    "# Download the dataset from: https://example.com/data",
    "dw project run prepare",
]
```
