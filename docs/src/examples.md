# Examples

Doubleword provides a set of real-world use-case examples that demonstrate different batch inference patterns.

## Listing Examples

```bash
dw examples list
```

## Cloning an Example

```bash
dw examples clone model-evals
cd model-evals
dw project setup
dw project info
```

Each example includes a `dw.toml` manifest, Python code, and a README with full instructions.

## Available Examples

### Model Evals

**Evaluate LLM accuracy at batch pricing.** Runs the full GSM8K test set and scores model answers against ground truth.

```bash
dw examples clone model-evals
```

### Embeddings

**Batch embeddings for semantic search.** Downloads Wikipedia abstracts, generates embeddings, and builds a searchable HNSW index.

```bash
dw examples clone embeddings
```

### Synthetic Data Generation

**Generate training data at scale.** Three-stage pipeline: scenario generation, conversation generation, and quality filtering.

```bash
dw examples clone synthetic-data-generation
```

### Data Processing Pipelines

**Clean and enrich company records.** Normalize names, deduplicate, and classify industries using real SEC EDGAR data.

```bash
dw examples clone data-processing-pipelines
```

### Image Summarization

**Vision-language batch inference.** Fetches images from Unsplash, encodes them, and generates social media-style summaries.

```bash
dw examples clone image-summarization
```

### Structured Extraction

**Extract fields from scanned documents.** Receipt data extraction with ensemble voting on the SROIE dataset.

```bash
dw examples clone structured-extraction
```

### Bug Detection Ensemble

**Classify security vulnerabilities.** CWE classification on the CVEfixes dataset with calibration via running twice.

```bash
dw examples clone bug-detection-ensemble
```

### Dataset Compilation

**Compile exhaustive datasets with LLM + search.** Recursive query expansion, web search, extraction, and filtering.

```bash
dw examples clone dataset-compilation
```

### Async Agents

**Deep research with recursive multi-agent orchestration.** A root agent spawns sub-agents that independently search the web and synthesize findings.

```bash
dw examples clone async-agents
```

## Running an Example

Every example follows the same pattern:

```bash
dw examples clone <name>
cd <name>
dw project setup       # Install dependencies
dw project info        # See the workflow
dw project run-all     # Run everything
```

Or run steps individually — see `dw project info` for the full workflow.
