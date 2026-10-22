# UNX

An experimental, open-source English-like programming language with editable `.unxc` source files and distributable `.unx` packages, built in Rust.

## Status

UNX is an early prototype, not a production-ready universal app platform. The `.unxc` file is editable source code. The `.unx` file is currently a ZIP-based source/deployment package, not a native installer or a universal executable yet. Windows desktop is the first target; platform-specific installers and signing are future work.

The GitHub Actions workflow currently passes cargo build and cargo test. The language prototype supports comments, string variables, and say output. Arithmetic, functions, conditions, loops, native UI, installer generation, publisher signing, and hardened sandbox enforcement are not implemented yet.

## Build and try

Install stable Rust from https://www.rust-lang.org/tools/install, then run:

```powershell
cargo build
cargo test
cargo run -- --help
cargo run -- --version
cargo run -- run .\examples\hello\main.unx
cargo run -- check .\examples\hello\main.unx
cargo run -- pack .\examples\hello .\Hello.unx
cargo run -- inspect .\Hello.unx
cargo run -- run .\Hello.unx
```

Expected sample output includes `Hello from` and `UNX`.

## Example language

```text
# A comment
set product to "UNX"
say "Hello from"
say product
```

## Roadmap and documentation

- [Beginner tutorial: code in UNX and build a .unx package](docs/TUTORIAL.md)\n- [VS Code language support](editors/vscode/README.md)\n- [Full project roadmap](docs/ROADMAP.md)
- [Language specification 0.1](docs/SPEC-0.1.md)
- [Build and test guide](docs/BUILD-AND-TEST.md)

## Security warning

Do not run .unx packages from untrusted sources. This is an experimental interpreter/package prototype, not a hardened sandbox. It does not yet implement robust resource limits, publisher signatures, dependency integrity, or a secure update mechanism. Package inspection should not be treated as proof that a package is safe to execute.

## Contributing

Contributions are welcome. Please open an issue to discuss larger changes before starting implementation, and run the build and test commands before submitting a pull request. The roadmap describes the intended sequence; features are not supported until implemented and tested.
