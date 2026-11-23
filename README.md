# Multi-Qirāʾāt Muṣḥaf

A muṣḥaf with fixed-line pages and calligraphic justification that displays any of the 20
riwāyāt of al-Shāṭibiyyah and al-Durrah, with every wajh the books allow as a reader option.
It is a website first; native iOS and Android apps come later.

The project is being rebuilt on a new data model (D67, milestones M0–M6). The current app
(the 8 riwāyāt KFGQPC publishes, on the legacy patch model) is frozen and keeps working until
the new model reproduces each riwāyah.

- `corpus/`: the data, as text tables people review: words, farsh, uṣūl, riwāyah profiles,
  writing styles, ayah counting, layouts (`docs/formats.md`).
- `core/`: the Rust engine: letter features (`mushaf-text`), corpus check and build
  (`mushaf-corpus`), shaping, fixed-point justification and drawing. The same code runs in the
  CLI and, compiled to WebAssembly, in the browser.
- `data/`: pinned sources, Python importers (books, KFGQPC), and the legacy pipeline.
- `apps/web/`: the web app (Vite + TypeScript). It only paints what the core returns.
- `docs/`: the architecture (`ARCHITECTURE.md`), glossary (`GLOSSARY.md`), decisions (`adr/`)
  and known gaps (`known-gaps.md`).

Read `CLAUDE.md` for the project rules and `PROJECT_PLAN.md` for the milestones.

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
   Some sources cannot be downloaded and are copied in by hand (`"acquire": "manual"` in the
   manifest). Without them their tests **skip** rather than fail, so check they are there:

   | File under `data/sources/raw/` | What it is |
   | --- | --- |
   | `books/al-futuh-al-bahira/vol1.pdf` … `vol6.pdf` | al-Futūḥ al-Bāhira, the owner's PDFs (source of truth, D68) |
   | `owner-fonts/majalla.ttf` | Sakkal Majalla, used to verify al-Futūḥ's prose text |
   | `books/al-budur-al-zahirah/al-budur.epub` | the owner's al-Budūr EPUB; missing (G-225), so the pipeline uses Aljam3's copy, which `data-fetch` downloads |

   `just data-fetch` then reports `verify: N/N sources ok`; a manual file that is present but
   different from its pin is reported, never used.
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
  tests. Install cargo-nextest (`cargo install --locked cargo-nextest`) so the Rust tests run
  in parallel. The Rust tests read the data from `build/mushaf.db` while it is current
  (after `just data-build`), and the few whole-corpus tests check a sample of riwāyāt.
- `just gate`: the full gate. It runs every riwāyah (`MUSHAF_FULL=1`) and adds all platform
  targets, the golden pages, the round-trip against KFGQPC and the web checks
  (`just web-gate`). It prints each step's time and keeps logs in `build/gate/`. Run it
  before every merge to `main`: GitHub Actions CI is switched off (2026-11-23), so this is
  the only full check.
- `just data-build` skips itself when nothing under `data/`, `core/`, `fonts/` or
  `docs/review/` changed since its last successful run; `FORCE=1 just data-build` rebuilds.

Other web checks, each also listed in `CLAUDE.md`:
- `just web-parity`: the WebAssembly engine draws every golden page byte-identically.
- `just web-test`: Playwright tests of every screen.
- `just web-golden`: the app's drawing compared with the golden PNGs.
- `just web-bench`: the 16 ms page-turn budget, with Chromium's CPU slowed 4×.

## Book extractions (untracked)

The book importers write to `build/books/`, which is never committed because it holds book
text (rule 10). Some tests need it and skip without it. To rebuild what they use:
```bash
just book-futuh --vol 1 --pages 47-63 --surah 1 --tag m0
```
That is al-Futūḥ for al-Fātiḥah and al-Baqarah 1–20, which the Warsh prototype
(`python -m pipeline.m0_warsh`, `docs/review/m0/warsh-residuals.md`) reads. A whole volume
takes about 3 minutes: `just book-futuh --vol 1`.

## Note on the Rust build cache

The justfile shares one Rust build directory between the main checkout and every worktree
(`<repo>/core/target`). If another checkout builds at the same time, a worktree can fail with
errors like "cannot find function … in crate mushaf_model". To avoid that, give the worktree its
own build directory:
```bash
export CARGO_TARGET_DIR=$PWD/build/cargo-target
```
A worktree also has an empty `data/sources/raw/` (the sources are untracked). Link the main
checkout's folders into it, or its source-dependent tests skip:
```bash
for d in /path/to/main/data/sources/raw/*/; do ln -s "$d" data/sources/raw/; done
```
