# UNX

An experimental, open-source English-like programming language with editable `.unxc` source files and distributable `.unx` packages, built in Rust.

## Status

UNX is an early prototype, not a production-ready universal app platform. The `.unxc` file is editable source code. The `.unx` file is currently a ZIP-based source/deployment package, not a native installer or a universal executable yet. Windows desktop is the first target; platform-specific installers and signing are future work.

The language prototype supports comments, string variables (including copying an existing variable with `set copy to original`), integer arithmetic with precedence and parentheses, numeric comparisons, single-line and multi-line conditions, `say` output, and an experimental desktop window statement with clickable buttons that can display literal messages or existing string variables. The first GUI runtime uses a simple pixel-rendered window with text and an Escape-to-close loop. It is an early prototype, not a full widget toolkit; decimal numbers, booleans, compound logical conditions, functions, loops, installer generation, publisher signing, and hardened sandbox enforcement are not implemented yet.

## Build and try

Install stable Rust from https://www.rust-lang.org/tools/install, then run:

```powershell
cargo build
cargo test
cargo run -- --help
cargo run -- --version
cargo run -- run .\examples\hello\main.unx
cargo run -- run .\examples\gui_test\main.unxc
cargo run -- check .\examples\hello\main.unx
cargo run -- pack .\examples\hello .\Hello.unx
cargo run -- inspect .\Hello.unx
cargo run -- run .\Hello.unx
```

The GUI smoke test opens an 800×500 window with two experimental clickable buttons. Click a button to update the status text; press `Esc` to close it. The test is `examples/gui_test/main.unxc`.

## Example language

```text
# A full-line comment
set product to "UNX"
say "Hello from"
set product_copy to product
if product_copy equals "UNX" say "Product matched"
if product_copy equals "UNX"
  say "This is a multi-line true branch"
else
  say "This is the alternate branch"
end
say product_copy # Inline comments are supported too
```

## Numeric expressions preview

```text
set subtotal to 2 + 3 * 4
say subtotal
set grouped to (2 + 3) * 4
say grouped
set score to 10
if score > 5
  say "Score is above five"
else
  say "Score is five or below"
end
if score >= 10 say "At least ten"
```

The current integer-expression preview supports `+`, `-`, `*`, `/`, `%`, unary minus, parentheses, and numeric comparisons `>`, `<`, `>=`, `<=`, `==`, and `!=`. It rejects division by zero, integer overflow, undefined numeric variables, and string values used as numbers. This is an early 0.2 preview; see the draft specification for current limitations.

## Roadmap and documentation

- [Beginner tutorial: code in UNX and build a .unx package](docs/TUTORIAL.md)
- [VS Code language support](editors/vscode/README.md)
- [Contributing guide](CONTRIBUTING.md)
- [Full project roadmap](docs/ROADMAP.md)
- [Language specification 0.1](docs/SPEC-0.1.md)
- [Prototype .unx package format v1](docs/UNX-PACKAGE-FORMAT.md)
- [Build and test guide](docs/BUILD-AND-TEST.md)
- [Rust toolchain and dependency reproducibility policy](docs/RUST-TOOLCHAIN.md)

## Security warning

Do not run .unx packages from untrusted sources. This is an experimental interpreter/package prototype, not a hardened sandbox. It does not yet implement robust resource limits, publisher signatures, dependency integrity, or a secure update mechanism. Package inspection should not be treated as proof that a package is safe to execute.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md), open an issue to discuss larger changes before starting implementation, and run the build and test commands before submitting a pull request. The roadmap describes the intended sequence; features are not supported until implemented and tested.
