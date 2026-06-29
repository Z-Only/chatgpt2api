# chatgpt2api CLI

This npm package installs only the `chatgpt2api` CLI. It does not include the Tauri desktop app.

```bash
npm install -g chatgpt2api
chatgpt2api --help
chatgpt2api serve
```

During `postinstall`, the package downloads the matching prebuilt CLI archive from GitHub Releases, verifies the SHA256 checksum, and stores the executable in the package cache directory.

If your npm policy blocks lifecycle scripts, reinstall with `npm install -g --allow-scripts=chatgpt2api chatgpt2api`.

Supported release targets:

- macOS arm64 and x64
- Linux x64
- Windows x64

For local release testing, set `CHATGPT2API_RELEASE_BASE_URL` to a directory-style release URL that contains the CLI archive and checksum file.
