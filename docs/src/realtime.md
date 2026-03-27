# Real-Time Inference

`dw realtime` sends a single inference request and streams the response. Useful for quick tests, prototyping prompts, and interactive use.

## Basic Usage

```bash
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "Explain batch inference in one paragraph"
```

The response streams token-by-token to stdout.

## System Message

```bash
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "Summarize this text" \
  --system "You are a concise technical writer."
```

## Reading from Stdin

When no prompt is given, `dw realtime` reads from stdin:

```bash
echo "What is 2+2?" | dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8
```

```bash
cat document.txt | dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 --system "Summarize this"
```

## Options

| Flag | Description |
|------|-------------|
| `--system <MSG>` | Set the system message |
| `--max-tokens <N>` | Maximum tokens to generate |
| `--temperature <T>` | Sampling temperature (0.0-2.0) |
| `--no-stream` | Wait for full response instead of streaming |
| `--output <FILE>` | Write response to a file |
| `--usage` | Print token usage summary after completion |

## Non-Streaming Mode

```bash
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "Hello" --no-stream
```

Waits for the complete response before printing. Useful when you need the full text at once (e.g., for piping to `jq`).

## Token Usage

```bash
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "Hello" --usage
```

Prints input/output token counts to stderr after the response completes.

## Output to File

```bash
dw realtime Qwen/Qwen3-VL-30B-A3B-Instruct-FP8 "Write a poem" -o poem.txt
```
