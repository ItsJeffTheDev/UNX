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
- `open window "Title"` opens an experimental desktop window.
- `set window width to "800"` sets the window width (200–1920 pixels).
- `set window height to "500"` sets the window height (150–1200 pixels).
- `button "Click me" say "Clicked!"` adds a clickable button to an open window. Clicking it displays the associated message in the window status area.
- When a window is opened, subsequent `say` statements are rendered as text in that window. Press Escape to close it.

Errors include 1-based source line and column numbers. Unknown statements and malformed tokens fail closed. This grammar describes the current 0.1 subset only; it is not a promise that future syntax is already implemented.

### Lexical and grammar rules

The current interpreter reads UTF-8 source one physical line at a time, tokenizes each non-comment line, and then parses the resulting tokens into an abstract syntax tree. Tokens include the reserved keywords `say`, `set`, `to`, `open`, `window`, `width`, `height`, and `button`, identifiers, and string literals. A statement may have leading/trailing whitespace. A comment begins when the first non-whitespace character is `#`; inline comments are not currently supported. Keywords are lowercase and case-sensitive. Identifiers begin with an ASCII letter or underscore and continue with ASCII letters, digits, or underscores. A digit cannot be the first character.

Simplified line grammar (EBNF):

```ebnf
program       = { blank-line | comment-line | statement-line } ;
blank-line    = whitespace ;
comment-line  = whitespace, "#", { any-character-except-newline } ;
statement-line = whitespace, statement, whitespace ;
statement     = say-string | say-variable | set-string | open-window | set-window-size | button-statement ;
open-window   = "open", space, "window", space, string ;
set-window-size = "set", space, "window", space, ("width" | "height"), space, "to", space, string ;
button-statement = "button", space, string, space, "say", space, string ;
say-string    = "say", space, string ;
say-variable  = "say", space, identifier ;
set-string    = "set", space, identifier, space, "to", space, string ;
string        = '"', { string-character | escape }, '"' ;
escape        = '\\', ( '\\' | '"' | 'n' | 't' ) ;
identifier    = ( letter | "_" ), { letter | digit | "_" } ;
space         = " ", { whitespace } ;
whitespace    = " " | "\t" ;
```

The grammar is intentionally limited: strings are the only value type, and `set` accepts a string literal rather than an expression. GUI statements are experimental. The current desktop window supports text and basic clickable buttons that update a status message; text inputs and portable platform-independent widgets are not implemented. A string may contain escaped quote, backslash, newline, and tab. Unknown or malformed escapes are errors. The implementation should reject malformed input rather than guess at its meaning. Diagnostics use 1-based line and column positions. Keywords are reserved and cannot be used as variable identifiers.

Future syntax such as numbers, arithmetic, conditions, loops, functions, inline comments, and multi-line blocks requires a language-versioned design and parser tests before it becomes part of the supported language.

## Manifest v1

```json
{
  "format": "UNX",
  "format_version": 1,
  "id": "org.example.hello",
  "name": "Hello UNX",
  "version": "0.1.0",
  "entry": "main.unxc",
  "permissions": []
}
```

`format` must be `UNX`; `format_version` must be `1`. `entry` is a relative package path. Production runtimes must reject unknown permissions rather than silently granting them.

## Source and distribution file roles\n\n- `.unxc` is the editable, human-authored UNX source file extension.\n- `.unx` is the distributable package extension. In the current prototype it is a ZIP archive containing `manifest.json` and the entry source; it is not native machine code or an operating-system installer.\n- The long-term portability goal requires a compatible UNX runtime or target-specific compiler/backend on each supported device. The extension alone does not make a package execute on every OS.\n\n## Package v1

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
