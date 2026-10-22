# UNX Technical Specification 0.1 (Draft)

Status: design draft for the initial prototype. Not a stable public standard.

## Goals

1. Human-readable language syntax.
2. Reproducible package layout.
3. Clear separation between source project, portable program, and platform installer.
4. Safe defaults and explicit permissions.
5. Platform adapters behind a stable runtime API.

## Language 0.1

Encoding: UTF-8. Newline-delimited statements. Whitespace around statements is ignored. Comments begin with `#`. String literals use double quotes. Supported escapes: `\\`, `\"`, `\n`, `\t`.

Statements:
- `say "Hello"` prints a string.
- `set name to "World"` creates/replaces a string variable.
- `say name` prints a previously defined variable.

Errors include a 1-based source line number. Unknown statements fail closed.

## Manifest v1

```json
{
  "format": "UNX",
  "format_version": 1,
  "id": "org.example.hello",
  "name": "Hello UNX",
  "version": "0.1.0",
  "entry": "main.unx",
  "permissions": []
}
```

`format` must be `UNX`; `format_version` must be `1`. `entry` is a relative package path. Production runtimes must reject unknown permissions rather than silently granting them.

## Package v1

A `.unx` package is a ZIP archive with `manifest.json` at root and the declared entry file. Additional resources may later live under `assets/`. Production archive handling must reject path traversal, symlinks, duplicate paths, oversized files, excessive compression ratios, and unexpected entries.

## Runtime architecture

- Front end: lexer/parser, AST, diagnostics.
- Execution: begin with an interpreter; consider bytecode or WebAssembly later.
- Standard library: deterministic, portable core APIs.
- UI layer: platform-independent widget model plus platform adapters.
- Windows first; Android, macOS, Linux, iOS, and web later.
- `.unx` is a source/deployment package, not an OS-native installer. Native packages require platform-specific build and signing tools.

## Security model

The 0.1 prototype is not a sandbox. Do not execute untrusted packages. Production requirements include deny-by-default capability permissions, resource limits, publisher signatures and revocation, verified updates, dependency integrity, audit logs, and parser/archive fuzz testing.

## Compatibility

A future manifest revision should declare minimum runtime version and required capabilities. Runtimes must reject unsupported mandatory features and must not silently reinterpret semantics.
