# tools/rust-port

Acceptance harness for the Rust port (see `docs/rust-port/HANDOFF.md`).

- `prepare-site.sh <site-repo> <workdir>` — checks out seeksnack at the golden commit, installs its npm deps, applies `site-overlay/`.
- `build-site.sh <neohugo-binary> <workdir>/seeksnack <outdir>` — builds with the golden settings (pinned clock, single worker, cold cache, `testdata/hugo_cache`).
- `compare.py <outdir> [--golden DIR]` — byte comparison against `golden/canonical.sha256` (6943 files).
- `golden/hugo_stats.json` — the golden build's `hugo_stats.json` (written to the site root, not the output).
