# Installation

## Developer

```bash
bun install
bun run tauri dev
```

CLI development:

```bash
bun run cli:dev --help
bun run cli:dev models
bun run cli:dev serve --port 14550
```

## Desktop Releases

Release workflows produce:

- macOS: DMG
- Windows: MSI and NSIS installers
- Linux: AppImage, deb, rpm

## CLI Releases

CLI install methods:

- npm: `npm install -g chatgpt2api`
- Homebrew:

```bash
OWNER="$(gh repo view --json owner -q .owner.login)"
brew tap "$OWNER/chatgpt2api"
brew install chatgpt2api
chatgpt2api login
chatgpt2api serve
```

- GitHub release binary archives
