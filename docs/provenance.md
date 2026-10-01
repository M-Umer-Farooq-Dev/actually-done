# Source provenance and licensing review

The project was created from the repository owner's brief and implementation requests, with AI-assisted development. The standalone Rust source was extracted from the earlier MIT-licensed project owned by the same maintainer. Its MIT notice is preserved. The Git history records that extraction and subsequent fixes.

The checked-in implementation, tests, examples, and documentation were reviewed for copied third-party implementation files, real transcripts, credentials, and unrelated assets. Fixtures are synthetic. Benchmark data describes local synthetic runs; it is not production customer data. No external implementation source or provider SDK is vendored. Third-party notice text is reproduced for attribution, with dependency names and versions.

Cargo.lock pins third-party dependencies. `docs/dependency-licenses.md` records their declared licenses and selected permissive options; `THIRD_PARTY_NOTICES.md` includes upstream notices for the Windows binary's dependency graph. The Unicode license applies in addition to MIT for unicode-ident. Apple-target dependencies have upstream SDK qualifications and must be reviewed before distributing an Apple binary; this release distributes only a Windows binary.

This is a documented provenance check, not a claim that tooling can prove copyright ownership. Contributors must have permission to submit their changes. Unless explicitly agreed otherwise, contributions to this MIT project are offered under its MIT license. Do not include code, transcripts, or other material you lack permission to distribute.

The public history uses the maintainer's GitHub noreply email. A local backup of the original commit metadata was retained outside the repository. That backup is not a release artifact.
