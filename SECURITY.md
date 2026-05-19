# Security Policy

## Supported versions

Security fixes are applied to the latest released version on [crates.io](https://crates.io/crates/convex-typegen).

## Reporting a vulnerability

Please report security issues privately by opening a [GitHub Security Advisory](https://github.com/JamalLyons/convex-typegen/security/advisories/new) or contacting the repository owner. Do not file public issues for undisclosed vulnerabilities.

Include steps to reproduce, impact, and any suggested fix if you have one.

## Scope

This crate runs at **build time** and reads TypeScript from your project tree. Untrusted Convex source files could cause high memory use; the lexer enforces a 10 MiB per-file limit. Runtime network access is only via the optional `convex` client feature in consuming applications, not during codegen.
