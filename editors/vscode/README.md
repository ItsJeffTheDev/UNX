# UNX Language Support for VS Code

This experimental extension gives .unxc files a language identity, syntax highlighting, starter snippets, and commands to run source or package a project. It is not yet published to the VS Code Marketplace.

## Try it in Extension Development Host

1. Install VS Code and the stable Rust toolchain.
2. Clone the UNX repository.
3. In a terminal at the repository root, run: cargo build
4. Open the repository in VS Code.
5. Launch VS Code with the extension development path: code --extensionDevelopmentPath=editors/vscode .
6. Open a .unxc file. VS Code should identify it as UNX and provide syntax colors and snippets.

The commands use the UNX CLI executable. If unx is not on PATH, set the UNX: Executable Path setting (unx.executablePath) to the full executable path, for example target\debug\unx.exe on Windows or target/debug/unx on macOS/Linux. An absolute path is recommended.

## Commands

- UNX: Run Current .unxc File (Ctrl+F5 on Windows/Linux) runs the active saved source through the current interpreter.
- UNX: Build .unx Package runs the CLI pack command on the workspace root. The root must contain manifest.json and the entry file named by manifest.entry. The package is written to build/<workspace-name>.unx.

The current interpreter supports comments, string variables, and say output only. The package command bundles the manifest and one entry source file; it does not compile to native machine code or produce native installers. Cross-platform runtimes and installers remain future work.
