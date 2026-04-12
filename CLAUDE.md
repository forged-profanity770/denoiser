# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

CLI Denoiser is a Rust CLI that strips terminal noise from command output before it reaches LLM agents. It targets zero false positives — when uncertain, it passes lines through unchanged. Published to crates.io, builds for Linux and macOS (x86_64 + aarch64).

## Commands

```bash
# Verification gates (CI runs all four)
cargo clippy -- -D warnings
cargo test
cargo fmt --check
cargo check

# Run a single test
cargo test <test_name>               # e.g. cargo test drops_compiling_lines
cargo test <module>::tests           # e.g. cargo test filters::git::tests

# Run benchmarks (verifies zero false positives across 11 scenarios)
cargo run -- bench

# Build release binary
cargo build --release
```

The pre-commit hook (`.githooks/pre-commit`) runs all four gates plus a check that staged `.rs` files don't increase `.unwrap()`/`.expect()` count in production code. Use `// allow-unwrap: <reason>` on the same line to suppress. Skip tests with `PRE_COMMIT_SKIP_TESTS=1`.

## Architecture

**Two-pass filter pipeline** (`Pipeline::process` in `src/pipeline.rs`):
1. **Line pass** — each filter's `filter_line()` returns `Keep | Drop | Replace(String) | Uncertain`. `Uncertain` always passes the line through (this is the zero-false-positive guarantee, enforced by the type system).
2. **Block pass** — each filter's `filter_block()` collapses consecutive noise lines into summaries (e.g. `[compiled 47 crates]`, `[pulled 12 layers]`).

**Pipeline construction** (`build_pipeline` in `src/lib.rs`): single shared function used by both the CLI and benchmark runner. Takes `CommandKind` + debug flag, wires up universal filters (ANSI, Progress, Dedup) then the command-specific filter.

**Filter ordering**:
1. `AnsiFilter` — strips escape codes (always first, purely signal-preserving)
2. `ProgressFilter` — drops progress bars/spinners
3. `DedupFilter` — collapses 3+ consecutive identical lines
4. Command-specific filter — selected by `CommandKind::detect()` which matches on binary name

**Command detection** (`CommandKind::detect` in `src/filters/mod.rs`): extracts the base binary name from the command string, maps to `Git | Npm | Cargo | Docker | Kubectl | Unknown`. The `Unknown` variant uses `GenericFilter` which only strips decorative borders.

**Execution modes** (`src/main.rs`):
- **Wrapper** (`cli-denoiser cargo build`) — spawns the command via `stream::run_filtered`, captures stdout/stderr, filters both through the pipeline
- **Pipe** (`cmd | cli-denoiser filter --command cargo`) — reads stdin, filters, writes stdout
- **Hook** (`--hook-mode`, hidden flag) — same as pipe but no savings message on stderr; used by agent hooks

**Token tracking** (`src/tracker/`): SQLite database at `$XDG_DATA_HOME/cli-denoiser/cli-denoiser.db`, 90-day retention. Records per-event savings. Queried by `gain`, `report`, and `log` subcommands. All three support `--json` for structured output.

**Agent hooks** (`src/hooks/`): Common install/uninstall logic lives in `mod.rs` (`generic_install`/`generic_uninstall`). Each agent module (claude, codex, gemini) provides an `AgentHookConfig` with its config path, JSON key names, and hook entry shapes. Uses `"cli-denoiser"` as a marker string for idempotent operations.

**Benchmarks** (`src/bench/`): `corpus.rs` defines 11 scenarios with hardcoded realistic terminal output and `required_signals` — strings that must survive filtering. The benchmark runner verifies all signals are preserved, catching any false positive regressions.

## Key Constraints

- **Zero false positives is non-negotiable.** If a filter can't confidently classify a line, return `FilterResult::Uncertain`. The pipeline treats `Uncertain` identically to `Keep`.
- **No `.unwrap()` or `.expect()` in production code.** Use `?` propagation or `thiserror`. The pre-commit hook enforces this.
- Clippy is set to `all = "deny"` and `pedantic = "deny"` in `Cargo.toml`.
- Rust edition 2024.

## Adding a New Filter

1. Create `src/filters/<command>.rs` implementing the `Filter` trait
2. Add a `CommandKind` variant to `src/filters/mod.rs`
3. Add binary name matching in `CommandKind::detect()`
4. Wire it into `build_pipeline()` in `src/lib.rs`
5. Add benchmark scenarios in `src/bench/corpus.rs` with `required_signals`
6. Run `cargo run -- bench` to verify zero false positives
