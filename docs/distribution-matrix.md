# Distribution Matrix

## GUI Desktop

- macOS: DMG
- Windows: MSI / NSIS
- Linux: AppImage / deb / rpm

## CLI

- npm: `npm install -g chatgpt2api`
- Homebrew: `brew install chatgpt2api`
- GitHub release: direct binary archive

## Developer

- `bun install`
- `bun run tauri dev`
- `cargo run --manifest-path src-tauri/Cargo.toml --bin chatgpt2api -- serve`

## Release Verification

By default, the verifier expects release assets in `release/`, the npm package metadata at `npm/chatgpt2api/package.json`, and the generated Homebrew formula at `packaging/homebrew/chatgpt2api.rb`. Use the script flags to point at downloaded release assets or a tap checkout when verifying elsewhere.

```bash
bun run scripts/verify-release-artifacts.ts --tag v0.1.0
chatgpt2api --help
chatgpt2api models
```
