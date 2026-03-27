# API Keys & Webhooks

## API Keys

Create and manage API keys for programmatic access.

### Create a Key

```bash
dw keys create --name "ci-pipeline"
dw keys create --name "my-agent" --description "Used by the research agent"
```

The key secret is shown once at creation time. Store it securely.

### List Keys

```bash
dw keys list
```

Secrets are masked in the output. Only metadata (name, ID, creation date) is shown.

### Delete a Key

```bash
dw keys delete <key-id>
dw keys delete <key-id> --yes  # Skip confirmation
```

## Webhooks

Webhooks notify your endpoints when batch events occur.

### Create a Webhook

```bash
dw webhooks create --url https://example.com/webhook
```

With specific events:

```bash
dw webhooks create --url https://example.com/webhook --events "batch.completed,batch.failed"
```

With a description:

```bash
dw webhooks create --url https://example.com/webhook --description "Notify Slack on batch completion"
```

The webhook signing secret is shown once at creation time. Use it to verify webhook payloads.

### List Webhooks

```bash
dw webhooks list
```

### Delete a Webhook

```bash
dw webhooks delete <webhook-id>
dw webhooks delete <webhook-id> --yes  # Skip confirmation
```

### Rotate Signing Secret

```bash
dw webhooks rotate-secret <webhook-id>
```

Generates a new signing secret. The old secret stops working immediately.
