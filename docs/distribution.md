# Distribution and adoption

## Current status and next release

v0.1.0 remains the published release with its original artifacts. This checkout prepares v0.1.1 with the agent skill, documentation site, demo, and cross-platform packaging. Do not replace v0.1.0 assets or retarget its tag.

The `Native packages` workflow builds and tests Windows x64, Linux x64 (GNU), and macOS Apple Silicon with Rust 1.85, then packages an executable, skill, MIT license, target-specific third-party notices, and individual SHA-256 checksum. CI artifacts are review material, not release downloads. macOS Intel is not covered by this matrix; it can use source installation.

Several macOS dependencies omit standalone license files from their crate archives. `licenses/` retains each original licensing policy and the upstream workspace MIT notice with separately pinned source URLs. Native archives include these provenance files. Preserve the upstream Apple SDK qualifications; packaging these notices is not an exhaustive legal assessment.

After the PR is reviewed and merged, dispatch the workflow on main, download the three verified packages, check their hashes and executable versions, create the v0.1.1 tag at that exact reviewed commit, and publish a release with the archives/checksums. Record commit and build provenance in release notes. Linux GNU binaries may require a sufficiently recent glibc; verify the chosen runner baseline before claiming support for older distributions. macOS packages are not signed or notarized.

## crates.io

The name appeared available during the October 2, 2026 check. Account authentication is required for first publication. Authenticate locally with `cargo login`; never put the token in source, issues, or chat.

```console
cargo package --locked
cargo publish --locked --dry-run
cargo publish --locked
```

Publish from the reviewed, clean release commit. Verify https://crates.io/crates/actually-done before adding a registry installation claim. Only after publication, document `cargo install actually-done --locked`. The CLI and skill are separate installations. Package verification runs against the actual archive; the package includes its referenced guides, fixtures, and skill license.

## Documentation site

`python scripts/build_site.py` creates `_site` using only Python's standard library. It validates local links, a single title/H1, and descriptions on all seven pages. No client analytics, external fonts, trackers, or accounts are added.

The Pages workflow validates PRs and deploys main. The canonical site URL is https://m-umer-farooq-dev.github.io/actually-done/. Set Pages to GitHub Actions after review, verify the deployment and anonymous access, then add the URL to repository About. Titles, descriptions, canonical URLs, Open Graph tags, sitemap, readable HTML, and source-code metadata support discovery; they do not guarantee indexing or ranking.

Verify a Search Console URL-prefix property using this URL. If Google offers HTML meta-tag verification, set the repository Actions variable `GOOGLE_SITE_VERIFICATION` to the supplied value and redeploy. Submit `sitemap.xml` only after verification. Ownership and account access are required; no unverified search-performance numbers are reported. The robots file allows crawling, including search bots; it makes no separate training opt-out promise.

## Skills discovery

Manual copying remains supported. Validate the third-party skills installer against this repository before documenting its command as tested. Its installer and telemetry policies are separate from the actually-done CLI, which has no telemetry. Read https://skills.sh/docs and the upstream CLI help. Do not imply that a directory listing, leaderboard placement, or install count is guaranteed.

## Weekly owner snapshots

```console
python scripts/adoption_snapshot.py
python scripts/adoption_snapshot.py --traffic
```

The first command gathers public stars/forks and published asset downloads. `--traffic` uses local GitHub credentials to add private visitor/clone/referrer aggregates. Snapshots stay in ignored `.metrics/` by default. No transcripts or individual user identities are read. Run manually once a week; no background scheduler is installed.

Compare star/asset count deltas and inspect daily visitor/clone data. GitHub traffic covers a rolling 14-day window, so do not sum overlapping snapshots. Downloads/clones are adoption proxies, not unique people, and automated activity can affect them. Record launch dates to interpret spikes. Track search impressions/clicks in the verified Search Console property and record repeatable AI-search queries, citations, dates, and platform settings separately.

## Community launch

Test the current installation path before sharing the project. Match each community's rules, answer questions, and request concrete feedback. Do not automate posts, solicit coordinated votes, claim competitors lack unverified features, or promise that DONE proves correctness. Keep launch drafts and internal implementation plans outside the public repository and package archives.
