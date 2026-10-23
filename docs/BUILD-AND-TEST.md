# Build and test on Windows

Install the stable Rust toolchain from https://www.rust-lang.org/tools/install, then open PowerShell in the repository directory.

```powershell
rustc --version
cargo --version
cargo build
cargo test
cargo test --test cli
cargo run -- run .\examples\hello\main.unx
cargo run -- check .\examples\hello\main.unx
cargo run -- pack .\examples\hello .\Hello.unx
cargo run -- inspect .\Hello.unx
cargo run -- run .\Hello.unx
```

Expected sample output includes `Hello from` and `UNX`. The `pack` command creates a ZIP-based `.unx` package; it does not create a native Windows installer.

## Continuous integration

GitHub Actions runs Clippy (`cargo clippy --all-targets --all-features -- -D warnings`), `cargo build`, and `cargo test` on pushes to `main`, pull requests, and manual workflow dispatches. It also runs `cargo fmt --all -- --check` as a non-blocking audit while the existing formatting baseline is being cleaned up. The `cli` integration-test target exercises real subprocess invocations of the CLI, including source checking, direct execution, package round-tripping, and malformed-package rejection. Check the current result on the [Actions page](https://github.com/ItsJeffTheDev/UNX/actions).

Until `Cargo.lock` is committed, builds may resolve newer compatible dependency versions over time. Do not treat this prototype as a hardened sandbox, and do not run packages from untrusted sources.


## CLI exit codes

The CLI uses these exit statuses:

| Exit code | Meaning |
|---:|---|
| `0` | Command completed successfully; `--help` and `--version` also succeed. |
| `1` | A valid command was requested, but execution failed—for example, invalid source, an unreadable file, or an invalid package. |
| `2` | The command name or argument count is invalid. UNX prints usage information to standard error. |

Scripts should check the process exit code rather than relying only on human-readable output. These statuses are covered by CLI integration tests for representative invalid invocations and command failures.
