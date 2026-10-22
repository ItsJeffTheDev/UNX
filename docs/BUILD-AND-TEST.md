# Build and test on Windows

Install the stable Rust toolchain from https://www.rust-lang.org/tools/install, then open PowerShell in the repository directory.

```powershell
rustc --version
cargo --version
cargo build
cargo test
cargo run -- run .\examples\hello\main.unx
cargo run -- check .\examples\hello\main.unx
cargo run -- pack .\examples\hello .\Hello.unx
cargo run -- inspect .\Hello.unx
cargo run -- run .\Hello.unx
```

Expected sample output includes `Hello from` and `UNX`. The `pack` command creates a ZIP-based `.unx` package; it does not create a native Windows installer.

## Continuous integration

GitHub Actions runs `cargo build` and `cargo test` on pushes to `main`, pull requests, and manual workflow dispatches. Check the current result on the [Actions page](https://github.com/ItsJeffTheDev/UNX/actions).

Until `Cargo.lock` is committed, builds may resolve newer compatible dependency versions over time. Do not treat this prototype as a hardened sandbox, and do not run packages from untrusted sources.
