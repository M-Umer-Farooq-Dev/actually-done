# Contributing

Use Rust 1.85 or newer and Git. Keep runtime behavior native and local; do not add telemetry or upload transcripts.

```console
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
python scripts/license_report.py --check
```

Add synthetic or thoroughly anonymized transcript fixtures for parsing changes. Never submit real credentials, private conversations, or local session archives. Cover verdict precedence and failure behavior when changing evidence collection. Preserve schema-versioned receipts or document a schema change explicitly.

The license-report script is development-only and needs Python 3.10 or newer. After dependency changes run `python scripts/license_report.py` and review the inventory and notices. CI tests Rust 1.85 on Windows, Linux, and macOS and checks formatting, Clippy, dependency advisories, licenses, and Git-history secrets. Vulnerability reports go through [SECURITY.md](SECURITY.md).

Describe the problem, resulting behavior, and checks in your pull request. Windows x64 is currently validated; contributions that exercise Unix process cleanup and provider discovery are welcome. Report bugs with OS, CLI version, exact command, expected result, and a minimal safe fixture.
