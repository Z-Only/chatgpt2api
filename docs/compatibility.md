# Compatibility

## Local API

- Default base URL: `http://127.0.0.1:14550/v1`
- Health URL: `http://127.0.0.1:14550/health`
- Supported text endpoints: `/v1/models`, `/v1/responses`, `/v1/chat/completions`, `/v1/completions`
- Supported image endpoints: `/v1/images/generations`, `/v1/images/edits`
- `/v1/images/variations` returns OpenAI-shaped `501 unsupported` until the upstream path supports it.

## Desktop Bundles

- macOS: DMG
- Windows: MSI and NSIS
- Linux: AppImage, deb, rpm

## CLI

The same binary supports GUI launch with no arguments and headless CLI commands with arguments.
