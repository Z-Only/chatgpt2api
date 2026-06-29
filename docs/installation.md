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

Planned CLI install methods:

- npm: `npm install -g chatgpt2api`
- Homebrew: `brew install chatgpt2api`
- GitHub release binary archives
