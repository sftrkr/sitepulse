# Contributing to sitepulse

Thanks for your interest in contributing to sitepulse.

## Development setup

Requirements:

- Rust stable
- Cargo

Clone and verify the project:

```bash
git clone https://github.com/sftrkr/sitepulse.git
cd sitepulse
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Local smoke testing

Use a small sitemap or limit the number of checked URLs:

```bash
cargo run -- check https://example.com/sitemap.xml --max-urls 5
```

For large or production-like sites, prefer conservative settings:

```bash
cargo run -- check https://example.com/sitemap.xml \
  --max-urls 20 \
  --concurrency 2 \
  --rate-limit-per-second 1 \
  --per-host-concurrency 1
```

## Code style

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Documentation

Keep documentation in sync with behavior changes. If you add or change a CLI option, update:

- README.md
- CHANGELOG.md, when release-relevant
- tests or smoke-test notes, when applicable

## Pull requests

A good pull request should include:

- A clear summary of the change
- Why the change is needed
- Tests or validation steps
- Any relevant README or changelog updates

## Security issues

Please do not open public issues for vulnerabilities. See SECURITY.md.
