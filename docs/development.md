# Development

This page is for contributors and for future-you when the obvious commands stop being obvious.

## Core commands

Build:

```sh
cargo build
```

Format:

```sh
cargo fmt --all -- --check
```

Lint:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

Tests:

```sh
cargo test --workspace --all-targets
```

Docs site locally:

```sh
python -m pip install -r requirements-docs.txt
mkdocs serve
```

Strict site build:

```sh
mkdocs build --strict
```

## Documentation layout

Use the repository docs for source-of-truth engineering detail:

- `DESIGN.md`
- `ERRORS.md`
- `INSPECT_API.md`
- `SECURITY.md`
- `REFERENCES.md`

Use `docs/` for curated public-facing explanations.

The intent is to summarize and link, not to create a second divergent spec.

## Website deployment

The repository includes a dedicated GitHub Pages workflow that builds the MkDocs site and deploys the generated `site/` output.

That workflow is separate from the Rust CI workflow on purpose:

- Rust CI checks build, lint, tests, and docs for the codebase
- Pages deployment builds the public site

## Domain note

The docs source includes `docs/CNAME` with `tosumu.org`. If the DNS and GitHub Pages configuration are not ready yet, keep the file but expect the public domain to need separate repository settings.