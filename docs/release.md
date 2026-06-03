# Release process

This document describes how sitepulse releases are prepared, tagged, built, and verified.

## Automation overview

sitepulse uses two release-related workflows:

- Release PR: version and changelog automation with release-plz.
- Release: tag-based binary builds and GitHub Release publishing.

## Versioning

Versioning is managed through conventional commits and release-plz.

Recommended commit prefixes:

- feat: for features
- fix: for bug fixes
- perf: for performance improvements
- docs: for documentation-only changes
- refactor: for internal refactors
- ci: for workflow changes
- test: for test-only changes

## Manual release checklist

Use this checklist when cutting a release manually.

1. Ensure the repository is clean.

```bash
git status --short --branch
```

2. Run local CI.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

3. Update the crate version in Cargo.toml.

4. Update CHANGELOG.md.

5. Add release notes under docs/releases/.

6. Commit the release changes.

```bash
git add Cargo.toml Cargo.lock CHANGELOG.md docs/releases
git commit -m "chore: release vX.Y.Z"
git push
```

7. Create and push the annotated tag.

```bash
git tag -a vX.Y.Z -m "sitepulse vX.Y.Z"
git push origin vX.Y.Z
```

8. Wait for the Release workflow to finish.

9. Verify the GitHub Release assets.

Expected assets:

- sitepulse-aarch64-apple-darwin.tar.gz
- sitepulse-x86_64-apple-darwin.tar.gz
- sitepulse-x86_64-pc-windows-msvc.zip
- sitepulse-x86_64-unknown-linux-gnu.tar.gz

## GitHub CLI verification

```bash
gh release view vX.Y.Z --json tagName,name,url,assets --jq '{tagName,name,url,assets:[.assets[].name]}'
```

Check asset count:

```bash
gh release view vX.Y.Z --json assets --jq '.assets | length'
```

## Notes

- Cargo registry publishing is disabled in release-plz.toml.
- GitHub Release binary publishing is handled by the tag-based Release workflow.
- If a workflow warning mentions deprecated Node.js runtimes, first check whether newer major versions of the affected action are available.
