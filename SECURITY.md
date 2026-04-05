# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in CLI Denoiser, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, email [orel@orellius.ai](mailto:orel@orellius.ai) with:

- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

You should receive a response within 48 hours. We will work with you to understand the issue and coordinate a fix before any public disclosure.

## Scope

CLI Denoiser processes terminal output locally. Security concerns include:

- **Filter bypass:** crafted input that causes signal to be dropped (data loss)
- **Injection via hook installation:** the `install` command modifies agent config files
- **Dependency vulnerabilities:** in Rust crates used by the project

Out of scope: denial of service via large input (CLI tools are expected to handle arbitrary input sizes at the caller's discretion).
