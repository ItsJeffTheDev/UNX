# UNX Starter

An experimental English-like programming language and `.unx` package format.

## Status

This is an early prototype, not a production-ready universal app platform. The `.unx` file is a ZIP-based project/deployment package, not a native installer. Windows-first support is the initial target; platform-specific installers and signing remain future work.

## Build

Install stable Rust from https://www.rust-lang.org/tools/install, then run:

```powershell
cargo build
cargo test
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

The prototype supports comments, string variables, and `say` output. Arithmetic, functions, conditions, loops, native UI, installer generation, signing, and sandbox enforcement are not implemented.

## Security warning

Do not run `.unx` packages from untrusted sources. This is an experimental interpreter/package prototype, not a hardened sandbox. It does not yet implement resource limits, publisher signatures, dependency integrity, or a secure update mechanism.

See `docs/SPEC-0.1.md` and `docs/BUILD-AND-TEST.md` for details. The code has not yet been verified by a successful build in this environment.