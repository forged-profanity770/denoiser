# CLI Denoiser

**Strip terminal noise for LLM agents. Zero false positives.**

CLI Denoiser is a Rust-powered filter proxy that removes terminal noise from command output before it reaches your LLM agent. It saves tokens, reduces context pollution, and guarantees zero false positives -- signal always passes through.

Built for [Claude Code](https://docs.anthropic.com/en/docs/claude-code), [Codex CLI](https://github.com/openai/codex), [Gemini CLI](https://github.com/google-gemini/gemini-cli), and any agentic coding workflow.

## Benchmark Results

<p align="center">
  <img src="https://raw.githubusercontent.com/Orellius/cli-denoiser/main/charts/bench-overview.png" alt="CLI Denoiser Benchmark Overview" width="100%">
</p>

### Overall Performance

| Metric | Value |
|--------|-------|
| **Overall Savings** | 58.5% |
| **Tokens Saved** | 912 across 11 scenarios |
| **Avg Latency** | ~1.5ms per filter pass |
| **False Positives** | **ZERO** |

### Scenario Deep Dives

<p align="center">
  <img src="https://raw.githubusercontent.com/Orellius/cli-denoiser/main/charts/scenarios-screenshot.png" alt="Scenario Deep Dives" width="100%">
</p>

### Full Benchmark Results

| Scenario | Original | Filtered | Saved | Savings | Signal |
|----------|----------|----------|-------|---------|--------|
| cargo build (69 crates) | 231 | 17 | 214 | **92.6%** | OK |
| git clone (verbose) | 85 | 8 | 77 | **90.6%** | OK |
| docker pull (layer progress) | 116 | 23 | 93 | **80.2%** | OK |
| npm install (deprecation spam) | 249 | 63 | 186 | **74.7%** | OK |
| mixed ANSI + progress | 101 | 41 | 60 | **59.4%** | OK |
| kubectl events (scheduling noise) | 162 | 69 | 93 | **57.4%** | OK |
| git push (transfer stats) | 166 | 77 | 89 | **53.6%** | OK |
| docker build (cached layers) | 189 | 103 | 86 | **45.5%** | OK |
| cargo test (with failures) | 170 | 158 | 12 | **7.1%** | OK |
| pure signal (no noise) | 70 | 68 | 2 | **2.9%** | OK |
| npm install (clean, no warnings) | 19 | 19 | 0 | **0.0%** | OK |

> **Signal column**: every scenario has required signal strings that must appear in filtered output. `OK` means zero information was lost. The benchmark catches false positives automatically.

## How It Works

CLI Denoiser runs a two-pass filter pipeline:

1. **Line-level filtering** -- each line is classified as Keep, Drop, Replace, or Uncertain. If any filter returns `Uncertain`, the line passes through (zero false positive guarantee baked into the type system).

2. **Block-level collapsing** -- consecutive noise lines are collapsed into summaries:
   - `Compiling serde v1.0.228` x47 lines becomes `[compiled 47 crates]`
   - Per-layer Docker pull progress becomes `[pulled 12 layers]`
   - Repeated deprecation warnings become `[8 deprecation warnings]`

### What Gets Filtered

| Filter | Strips | Preserves |
|--------|--------|-----------|
| **ANSI** | CSI sequences, OSC, color codes, carriage returns | All text content |
| **Progress** | Progress bars, Unicode spinners, percentage indicators | Final state |
| **Dedup** | 3+ consecutive identical lines | First occurrence + count |
| **Git** | Transfer stats (Enumerating/Counting/Compressing objects) | Branch names, errors, diffs, status |
| **npm** | Timing logs, HTTP fetch, deprecation spam, peer dep warnings | Installed packages, errors, audit |
| **Cargo** | Compiling/Checking/Fresh/Downloading per-crate lines | Errors, warnings, test results, Finished summary |
| **Docker** | Layer cache, pull progress, digest lines, intermediate containers | Build steps, errors, final image ID |
| **Kubectl** | klog verbose output, routine scheduling events | Pod status, errors, custom resources |
| **Generic** | Decorative border lines (box-drawing characters) | Everything else (maximally conservative) |

## Install

### From crates.io (recommended)

```bash
cargo install cli-denoiser
```

### From GitHub releases

Download the latest binary for your platform from [Releases](https://github.com/Orellius/cli-denoiser/releases), extract, and add to your PATH:

```bash
# macOS (Apple Silicon)
curl -L https://github.com/Orellius/cli-denoiser/releases/latest/download/cli-denoiser-aarch64-apple-darwin.tar.gz | tar xz
sudo mv cli-denoiser /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/Orellius/cli-denoiser/releases/latest/download/cli-denoiser-x86_64-apple-darwin.tar.gz | tar xz
sudo mv cli-denoiser /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/Orellius/cli-denoiser/releases/latest/download/cli-denoiser-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv cli-denoiser /usr/local/bin/
```

### From source

```bash
git clone https://github.com/Orellius/cli-denoiser.git
cd cli-denoiser
cargo install --path .
```

### Setup hooks (one command)

```bash
cli-denoiser install
```

This auto-detects and configures hooks for:
- **Claude Code** -- PostToolUse hooks on Bash, Read, and Grep tools
- **Codex CLI** -- post_exec hook on shell tool
- **Gemini CLI** -- post_tool_use hook on shell and bash tools

To remove:

```bash
cli-denoiser uninstall
```

## Usage

### As a command wrapper

```bash
# Wrap any command -- output is filtered automatically
cli-denoiser cargo build
cli-denoiser npm install
cli-denoiser docker build .
cli-denoiser git push origin main
```

### As a pipe filter

```bash
# Pipe any output through the filter
cargo build 2>&1 | cli-denoiser filter --command cargo
kubectl get events | cli-denoiser filter --command kubectl
```

### As an agent hook (recommended)

```bash
# Install hooks for all detected agents
cli-denoiser install

# That's it -- hooks run automatically on every tool call
```

### View savings

```bash
cli-denoiser gain           # Last 30 days
cli-denoiser gain --days 7  # Last 7 days
```

### Run benchmarks

```bash
cli-denoiser bench                              # Print to terminal
cli-denoiser bench --output bench-results.json  # Export JSON
```

## Architecture

```
stdin/command
     |
     v
 +-------------------+
 | ANSI Filter       |  Strip escape codes (always first)
 +-------------------+
     |
     v
 +-------------------+
 | Progress Filter   |  Collapse progress bars and spinners
 +-------------------+
     |
     v
 +-------------------+
 | Dedup Filter      |  Collapse repeated lines (3+ threshold)
 +-------------------+
     |
     v
 +-------------------+
 | Command Filter    |  Git / npm / Cargo / Docker / Kubectl / Generic
 +-------------------+
     |
     v
  filtered output
```

**Universal filters** (ANSI, Progress, Dedup) run on every command. The **command-specific filter** is selected based on the binary name via `CommandKind::detect()`.

## Zero False Positive Guarantee

The type system enforces this:

```rust
pub enum FilterResult {
    Keep,                    // Definitely signal
    Drop,                    // Definitely noise
    Replace(String),         // Noise, but summarize
    Uncertain,               // Not sure -- ALWAYS passes through
}
```

When a filter returns `Uncertain`, the pipeline keeps the original line unchanged. There is no "aggressive mode" that might lose signal. The benchmark suite verifies this automatically: every scenario has required signal strings that must appear in filtered output.

## Development

```bash
# Run all 4 verification gates
cargo clippy -- -D warnings
cargo test
cargo fmt --check
cargo check

# Build release binary
cargo build --release
```

## Contributing

Contributions are welcome. Here's how to get started:

### Setup

```bash
git clone https://github.com/Orellius/cli-denoiser.git
cd cli-denoiser
cargo build
cargo test
```

### Adding a New Filter

CLI Denoiser's filter pipeline is extensible. To add a new command filter:

1. Create `src/filters/<command>.rs` implementing the `Filter` trait
2. Add your `CommandKind` variant to `src/filters/command.rs`
3. Add detection logic in `CommandKind::detect()` (matches on binary name)
4. Write benchmark scenarios in `src/bench.rs` with required signal strings
5. Run `cli-denoiser bench` to verify zero false positives

### Submitting Changes

1. Fork the repo and create a branch: `git checkout -b feat/your-feature`
2. Make your changes and pass all 4 verification gates:
   ```bash
   cargo clippy -- -D warnings
   cargo test
   cargo fmt --check
   cargo check
   ```
3. Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, etc.
4. Open a PR against `main`. Describe what you changed and why.

### Guidelines

- Zero false positives is non-negotiable. If a filter is unsure, it must return `Uncertain` (which passes the line through unchanged).
- Keep filters conservative. It's better to pass noise through than to drop signal.
- Benchmark every change. Run `cli-denoiser bench` and include results in your PR.
- No `unwrap()` or `expect()` outside tests. Use `thiserror` for error types.

### Ideas for Contributions

- New command filters (pip, yarn, gradle, terraform, ansible)
- Windows support and testing
- Integration with more agentic coding tools
- Performance optimizations in the filter pipeline

## License

Apache-2.0. See [LICENSE](LICENSE).
