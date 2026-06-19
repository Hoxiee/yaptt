# PTT GUI

Tauri + Vue desktop app for configuring the PTT daemon.

## Build

```bash
# From the gui/ directory:
npm install
npm run tauri build
```

Requires Rust, Node.js, and Tauri system dependencies (webkit2gtk, etc.).

## Features

- Toggle PTT on/off
- Configure PTT key and remap key
- View daemon status and PID
- Apply config changes (requires daemon restart)

## Development

```bash
npm run tauri dev
```
