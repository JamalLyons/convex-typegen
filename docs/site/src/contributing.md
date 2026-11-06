# Develop this project

The [contributor guide](https://github.com/JamalLyons/convex-typegen/blob/master/CONTRIBUTING.md)
covers building, testing, and proposing changes. Consumer chapters live under
`docs/site/src/`; edit those Markdown sources and use `just docs` to preview the
mdBook. The first project includes code directly from `examples/readme/`.

- [Local end-to-end checks](docs/e2e.md) explain how to exercise the Rust client against
  a disposable, self-hosted Convex backend. CI compiles the examples without starting Convex.
- [Architecture](https://github.com/JamalLyons/convex-typegen/blob/master/docs/architecture.md)
  maps the pipeline and its invariants for maintainers.
- [Semantic versioning](docs/semver-policy.md) and [API stability](docs/api-stability.md)
  describe how to review changes to the public contract.
