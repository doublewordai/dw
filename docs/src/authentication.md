# Authentication

## Browser Login (recommended)

```bash
dw login
```

Opens your browser to the Doubleword SSO page. After authenticating, the CLI receives API keys for both the inference and admin APIs. Credentials are stored in `~/.dw/credentials.toml` with 0600 permissions.

### Logging into an Organization

```bash
dw login --org my-org
```

When you log in with `--org`, the CLI creates credentials scoped to that organization. Batches, files, and usage are billed to the org.

### Custom Account Name

```bash
dw login --as work
```

By default the account is named after your email or org. Use `--as` to set a custom name.

## API Key Login (headless)

For SSH sessions, containers, and CI:

```bash
dw login --api-key <YOUR_INFERENCE_KEY>
```

API key login stores only the inference key. Commands that use the admin API will not be available, including `dw whoami`, `dw usage`, `dw requests`, `dw keys`, and `dw webhooks`. For full functionality, use browser login (with port forwarding for remote sessions).

## Credentials Storage

Credentials are stored in `~/.dw/credentials.toml`:

```
~/.dw/
├── config.toml        # Active account, server URLs
└── credentials.toml   # API keys (0600 permissions)
```

## Logging Out

```bash
# Log out of the active account
dw logout

# Log out of a specific account
dw logout work

# Log out of all accounts
dw logout --all
```

## Checking Your Identity

```bash
dw whoami
```

Shows the authenticated user and active organization (if any).
