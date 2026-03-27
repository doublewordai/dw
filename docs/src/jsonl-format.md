# JSONL Format

Doubleword uses the OpenAI-compatible batch JSONL format. Each line is a JSON object representing one API request.

## Chat Completions

```json
{
  "custom_id": "request-001",
  "method": "POST",
  "url": "/v1/chat/completions",
  "body": {
    "model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "messages": [
      {"role": "system", "content": "You are a helpful assistant."},
      {"role": "user", "content": "What is batch inference?"}
    ],
    "max_tokens": 512,
    "temperature": 0
  }
}
```

## Embeddings

```json
{
  "custom_id": "emb-001",
  "method": "POST",
  "url": "/v1/embeddings",
  "body": {
    "model": "Qwen/Qwen3-Embedding-8B",
    "input": "The quick brown fox"
  }
}
```

## Vision (Multimodal)

```json
{
  "custom_id": "img-001",
  "method": "POST",
  "url": "/v1/chat/completions",
  "body": {
    "model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "messages": [
      {
        "role": "user",
        "content": [
          {"type": "text", "text": "Describe this image."},
          {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AA..."}}
        ]
      }
    ]
  }
}
```

## Required Fields

| Field | Description |
|-------|-------------|
| `custom_id` | Unique identifier for the request. Used to match results. |
| `method` | HTTP method. Always `"POST"`. |
| `url` | API endpoint. `/v1/chat/completions` or `/v1/embeddings`. |
| `body` | The request body, matching the OpenAI API format. |

## Model Field

The `model` field in the body can be:
- Set in the JSONL file directly
- Omitted and set later with `dw files prepare --model <name>`
- Overridden at upload/run time with `--model`

The recommended pattern for reusable batch files is to omit the model and set it with `dw files prepare`:

```bash
dw files prepare batch.jsonl --model Qwen/Qwen3-VL-30B-A3B-Instruct-FP8
```

## Result Format

Results are also JSONL, one line per completed request:

```json
{
  "id": "req-abc123",
  "custom_id": "request-001",
  "response_body": {
    "id": "chatcmpl-xyz",
    "object": "chat.completion",
    "model": "Qwen/Qwen3-VL-30B-A3B-Instruct-FP8",
    "choices": [
      {
        "index": 0,
        "message": {"role": "assistant", "content": "Batch inference is..."},
        "finish_reason": "stop"
      }
    ],
    "usage": {
      "prompt_tokens": 42,
      "completion_tokens": 128,
      "total_tokens": 170
    }
  }
}
```

Match results to requests using the `custom_id` field.
