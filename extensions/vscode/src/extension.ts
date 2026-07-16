import * as vscode from "vscode";
import { registerDiagnostics } from "./providers/diagnostics";
import { registerHoverProvider } from "./providers/hover";
import { registerCodeLens } from "./providers/codeLens";
import { registerQuickFix } from "./providers/quickFix";

export function activate(context: vscode.ExtensionContext) {
  console.log("Grat extension activated");

  registerDiagnostics(context);
  registerHoverProvider(context);
  registerCodeLens(context);
  registerQuickFix(context);

  context.subscriptions.push(
    vscode.commands.registerCommand("grat.decode", () => {
      vscode.window.showInformationMessage("Grat: Decode not yet implemented");
    })
  );
}

export function deactivate() {}
