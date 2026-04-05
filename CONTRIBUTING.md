# Contributing to CLI Denoiser

Contributions are welcome. Here's how to get started.

## Setup

```bash
git clone https://github.com/Orellius/cli-denoiser.git
cd cli-denoiser
cargo build
cargo test
```

## Adding a New Filter

CLI Denoiser's filter pipeline is extensible. To add a new command filter:

1. Create `src/filters/<command>.rs` implementing the `Filter` trait
2. Add your `CommandKind` variant to `src/filters/command.rs`
3. Add detection logic in `CommandKind::detect()` (matches on binary name)
4. Write benchmark scenarios in `src/bench.rs` with required signal strings
5. Run `cli-denoiser bench` to verify zero false positives

## Submitting Changes

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

## Guidelines

- **Zero false positives is non-negotiable.** If a filter is unsure, it must return `Uncertain` (which passes the line through unchanged).
- **Keep filters conservative.** It's better to pass noise through than to drop signal.
- **Benchmark every change.** Run `cli-denoiser bench` and include results in your PR.
- **No `unwrap()` or `expect()` outside tests.** Use `thiserror` for error types.
- **Conventional commits required.** PRs with unclear commit messages will be asked to rewrite.

## Ideas for Contributions

- New command filters (pip, yarn, gradle, terraform, ansible)
- Windows support and testing
- Integration with more agentic coding tools
- Performance optimizations in the filter pipeline

## License

By contributing, you agree that your contributions will be licensed under the [Apache-2.0 License](LICENSE).
