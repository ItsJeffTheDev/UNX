# UNX

An experimental, open-source English-like programming language with editable `.unxc` source files and distributable `.unx` packages, built in Rust.

## Status

UNX is an early prototype, not a production-ready universal app platform. The `.unxc` file is editable source code. The `.unx` file is currently a ZIP-based source/deployment package, not a native installer or a universal executable yet. Windows desktop is the first target; platform-specific installers and signing are future work.

The language prototype supports comments, string variables, say output, and an experimental desktop window statement with clickable buttons that can display literal messages or existing string variables. The first GUI runtime uses a simple pixel-rendered window with text and an Escape-to-close loop. It is an early prototype, not a full widget toolkit; inputs, layout containers, general event handlers, arithmetic, functions, conditions, loops, installer generation, publisher signing, and hardened sandbox enforcement are not implemented yet.

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
say product # Inline comments are supported too
```

## Roadmap and documentation

- [Beginner tutorial: code in UNX and build a .unx package](docs/TUTORIAL.md)
- [VS Code language support](editors/vscode/README.md)
- [Contributing guide](CONTRIBUTING.md)\n- [Full project roadmap](docs/ROADMAP.md)
- [Language specification 0.1](docs/SPEC-0.1.md)
- [Build and test guide](docs/BUILD-AND-TEST.md)

## Security warning

Do not run .unx packages from untrusted sources. This is an experimental interpreter/package prototype, not a hardened sandbox. It does not yet implement robust resource limits, publisher signatures, dependency integrity, or a secure update mechanism. Package inspection should not be treated as proof that a package is safe to execute.

## Contributing

Contributions are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md), open an issue to discuss larger changes before starting implementation, and run the build and test commands before submitting a pull request. The roadmap describes the intended sequence; features are not supported until implemented and tested.
