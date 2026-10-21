# Toolchain

Pinned versions. Update this file in the same commit as any toolchain change.

| Tool | Version | Pinned in |
|---|---|---|
| Rust | 1.99.1 | `rust-toolchain.toml` |
| Python | 3.12.15 | `data/pyproject.toml` (`==3.12.*`), `data/uv.lock` |
| uv | 0.13.4 | — |
| just | 1.59.0 | — |
| HarfBuzz (`hb-shape`, spikes only) | 14.3.0 | — |

## Local setup (macOS)

```
brew install rustup uv just
rustup default stable        # rust-toolchain.toml then selects the pinned version
uv python install 3.12
```

Homebrew's `rustup` is keg-only: add `/opt/homebrew/opt/rustup/bin` to `PATH`.
