# Development Setup

## Prerequisites

- **Node.js** 20+ (`node --version`)
- **Rust** 1.77+ (`rustc --version`) — install via [rustup](https://rustup.rs)
- **Tauri CLI** — installed as a local devDependency (`npm run tauri`)
- **System dependencies** (Linux): `sudo apt install libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev`

## First Run

```sh
git clone https://github.com/Gizele1/stash.git
cd stash
npm install
npm run tauri   # Starts full Tauri app (Vite + Rust)
```

The first Rust compilation takes several minutes. Subsequent runs are fast.

## Frontend Only (faster iteration)

```sh
npm run dev     # Starts Vite dev server at http://localhost:1420
```

Note: Tauri IPC calls (`api.*`) will fail without the backend. Use mock data for pure UI work.

## Running Tests

```sh
npm test                    # Vitest (frontend + architecture boundary test)
cd src-tauri && cargo test  # Rust unit + integration tests
```

## Linting

```sh
npm run lint        # ESLint
npm run typecheck   # TypeScript strict check
cd src-tauri && cargo clippy
```

## Building for Release

```sh
npm run tauri -- build   # Produces platform installer in src-tauri/target/release/bundle/
```
