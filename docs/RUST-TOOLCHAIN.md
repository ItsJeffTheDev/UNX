# Rust toolchain policy

UNX is an early-stage Rust project. Toolchain choices should favor contributor consistency without claiming a compiler version has been pinned when it has not.

## Current policy

- Use the **stable Rust channel** for local development.
- GitHub Actions installs the stable toolchain before running `cargo build` and `cargo test`.
- Check your environment with `rustc --version` and `cargo --version`.
- Run `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` before submitting changes when your local toolchain supports them; CI does not yet enforce these checks.
- If a change depends on a specific compiler behavior or minimum supported Rust version (MSRV), explain that in the pull request and update this policy after the project makes an explicit MSRV decision.

## Dependency reproducibility

A committed `Cargo.lock` and locked CI commands are still planned work. Until the lockfile is committed, dependency resolution can select newer compatible versions over time, so the current workflow is not fully reproducible. Do not hand-edit or fabricate a lockfile: generate it with Cargo, verify it with a clean build and test run, then commit it.

The intended follow-up is to commit a Cargo-generated lockfile and change CI to use `cargo build --locked` and `cargo test --locked`. Formatting and lint checks should become required CI steps only after existing code has been formatted and any reported warnings have been resolved.

## Updating the policy

When the project adopts an MSRV, pins a toolchain, or changes its dependency-locking strategy, update this document, the CI workflow, and contributor instructions together. Document what is actually enforced rather than what is merely recommended.
