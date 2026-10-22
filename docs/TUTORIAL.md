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

Comments:

    # This is ignored by the current interpreter
    say "Comments start with a number sign"

String literals use double quotes. Supported escapes include backslash-n for newline, backslash-t for tab, escaped quote, and escaped backslash. Keywords are lowercase and case-sensitive. The prototype does not yet support arithmetic, conditions, loops, functions, buttons, screens, or networking.

## 5. Build a distributable package

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

## 6. Use the VS Code extension

The experimental extension is in editors/vscode/. It registers .unxc as the UNX language, adds syntax coloring and snippets, and provides UNX: Run Current .unxc File and UNX: Build .unx Package commands. See the extension guide in editors/vscode/README.md.

## 7. Share your project

Share the source project (including .unxc files and manifest.json) through GitHub. For distributing a built prototype package, share the .unx file and explain which UNX runtime is required. Never run packages from untrusted sources; this prototype is not a hardened sandbox.

## 8. What cross-platform will eventually require

To deliver the long-term write-once, distribute-widely goal, UNX still needs:
- a stable language specification and compiler/build pipeline;
- a runtime or compiler backend for each target;
- a portable UI and system API layer;
- asset and dependency packaging;
- target-specific builds for Windows, macOS, Linux, Android, iOS, and web where feasible;
- signing, updates, permission enforcement, and security testing.

The file extension is the packaging convention; the runtime/backend is what makes the program work on a device. UNX should only claim support for targets that are built and tested.

Follow docs/ROADMAP.md for implementation status.
