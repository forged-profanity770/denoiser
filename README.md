# Denoiser

Strip terminal-output noise before feeding it to an LLM agent. Zero false positives - when the filter isn't sure, the line passes through unchanged.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

> Open to contributors. The core pipeline is production-ready; most of the work left is adding filters for more commands (pip, yarn, terraform, ansible) and Windows testing.

## Why this exists

An agent shouldn't waste tokens on 400 lines of "Compiling foo v0.1.2..." or five identical "Downloading..." progress bars. I wanted a filter that sat in front of `cargo`, `npm`, `git`, `docker`, `kubectl` and stripped the noise without ever dropping a signal the agent actually needed (errors, warnings, the first line of a diff, etc.).

Denoiser is that filter. It works as a wrapper (`denoiser cargo build`), as a pipe (`cargo build |& denoiser filter --command cargo`), or as a hook (auto-installed into Claude Code / Codex / Gemini agent configs).

## What it does

Two-pass pipeline (`src/pipeline.rs`):

- **Pass 1** - per-line verdict: `Keep` / `Drop` / `Replace` / `Uncertain`. `Uncertain` passes through.
- **Pass 2** - collapse runs (e.g. 50 `Downloading crate foo` lines → `[compiled 50 crates]`).

Universal filters (always applied): ANSI escape codes (`src/filters/ansi.rs`), progress bars and spinners (`src/filters/progress.rs`), 3+ identical consecutive line dedup (`src/filters/dedup.rs`).

Command-specific filters, selected by `CommandKind::detect()` on binary name:

- `cargo` - per-crate Compiling / Checking / Fresh / Downloading, Locking / Updating
- `npm` - deprecation warnings, timing logs, HTTP fetch, peer resolution
- `git` - transfer stats, object counting / enumerating / compressing
- `docker` - layer cache hits, intermediate container IDs
- `kubectl` - event timestamps, "Watching for changes"
- generic fallback - strips decorative borders only

## Usage

Wrap a command:

```sh
denoiser cargo build            # filtered stdout + stderr, real exit code
denoiser git push origin main
```

Pipe into it:

```sh
docker build . 2>&1 | denoiser filter --command docker
```

Install as agent hooks (auto-detects Claude, Codex, Gemini):

```sh
denoiser install
denoiser uninstall
```

See your token savings:

```sh
denoiser gain --days 30         # total savings (SQLite tracker)
denoiser report --days 7        # daily trend
denoiser log --limit 20         # recent events
denoiser bench                  # verify zero false positives across 11 scenarios
```

All of those accept `--json`.

## Build

```sh
cargo build --release
# binary at target/release/cli-denoiser

cargo clippy -- -D warnings
cargo test
cargo fmt --check
```

`.githooks/pre-commit` enforces all four plus a ban on new `.unwrap()` / `.expect()` in production code.

## Help Wanted

1. **pip / Python filter** (medium). Create `src/filters/pip.rs`, add `CommandKind::Python` detection in `src/filters/mod.rs:53-66`, add a test scenario to `src/bench/corpus.rs`.
2. **yarn / pnpm filter** (medium). Consolidate with npm into `src/filters/nodejs.rs`.
3. **Terraform / Ansible filters** (medium). Terraform: drop `Still creating... [10s elapsed]`. Ansible: collapse repetitive task output. New files `src/filters/terraform.rs`, `src/filters/ansible.rs`.
4. **Better token estimation** (easy). `pipeline.rs:107` uses `char_count / 4`. Swap for tiktoken or byte-pair encoding.
5. **Windows testing + hook paths** (easy-medium). Claude config on Windows lives at `%APPDATA%\Claude\...`. Verify path handling in `src/hooks/claude.rs`.

## Contributing

- Conventional Commits
- No new `.unwrap()` / `.expect()` in prod code (unless annotated `// allow-unwrap: <reason>`)
- A filter must never drop a required signal - every new filter adds a scenario to `src/bench/corpus.rs` listing the signals that must survive

## License

[MIT](LICENSE).
