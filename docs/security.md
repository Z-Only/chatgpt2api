# Security

ChatGPT2API is local-first by default.

- The server binds to `127.0.0.1:14550` by default.
- `0.0.0.0` is rejected unless `server.allow_external_bind = true`.
- Default CORS is local-only and never wildcard.
- `~/.chatgpt2api/config.toml` stores non-secret settings only.
- Refresh tokens are intended for the OS keychain; if unavailable, they stay memory-only.
- Do not put API keys, access tokens, refresh tokens, or id tokens in `config.toml`.

Config file permissions are set to user-only where the platform supports it.
