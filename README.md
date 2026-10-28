# Multi-Qirāʾāt Muṣḥaf

A muṣḥaf with fixed-line pages, qirāʾāt that can be changed at runtime, and calligraphic
justification. It is a website first; native iOS and Android apps come later.
Stage 1 covers the 8 riwāyāt for which KFGQPC publishes full text (`hafs`, `shuba`, `warsh`,
`qalun`, `bazzi`, `qunbul`, `duri_abuamr`, `susi`), in the 15-line Madinah ʿUthmānī script.

- `core/`: the Rust engine (resolver, rules, shaping, fixed-point justification, drawing). The
  same code runs in the CLI and, compiled to WebAssembly, in the browser.
- `data/`: the Python data pipeline (`uv`). It builds the spine, layouts, patches and rules
  from pinned sources.
- `apps/web/`: the web app (Vite + TypeScript). It only paints what the core returns.
- `docs/`: decisions (`decisions.md`) and known gaps (`known-gaps.md`).

Read `CLAUDE.md` for the project rules and `PROJECT_PLAN.md` for the phases.

## Prerequisites

macOS with Homebrew was used. You need:

- **Rust through rustup.** The toolchain is pinned in `rust-toolchain.toml`. Homebrew's rustup
  is keg-only, so put it on `PATH` first:
  ```bash
  export PATH=/opt/homebrew/opt/rustup/bin:$PATH
  ```
  Add the WebAssembly target. The mobile targets are only needed for `just core-targets`:
  ```bash
  rustup target add wasm32-unknown-unknown aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android x86_64-linux-android
  ```
- **wasm-bindgen-cli 0.2.129.** It must match the `wasm-bindgen` version in
  `core/mushaf-wasm/Cargo.toml`. It installs to `~/.cargo/bin`, which must be on `PATH`:
  ```bash
  cargo install --locked wasm-bindgen-cli --version 0.2.129
  export PATH=$HOME/.cargo/bin:$PATH
  ```
- **uv** with Python 3.12, for the data pipeline (`brew install uv`).
- **just**, the command runner (`brew install just`).
- **Node.js** 24 with npm, for the web app. Run this once to install its packages and
  Playwright's Chromium:
  ```bash
  just web-setup
  ```

## Build and run

Run everything from the repository root.

1. **Fetch the pinned sources.** They are downloaded and checked against
   `data/sources/MANIFEST.json`, and are never committed:
   ```bash
   just data-fetch
   ```
2. **Build the data.** This runs the full pipeline and writes `build/mushaf.db`:
   ```bash
   just data-build
   ```
3. **Test the core:**
   ```bash
   just core-test
   ```
4. **Build the web app's inputs.** These are the core as WebAssembly (`build/web/pkg`) and the
   static data bundle (`build/web/data/mushaf.bundle`):
   ```bash
   just web-wasm
   ```
   ```bash
   just web-data
   ```
5. **Run the site locally.** This builds both inputs and starts the dev server:
   ```bash
   just web-dev
   ```
   Then open **http://127.0.0.1:5173**. The first load parses the data, which takes a few
   seconds. After that the site works offline.

To build the site as plain static files in `apps/web/dist` (the app, the WebAssembly and the
data):
```bash
just web-build
```
To serve that build locally at **http://127.0.0.1:4173**:
```bash
npm --prefix apps/web run preview
```
### Test deploy on Netlify

The site is plain static files, so it can be deployed as they are. Build it from an up-to-date
`main`, then deploy the `dist` folder with the Netlify CLI. The CLI asks you to log in and to
choose or create a site the first time:
```bash
just web-build
```
```bash
npx netlify-cli deploy --dir=apps/web/dist --prod
```
Turning on password protection for a test site is recommended. It is in the Netlify site
settings, under Access control.

Free edits are saved in each browser (`localStorage`, per muṣḥaf) and restored on the next
visit, with the "edited" marker. Nothing is sent to the server.

To draw a single page with the reference renderer (writes SVG, PNG and JSON to
`build/render/<riwayah>/`):
```bash
just render PAGE=1 RIWAYAH=hafs
```

## Checks

- `just gate-fast`: run before every commit. It covers lint, the Rust tests and the Python
  tests.
- `just gate`: the full gate, run once per phase before its final push. It adds all platform
  targets, the golden pages, the round-trip against KFGQPC and the web checks
  (`just web-gate`). It prints each step's time and keeps logs in `build/gate/`.

Other web checks, each also listed in `CLAUDE.md`:
- `just web-parity`: the WebAssembly engine draws every golden page byte-identically.
- `just web-test`: Playwright tests of every screen.
- `just web-golden`: the app's drawing compared with the golden PNGs.
- `just web-bench`: the 16 ms page-turn budget, with Chromium's CPU slowed 4×.

## Note on the Rust build cache

The justfile shares one Rust build directory between the main checkout and every worktree
(`<repo>/core/target`). If another checkout builds at the same time, a worktree can fail with
errors like "cannot find function … in crate mushaf_model". To avoid that, give the worktree its
own build directory:
```bash
export CARGO_TARGET_DIR=$PWD/build/cargo-target
```
