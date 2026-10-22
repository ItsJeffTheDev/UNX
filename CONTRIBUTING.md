# Contributing to UNX

Thanks for helping build UNX. The project is an early prototype, so clear bug reports, small pull requests, tests, and documentation improvements are especially valuable.

## Before starting

1. Read the [README](README.md), [tutorial](docs/TUTORIAL.md), [draft package specification](docs/SPEC-0.1.md), and [roadmap](docs/ROADMAP.md).
2. Search existing issues and pull requests to avoid duplicate work.
3. For large language or package-format changes, open an issue first to discuss semantics and compatibility.

## Development setup

- Install the current stable Rust toolchain using [rustup](https://rustup.rs/).
- Clone the repository and work on a topic branch.
- Build and test with:

  ```sh
  cargo build
  cargo test
  ```

- Before opening a pull request, run `cargo fmt --check` and, when available in your environment, `cargo clippy --all-targets --all-features -- -D warnings`.
- For changes to the VS Code extension, validate its JSON files and test it in an Extension Development Host.

## Pull request expectations

- Keep each pull request focused and explain the user-visible reason for the change.
- Add regression tests for bug fixes and tests for new behavior.
- Update documentation and examples when syntax, commands, manifests, or package behavior changes.
- Do not claim native compilation, universal OS support, or secure sandboxing unless the implementation and tests actually provide it.
- Never commit credentials, tokens, private keys, personal data, build outputs, or untrusted packages.
- Explain checks you ran and any checks you could not run.

## Language and package compatibility

The language and `.unx` package format are still evolving. Treat `docs/SPEC-0.1.md` as a draft, not a guarantee of long-term compatibility. Discuss breaking changes before implementing them.

## Security reports

Please do not publish exploitable vulnerabilities or sensitive exploit details in a public issue. Until a private security-reporting process is documented, contact the repository owner privately through GitHub to coordinate a responsible report.

## License

By contributing, you agree that your contributions are provided under the repository's MIT license.
