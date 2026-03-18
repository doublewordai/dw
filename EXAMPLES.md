# DW CLI — Workflow Examples

Practical workflows you can run entirely from the command line. Each example builds on real use cases and demonstrates how `dw` commands compose together.

---

## 1. Model Evaluation: Compare Models on the Same Prompts

You have a set of prompts and want to find out which model gives the best answers. Run the same JSONL against multiple models, then group results by prompt for side-by-side comparison.

### Create the evaluation set

```bash
cat > eval.jsonl << 'JSONL'
{"custom_id": "math-001", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "PLACEHOLDER", "messages": [{"role": "user", "content": "What is 127 * 34? Show your working."}], "max_tokens": 300}}
{"custom_id": "code-001", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "PLACEHOLDER", "messages": [{"role": "user", "content": "Write a Python function that finds the longest palindromic substring in a string. Include comments."}], "max_tokens": 500}}
{"custom_id": "reason-001", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "PLACEHOLDER", "messages": [{"role": "user", "content": "A farmer has 17 sheep. All but 9 die. How many are left? Explain your reasoning step by step."}], "max_tokens": 300}}
JSONL
```

### Check what models are available

```bash
dw models list
```

### Submit to multiple models

```bash
# Pick your models
MODELS=("Qwen/Qwen3-VL-30B-A3B-Instruct-FP8" "Qwen/Qwen3-VL-235B-A22B-Instruct-FP8")

# Submit each and collect batch IDs
for model in "${MODELS[@]}"; do
    # Create a safe filename from the model name
    slug=$(echo "$model" | tr '/' '-' | tr '[:upper:]' '[:lower:]')

    echo "Submitting to $model..."
    dw stream eval.jsonl --model "$model" > "results-${slug}.jsonl"
    echo "Results written to results-${slug}.jsonl"
done
```

Each `dw stream` call overrides the model, uploads, runs the batch, waits for completion, and writes results to a separate file.

### Group results by prompt for comparison

Now the interesting part — reorganize the results so each prompt's responses are grouped together:

```bash
#!/bin/bash
# compare.sh — Group batch results by custom_id across model runs

mkdir -p comparison

# Get all unique custom_ids from the first results file
first_file=$(ls results-*.jsonl | head -1)
custom_ids=$(jq -r '.custom_id' "$first_file" | sort)

for id in $custom_ids; do
    outfile="comparison/${id}.md"
    echo "# ${id}" > "$outfile"
    echo "" >> "$outfile"

    # Extract the original prompt
    prompt=$(jq -r "select(.custom_id == \"$id\") | .response.body.choices[0].message.content // \"(no response)\"" "$first_file" | head -c 0)
    original_prompt=$(jq -r "select(.custom_id == \"$id\")" eval.jsonl | jq -r '.body.messages[-1].content')
    echo "**Prompt:** $original_prompt" >> "$outfile"
    echo "" >> "$outfile"

    for results_file in results-*.jsonl; do
        # Extract model name from filename
        model_slug=$(basename "$results_file" .jsonl | sed 's/results-//')

        # Extract this prompt's response
        response=$(jq -r "select(.custom_id == \"$id\") | .response.body.choices[0].message.content // \"(no response)\"" "$results_file")
        tokens=$(jq -r "select(.custom_id == \"$id\") | .response.body.usage | \"\(.prompt_tokens) in / \(.completion_tokens) out\"" "$results_file")

        echo "## $model_slug" >> "$outfile"
        echo "Tokens: $tokens" >> "$outfile"
        echo "" >> "$outfile"
        echo "$response" >> "$outfile"
        echo "" >> "$outfile"
        echo "---" >> "$outfile"
        echo "" >> "$outfile"
    done

    echo "Written: $outfile"
done

echo ""
echo "Comparison files in comparison/:"
ls comparison/
```

Run it:

```bash
chmod +x compare.sh
./compare.sh
```

This gives you one markdown file per prompt, with each model's response and token usage side by side. Point an AI assistant at the `comparison/` directory to help you analyse the results.

### With cost awareness

Check costs before committing:

```bash
# Upload without running, then estimate cost per model
for model in "${MODELS[@]}"; do
    slug=$(echo "$model" | tr '/' '-' | tr '[:upper:]' '[:lower:]')

    # Prepare with model override, upload, check cost
    dw files prepare eval.jsonl --model "$model" --output "/tmp/eval-${slug}.jsonl"
    file_id=$(dw files upload "/tmp/eval-${slug}.jsonl" --output json | jq -r '.id')
    echo "$model:"
    dw files cost-estimate "$file_id"
    echo ""
done
```

---

## 2. Data Processing Pipeline: Clean and Enrich a CSV

You have a messy CSV of company records and want to use an LLM to normalize names, classify industries, and extract structured data.

### Convert CSV to JSONL

```bash
# Sample CSV
cat > companies.csv << 'CSV'
name,description
"acme corp","makes stuff for roadrunners, based in arizona"
"GLOBEX Inc.","Hank Scorpio's company. Does something with hammocks??"
"initech","software company from office space. TPS reports."
"Stark Industries","Defense contractor turned clean energy. Tony's company."
"Umbrella Corp","Pharmaceutical and biotech research company"
CSV

# Convert to batch JSONL (skip header)
tail -n +2 companies.csv | nl -ba -w1 | while IFS=$'\t' read -r num line; do
    name=$(echo "$line" | cut -d',' -f1 | tr -d '"')
    desc=$(echo "$line" | cut -d',' -f2- | tr -d '"')
    cat << JSONL
{"custom_id": "company-$(printf '%03d' $num)", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "PLACEHOLDER", "messages": [{"role": "system", "content": "Extract structured data from the company description. Return JSON with fields: canonical_name (properly capitalized), industry (one of: Technology, Defense, Pharmaceuticals, Consumer Goods, Other), is_fictional (boolean), source_material (string or null)."}, {"role": "user", "content": "Company: $name\nDescription: $desc"}], "max_tokens": 200, "response_format": {"type": "json_object"}}}
JSONL
done > companies-batch.jsonl

# Validate
dw files validate companies-batch.jsonl
```

### Run and collect results

```bash
# Stream results, using the cheapest model
dw stream companies-batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 > companies-results.jsonl
```

### Extract structured output

```bash
# Parse the LLM responses into a clean JSON array
jq -s '[.[] | {
    custom_id: .custom_id,
    data: (.response.body.choices[0].message.content | fromjson)
}]' companies-results.jsonl > companies-structured.json

# Pretty print
jq '.' companies-structured.json

# Convert to CSV
jq -r '.[] | [.data.canonical_name, .data.industry, .data.is_fictional, .data.source_material] | @csv' companies-structured.json
```

### Stage 2: Enrich with follow-up batch

Use the structured output to generate follow-up prompts:

```bash
# Generate enrichment prompts from stage 1 results
jq -r '.[] | @json' companies-structured.json | nl -ba -w1 | while IFS=$'\t' read -r num line; do
    name=$(echo "$line" | jq -r '.data.canonical_name')
    industry=$(echo "$line" | jq -r '.data.industry')
    cat << JSONL
{"custom_id": "enrich-$(printf '%03d' $num)", "method": "POST", "url": "/v1/chat/completions", "body": {"model": "PLACEHOLDER", "messages": [{"role": "system", "content": "You are a business analyst. Given a company name and industry, provide a brief competitive analysis. Return JSON with: competitors (array of 3 strings), market_position (string), and risk_factors (array of 2 strings)."}, {"role": "user", "content": "Company: $name\nIndustry: $industry"}], "max_tokens": 300, "response_format": {"type": "json_object"}}}
JSONL
done > companies-enrich.jsonl

# Stream stage 2
dw stream companies-enrich.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 > companies-enriched.jsonl
```

This is a two-stage pipeline: classify first, then enrich based on classifications. Each stage is a single `dw stream` command.

---

## 3. Rapid Prototyping: Test a Prompt Before Scaling

Before committing to a 10,000-row batch, test your prompt on a few examples with real-time inference.

### Iterate on the prompt

```bash
# Try a prompt
dw realtime Qwen3-30B "Extract the email and phone number from this text: 'Call John at 555-0123 or email john@example.com'" \
  --system "Return JSON with fields: email, phone. If not found, use null."

# Tweak temperature
dw realtime Qwen3-30B "Write 3 creative names for a coffee shop" --temperature 0.9
dw realtime Qwen3-30B "Write 3 creative names for a coffee shop" --temperature 0.2

# Try different models
dw realtime Qwen3-30B "Explain the Monty Hall problem" --max-tokens 200
dw realtime Qwen3-235B "Explain the Monty Hall problem" --max-tokens 200
```

### Lock in the prompt and scale up

Once you're happy with the prompt, generate a JSONL file from your data:

```bash
# Generate batch file from a list of inputs
cat inputs.txt | nl -ba -w1 | while IFS=$'\t' read -r num line; do
    echo "{\"custom_id\": \"req-$(printf '%05d' $num)\", \"method\": \"POST\", \"url\": \"/v1/chat/completions\", \"body\": {\"model\": \"PLACEHOLDER\", \"messages\": [{\"role\": \"system\", \"content\": \"Extract the email and phone number. Return JSON with fields: email, phone.\"}, {\"role\": \"user\", \"content\": $(echo "$line" | jq -Rs .)}], \"max_tokens\": 100, \"response_format\": {\"type\": \"json_object\"}}}"
done > extraction-batch.jsonl

# Check cost before submitting
dw files upload extraction-batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 --output json \
  | jq -r '.id' \
  | xargs dw files cost-estimate

# Submit
dw stream extraction-batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 > extraction-results.jsonl
```

---

## 4. Batch Directory: Process Multiple Files at Once

When your data is split across files (e.g. one per day, per source, or per category):

```bash
mkdir -p daily-batches/

# Imagine these are generated by your pipeline
cp batch-monday.jsonl daily-batches/
cp batch-tuesday.jsonl daily-batches/
cp batch-wednesday.jsonl daily-batches/

# Override model and submit all files
dw batches run daily-batches/ --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8

# Or stream all and concatenate results
dw stream daily-batches/ > weekly-results.jsonl
```

Each file becomes a separate batch. `dw stream` processes them sequentially, concatenating all results to stdout.

---

## 5. Agent Workflow: AI Assistant Builds and Runs a Batch

This is the workflow when an AI coding assistant (Claude Code, Cursor, etc.) uses `dw` as a tool.

The agent would:

1. **Clone an example** to understand the JSONL format:
```bash
dw examples clone structured-extraction
# Agent reads structured-extraction/src/ and sample files for context
```

2. **Generate a JSONL file** from the user's data (agent writes code for this step).

3. **Validate the file**:
```bash
dw files validate generated-batch.jsonl
```

4. **Check cost**:
```bash
file_id=$(dw files upload generated-batch.jsonl --output json | jq -r '.id')
dw files cost-estimate "$file_id"
```

5. **Run and collect results**:
```bash
dw stream generated-batch.jsonl > results.jsonl
```

6. **Process results** (agent writes code to parse the JSONL output).

The entire flow works without a browser, without interactive prompts, and with machine-readable JSON output. The agent just needs a `DOUBLEWORD_API_KEY` environment variable or a prior `dw login --api-key`.

---

## 6. Webhook-Driven Pipeline

For production workflows where you submit batches and get notified on completion, rather than polling.

```bash
# Set up a webhook
dw webhooks create --url https://your-app.com/api/batch-complete --events batch.completed

# Submit batches (no --watch, returns immediately)
dw batches run batch1.jsonl
dw batches run batch2.jsonl
dw batches run batch3.jsonl

# Your webhook receives a POST when each batch completes:
# {
#   "event": "batch.completed",
#   "batch_id": "batch_abc123",
#   "timestamp": "2026-03-18T12:00:00Z"
# }

# Your backend then calls:
# dw batches results batch_abc123 -o /data/results/batch_abc123.jsonl
```

---

## Tips

**Pipe composition**: stdout is always clean data, stderr has progress and status messages.
```bash
# These all work naturally
dw stream batch.jsonl > results.jsonl          # Progress visible, results to file
dw stream batch.jsonl | jq '.response'         # Process results inline
dw realtime Qwen3-30B "Hello" | pbcopy         # Copy response to clipboard
dw batches results batch_123 | wc -l           # Count results
```

**JSON output for scripting**: When piped, output auto-switches to JSON. Force it explicitly with `--output json`.
```bash
# Get a file ID programmatically
file_id=$(dw files upload batch.jsonl --output json | jq -r '.id')

# Get batch status
status=$(dw batches get batch_123 --output json | jq -r '.status')
```

**Quick model override**: The `--model` flag on `upload`, `run`, and `stream` rewrites every line's `body.model` before upload. Use `PLACEHOLDER` as the model in your template JSONL and override at submission time.
```bash
dw stream template.jsonl --model Qwen3-30B    # Today
dw stream template.jsonl --model Qwen3-235B   # Tomorrow, same file
```

**Cost check before commit**: Always estimate before large batches.
```bash
dw files upload big-batch.jsonl --output json | jq -r '.id' | xargs dw files cost-estimate
```
