# 0001: Standalone Rust repository

Date: 2026-10-01. Status: accepted.

The Python prototype and native Rust variant previously shared a repository. The public release needs a simple source checkout and one native command.

Extract the Rust implementation into its own repository, with package and executable named `actually-done`, version 0.1.0, MIT licensing, and schema 1 receipts. The prior repository remains intact. Runtime operation does not depend on Python. Historical Python comparison results are retained as dated evidence, with the original executable name clearly identified.

Keep deterministic heuristics and the existing verdict precedence. Do not infer session authorship from current Git state, add an LLM judge, or upload transcripts. Unit contracts and native CLI tests belong in this repository; compatibility checks against the Python implementation can additionally run from the original development checkout.

The public name is independent of the local directory name `actually-done-rust`. Publishing to GitHub is a separate maintainer action. Windows x64 is validated; other operating systems need runtime validation before support claims expand.
