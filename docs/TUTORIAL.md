# Learn to Code in UNX: Beginner Tutorial

UNX is an experimental open-source programming language designed around readable code and portable app packages. This tutorial teaches the language subset that works today and the intended source-to-package workflow.

> **Important:** UNX is still a prototype. The .unx package is a ZIP-based package, not a native executable or a promise that the app already runs on every operating system.

## 1. Understand the two file types

- **.unxc — UNX Code:** the editable source file developers write. Keep it in your project and version-control it.
- **.unx — UNX Package:** the distributable package created from a project. The long-term goal is one package that can run across supported UNX runtimes on phones and computers. Each platform still needs a compatible runtime or platform-specific build/installer.

The current CLI reads source text regardless of filename extension, so running main.unxc works with this prototype.

## 2. Install the tools

1. Install Visual Studio Code.
2. Install Rust from https://www.rust-lang.org/tools/install because the current CLI is written in Rust.
3. Clone https://github.com/ItsJeffTheDev/UNX and open its folder in a terminal.
4. Build and test: cargo build and cargo test.

## 3. Write your first program

Create a file named main.unxc:

    # My first UNX program
    set appName to "My First App"
    say "Hello from "
    say appName

Meaning:
- A line beginning with # is a comment.
- set appName to "My First App" stores a string in a variable.
- say "Hello from " prints a literal string.
- say appName prints the variable's value.

Run and check the program from the UNX repository root:

    cargo run -- run ./main.unxc
    cargo run -- check ./main.unxc

Expected output:

    Hello from
    My First App

## 4. Learn the current syntax

Print text:

    say "Welcome to UNX"

Store and print a value:

    set language to "UNX"
    say language

Copy a value and test it:

    set language to "UNX"
    set copied_language to language
    if copied_language equals "UNX" say "The value matches!"

The single-line `if` compares a previously defined variable with an exact quoted string and only runs its `say` action when the values match. Multi-line blocks start with `if VARIABLE equals "VALUE"`, may include one `else`, and must end with `end`. Blocks can be nested. The interpreter executes only the selected branch.

Comments:

    # This is ignored by the current interpreter
    say "Comments start with a number sign"

String literals use double quotes. Supported escapes include backslash-n for newline, backslash-t for tab, escaped quote, and escaped backslash. Keywords are lowercase and case-sensitive. The prototype supports single-line and multi-line exact-string conditions, including nested `if` blocks and an optional `else`. It does not yet support arithmetic, loops, functions, or networking. GUI buttons and windows are experimental.

## 5. Work with numbers and conditions

The current 0.2 preview supports integer arithmetic. Multiplication, division, and remainder bind more tightly than addition and subtraction, and parentheses can group operations:

    set subtotal to 2 + 3 * 4
    say subtotal
    set grouped to (2 + 3) * 4
    say grouped

Output:

    14
    20

Numeric conditions use comparison symbols and can be written as a block or a single-line condition:

    set score to 10
    if score > 5
      say "Score is above five"
    else
      say "Score is five or below"
    end
    if score >= 10 say "At least ten"

The supported numeric comparisons are `>`, `<`, `>=`, `<=`, `==`, and `!=`. Variables used in numeric expressions must contain integers. Decimal numbers, Boolean values, compound conditions, loops, and functions are not supported yet. The 0.2 expression syntax is still a preview rather than a stable compatibility promise.

## 6. Build a distributable package

Create a folder like this:

    MyApp/
      manifest.json
      main.unxc

Create manifest.json with this content:

    {
      "format": "UNX",
      "format_version": 1,
      "id": "org.example.myapp",
      "name": "My App",
      "version": "0.1.0",
      "entry": "main.unxc",
      "permissions": []
    }

Make sure main.unxc exists and contains valid code. Then run:

    cargo run -- pack ./MyApp ./MyApp.unx

Inspect the package without executing it:

    cargo run -- inspect ./MyApp.unx

Run it with the current prototype:

    cargo run -- run ./MyApp.unx

This creates a ZIP-based .unx package containing the manifest and source entry. It does not compile to machine code or make a standalone mobile/desktop installer.

## 7. Use the VS Code extension

The experimental extension is in editors/vscode/. It registers .unxc as the UNX language, adds syntax coloring and snippets, and provides UNX: Run Current .unxc File and UNX: Build .unx Package commands. See the extension guide in editors/vscode/README.md.

## 8. Share your project

Share the source project (including .unxc files and manifest.json) through GitHub. For distributing a built prototype package, share the .unx file and explain which UNX runtime is required. Never run packages from untrusted sources; this prototype is not a hardened sandbox.

## 9. What cross-platform will eventually require

To deliver the long-term write-once, distribute-widely goal, UNX still needs:
- a stable language specification and compiler/build pipeline;
- a runtime or compiler backend for each target;
- a portable UI and system API layer;
- asset and dependency packaging;
- target-specific builds for Windows, macOS, Linux, Android, iOS, and web where feasible;
- signing, updates, permission enforcement, and security testing.

The file extension is the packaging convention; the runtime/backend is what makes the program work on a device. UNX should only claim support for targets that are built and tested.

Follow docs/ROADMAP.md for implementation status.
