# Publishing and releases

The repository is `M-Umer-Farooq-Dev/actually-done`. Its main branch tracks GitHub. Repository visibility and release publication are explicit maintainer actions; preparing a build does not change visibility.

Before publishing a version:

1. Review code provenance, the MIT license, dependency license inventory, and generated notices. Do not add material you lack permission to distribute.
2. Scan all Git history for credentials and review fixtures, benchmark data, and workflow logs for private information.
3. Require passing CI for the release commit: quality, tests and builds on Rust 1.85, dependency audit, and secret scanning.
4. Read the README and inspect diagrams on GitHub. Keep platform claims limited to actual validation.
5. Update CHANGELOG.md and the version in Cargo.toml, regenerate Cargo.lock and notices if dependencies change, and check source installation.
6. Obtain independent review, address blockers, then tag the reviewed commit. Do not silently retarget a published version tag.

```console
git tag -a v0.1.0 -m "actually-done 0.1.0"
git push origin v0.1.0
```

GitHub's Settings → Danger Zone → Change visibility controls public access. Check public-history implications before using it. SECURITY.md links to private vulnerability reporting, which must be enabled in repository settings.

A source-only release is sufficient for open-source publication. When distributing a binary, include LICENSE, THIRD_PARTY_NOTICES.md, its platform/architecture, and a SHA-256 checksum. Validate that exact binary before uploading it. Do not advertise a crates.io package or a binary for an untested target. Additional platform release packages need notices generated for their corresponding target.
