# ChatGPT2API

ChatGPT2API turns a ChatGPT subscription into a local OpenAI-compatible API.

Default local API:

- OpenAI-compatible base URL: `http://127.0.0.1:14550/v1`
- Health URL: `http://127.0.0.1:14550/health`

## Run

```bash
bun install
bun run tauri dev
```

CLI:

```bash
bun run cli:dev --help
bun run cli:dev models
bun run cli:dev serve --port 14550
```

Install the packaged CLI with npm:

```bash
npm install -g chatgpt2api
chatgpt2api --help
chatgpt2api serve --port 14550
```

Install the packaged CLI with Homebrew:

```bash
OWNER="$(gh repo view --json owner -q .owner.login)"
brew tap "$OWNER/chatgpt2api"
brew install chatgpt2api
chatgpt2api serve
```

## Endpoints

- `GET /v1/models`
- `POST /v1/responses`
- `POST /v1/chat/completions`
- `POST /v1/completions`
- `POST /v1/images/generations`
- `POST /v1/images/edits`

`/v1/images/variations` returns OpenAI-shaped `501 unsupported` until the upstream image path supports it.

## Configuration

Config path: `~/.chatgpt2api/config.toml`

See [`config.example.toml`](config.example.toml).

Supported sections:

- `server`
- `api`
- `reasoning`
- `text`
- `image`
- `features`
- `ui`

Custom port options:

- GUI settings
- CLI: `chatgpt2api serve --port 18080`
- Config: `server.port = 18080`
- Env: `CHATGPT2API_PORT=18080`

## Security Defaults

- Local-only bind by default.
- No wildcard CORS by default.
- Config stores non-secret settings only.
- Refresh tokens should use the OS keychain; memory-only fallback does not persist tokens.

## Packaging

Desktop release workflow targets macOS DMG, Windows MSI/NSIS, and Linux AppImage/deb/rpm.

CLI distribution supports npm package installs, Homebrew installs, and direct GitHub release binary archives.
