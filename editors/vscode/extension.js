const vscode = require("vscode");
const path = require("node:path");
const { spawn } = require("node:child_process");

function activate(context) {
  const output = vscode.window.createOutputChannel("UNX");

  const runSource = vscode.commands.registerCommand("unx.runSource", async () => {
    const editor = vscode.window.activeTextEditor;
    const document = editor && editor.document;
    if (!document || document.languageId !== "unx") {
      vscode.window.showErrorMessage("Open a .unxc source file first.");
      return;
    }
    await document.save();
    const executable = vscode.workspace.getConfiguration("unx").get("executablePath", "unx");
    output.show(true);
    output.appendLine("\n> UNX run " + document.fileName);
    launch(executable, ["run", document.fileName], path.dirname(document.fileName), output);
  });

  const buildPackage = vscode.commands.registerCommand("unx.buildPackage", async () => {
    const folder = vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders[0];
    if (!folder) {
      vscode.window.showErrorMessage("Open a UNX project folder containing manifest.json first.");
      return;
    }
    try {
      await vscode.workspace.fs.stat(vscode.Uri.joinPath(folder.uri, "manifest.json"));
    } catch {
      vscode.window.showErrorMessage("No manifest.json found in the workspace root. See docs/TUTORIAL.md for the project layout.");
      return;
    }
    const outputDir = path.join(folder.uri.fsPath, "build");
    try {
      await vscode.workspace.fs.createDirectory(vscode.Uri.file(outputDir));
    } catch (error) {
      vscode.window.showErrorMessage("Could not create the build directory: " + error.message);
      return;
    }
    const outputFile = path.join(outputDir, path.basename(folder.uri.fsPath) + ".unx");
    const executable = vscode.workspace.getConfiguration("unx").get("executablePath", "unx");
    output.show(true);
    output.appendLine("\n> UNX pack " + folder.uri.fsPath + " " + outputFile);
    launch(executable, ["pack", folder.uri.fsPath, outputFile], folder.uri.fsPath, output);
  });

  context.subscriptions.push(output, runSource, buildPackage);
}

function launch(executable, args, cwd, output) {
  const child = spawn(executable, args, { cwd, windowsHide: true });
  child.stdout.on("data", data => output.append(data.toString()));
  child.stderr.on("data", data => output.append(data.toString()));
  child.on("error", error => {
    output.appendLine("\nCould not start the UNX CLI: " + error.message);
    output.appendLine("Build UNX and set unx.executablePath to the executable, or add it to PATH.");
    vscode.window.showErrorMessage("Could not start UNX. Check the UNX Output panel and unx.executablePath.");
  });
  child.on("close", code => {
    output.appendLine("\n[UNX exited with code " + code + "]");
    if (code === 0) vscode.window.setStatusBarMessage("UNX command completed", 4000);
  });
}

function deactivate() {}
module.exports = { activate, deactivate };
