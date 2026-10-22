# Build and test on Windows

Open PowerShell in this directory.

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

Expected sample output includes `Hello from` and `UNX`.

The build commands have not been executed in this file-generation environment. Run them locally and report any compiler errors so they can be corrected.
