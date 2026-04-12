<p align="center">
  <h1 align="center">CLI Denoiser</h1>
  <p align="center">
    <strong>Strip terminal noise for LLM agents. Zero false positives.</strong>
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/status-archived-lightgrey" alt="Archived">
    <a href="https://crates.io/crates/cli-denoiser"><img src="https://img.shields.io/crates/v/cli-denoiser.svg?style=flat-square&logo=rust" alt="crates.io"></a>
    <img src="https://img.shields.io/crates/l/cli-denoiser?style=flat-square" alt="License: MIT">
  </p>
</p>

---

> **This project is archived.** It works, but is no longer actively maintained. The crate remains available on [crates.io](https://crates.io/crates/cli-denoiser).

---

## What it was

A Rust-powered filter proxy that removes terminal noise (progress bars, ANSI codes, duplicate lines, verbose build output) from command output before it reaches LLM agents. Built for Claude Code, Codex CLI, and Gemini CLI.

Two-pass pipeline: line-level classification then block-level collapsing. Zero false positives enforced via the type system — uncertain lines always pass through.

## Results

| Metric | Value |
|--------|-------|
| **Overall Savings** | 58.5% token reduction |
| **Avg Latency** | ~1.5ms per filter pass |
| **False Positives** | Zero |

Filters for: git, npm, cargo, docker, kubectl, plus universal ANSI/progress/dedup filters.

## What I built

- **Type-safe filter pipeline** with `Keep`/`Drop`/`Replace`/`Uncertain` — `Uncertain` always preserves the line, making false positives structurally impossible
- **Block collapsing** — 47 `Compiling` lines become `[compiled 47 crates]`
- **Auto-install hooks** for Claude Code, Codex CLI, Gemini CLI
- **Benchmark suite** with signal verification — every scenario has required strings that must survive filtering
- **CLI subcommands**: `gain` (token savings), `report` (daily stats), `log` (event history), `bench` (run benchmarks)

## What I learned

- Designing filter pipelines where correctness is enforced by the type system, not by testing
- Integrating with multiple agentic coding tools via their hook systems
- Publishing and maintaining a Rust crate on crates.io
- Building CI/CD with cross-platform binary releases

---

## Tech stack

| Component | Tech |
|-----------|------|
| Core | Rust, clap, serde |
| Filters | ANSI, progress, dedup, git, npm, cargo, docker, kubectl |
| Distribution | crates.io, GitHub Releases (macOS ARM/x64, Linux x64) |

---

## License

[MIT](LICENSE)
