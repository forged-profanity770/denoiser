# CLI Denoiser

Strip terminal noise for LLM agents. Zero false positives. Rust.

Two-pass filter pipeline that removes progress bars, ANSI codes, duplicate lines, and verbose build output before it reaches your agent. Supports git, npm, cargo, docker, kubectl.

## Stack

Rust, clap, serde. Available on [crates.io](https://crates.io/crates/cli-denoiser).

## License

[MIT](LICENSE)