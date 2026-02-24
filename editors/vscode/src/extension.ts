import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import { AgentCapabilitiesPanel } from "./agentPanel.js";
import { registerCommands } from "./commands.js";
import { StatusBarManager } from "./statusBar.js";

let client: LanguageClient | undefined;
let outputChannel: vscode.OutputChannel;
let statusBar: StatusBarManager | undefined;

export function activate(context: vscode.ExtensionContext): void {
  outputChannel = vscode.window.createOutputChannel("Tenor");
  context.subscriptions.push(outputChannel);
  outputChannel.appendLine("Tenor extension activated");

  // Create status bar manager
  statusBar = new StatusBarManager();
  context.subscriptions.push(statusBar);

  // Register command palette commands first — these must work even if LSP fails
  registerCommands(context);

  // Register the agent capabilities panel command
  const openCapabilities = vscode.commands.registerCommand(
    "tenor.openAgentCapabilities",
    () => {
      if (!client) {
        vscode.window.showWarningMessage(
          "Tenor LSP client is not running. Set 'tenor.path' in settings or ensure 'tenor' is on PATH."
        );
        return;
      }

      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "tenor") {
        vscode.window.showWarningMessage(
          "Open a .tenor file first to view agent capabilities."
        );
        return;
      }

      const panel = AgentCapabilitiesPanel.createOrShow(
        context.extensionUri,
        client
      );
      const uri = editor.document.uri.toString();
      void panel.refresh(uri);
    }
  );
  context.subscriptions.push(openCapabilities);

  // Try to start the LSP client — don't let failure break the extension
  startLspClient(context);

  context.subscriptions.push({
    dispose: () => {
      if (client) {
        client.stop();
      }
    },
  });
}

function startLspClient(_context: vscode.ExtensionContext): void {
  try {
    // Find the tenor binary
    const config = vscode.workspace.getConfiguration("tenor");
    const customPath = config.get<string>("path", "");
    const command = customPath || "tenor";

    // Server options: spawn `tenor lsp` over stdio
    const serverOptions: ServerOptions = {
      command,
      args: ["lsp"],
    };

    // Client options: activate for .tenor files
    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ scheme: "file", language: "tenor" }],
      outputChannel,
      workspaceFolder: vscode.workspace.workspaceFolders?.[0],
    };

    // Create and start the language client
    client = new LanguageClient(
      "tenor-lsp",
      "Tenor Language Server",
      serverOptions,
      clientOptions
    );

    client.start().then(
      () => {
        outputChannel.appendLine("Tenor LSP client started");

        // Listen for agent capabilities update notifications from LSP
        if (client) {
          client.onNotification(
            "tenor/agentCapabilitiesUpdated",
            (params: { uri: string; capabilities: unknown }) => {
              const panel = AgentCapabilitiesPanel.getCurrent();
              if (panel && panel.isVisible()) {
                panel.updateWithData(params.capabilities);
              }
            }
          );
        }
      },
      (err) => {
        outputChannel.appendLine(
          `Tenor LSP client failed to start: ${err}`
        );
        client = undefined;
      }
    );
  } catch (err) {
    outputChannel.appendLine(
      `Failed to initialize LSP client: ${err}`
    );
    client = undefined;
  }
}

export function deactivate(): Promise<void> | undefined {
  if (client) {
    return client.stop();
  }
  return undefined;
}
