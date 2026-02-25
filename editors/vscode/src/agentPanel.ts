/**
 * Agent Capabilities webview panel.
 *
 * Shows a rich HTML view of the contract from the agent's perspective:
 * operations grouped by persona, entity state machine SVG diagrams,
 * flow summaries, and static analysis findings.
 *
 * Communicates with the LSP server via the custom
 * `tenor/agentCapabilities` request.
 */

import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { renderStateMachine, EntityView } from "./svgRenderer.js";

/**
 * Singleton webview panel for agent capabilities.
 */
export class AgentCapabilitiesPanel {
  public static readonly viewType = "tenor.agentCapabilities";

  private static currentPanel: AgentCapabilitiesPanel | undefined;
  private readonly panel: vscode.WebviewPanel;
  private readonly extensionUri: vscode.Uri;
  private client: LanguageClient;
  private currentUri: string | undefined;
  private disposables: vscode.Disposable[] = [];

  /**
   * Create or reveal the agent capabilities panel.
   */
  public static createOrShow(
    extensionUri: vscode.Uri,
    client: LanguageClient
  ): AgentCapabilitiesPanel {
    if (AgentCapabilitiesPanel.currentPanel) {
      AgentCapabilitiesPanel.currentPanel.panel.reveal(
        vscode.ViewColumn.Beside
      );
      AgentCapabilitiesPanel.currentPanel.client = client;
      return AgentCapabilitiesPanel.currentPanel;
    }

    const panel = vscode.window.createWebviewPanel(
      AgentCapabilitiesPanel.viewType,
      "Agent Capabilities",
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: [vscode.Uri.joinPath(extensionUri, "media")],
      }
    );

    AgentCapabilitiesPanel.currentPanel = new AgentCapabilitiesPanel(
      panel,
      extensionUri,
      client
    );
    return AgentCapabilitiesPanel.currentPanel;
  }

  /**
   * Get the current panel instance, if any.
   */
  public static getCurrent(): AgentCapabilitiesPanel | undefined {
    return AgentCapabilitiesPanel.currentPanel;
  }

  private constructor(
    panel: vscode.WebviewPanel,
    extensionUri: vscode.Uri,
    client: LanguageClient
  ) {
    this.panel = panel;
    this.extensionUri = extensionUri;
    this.client = client;

    // Set initial HTML content
    this.panel.webview.html = this.getHtmlContent();

    // Handle messages from the webview
    this.panel.webview.onDidReceiveMessage(
      (message) => {
        if (message.type === "refresh" && this.currentUri) {
          void this.refresh(this.currentUri);
        }
      },
      undefined,
      this.disposables
    );

    // Clean up on dispose
    this.panel.onDidDispose(
      () => this.dispose(),
      undefined,
      this.disposables
    );
  }

  /**
   * Refresh the panel with capabilities for the given file URI.
   */
  public async refresh(uri: string): Promise<void> {
    this.currentUri = uri;

    // Show loading state
    this.panel.webview.postMessage({ type: "loading" });

    try {
      // Send custom LSP request
      const result = await this.client.sendRequest(
        "tenor/agentCapabilities",
        { uri }
      );

      // Inject SVG diagrams for entities
      const caps = result as Record<string, unknown>;
      const entities = caps.entities as EntityView[] | undefined;
      if (entities) {
        for (const entity of entities) {
          (entity as EntityView & { svg?: string }).svg =
            renderStateMachine(entity);
        }
      }

      this.panel.webview.postMessage({ type: "update", data: caps });
    } catch (err) {
      const message =
        err instanceof Error ? err.message : "Unknown error";
      this.panel.webview.postMessage({ type: "error", message });
    }
  }

  /**
   * Update the panel directly with capabilities data (for notifications).
   */
  public updateWithData(caps: unknown): void {
    // Inject SVG diagrams for entities
    const data = caps as Record<string, unknown>;
    const entities = data.entities as EntityView[] | undefined;
    if (entities) {
      for (const entity of entities) {
        (entity as EntityView & { svg?: string }).svg =
          renderStateMachine(entity);
      }
    }

    this.panel.webview.postMessage({ type: "update", data: caps });
  }

  /**
   * Check if the panel is currently visible.
   */
  public isVisible(): boolean {
    return this.panel.visible;
  }

  private getHtmlContent(): string {
    const webview = this.panel.webview;
    const cssUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.extensionUri, "media", "panel.css")
    );
    const jsUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.extensionUri, "media", "panel.js")
    );

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <link rel="stylesheet" href="${cssUri}">
  <title>Agent Capabilities</title>
</head>
<body>
  <h1>Agent Capabilities</h1>
  <div id="root">
    <div class="loading">
      <div class="spinner"></div>
      <div>Analyzing contract...</div>
    </div>
  </div>
  <script src="${jsUri}"></script>
</body>
</html>`;
  }

  private dispose(): void {
    AgentCapabilitiesPanel.currentPanel = undefined;
    this.panel.dispose();
    for (const d of this.disposables) {
      d.dispose();
    }
    this.disposables = [];
  }
}
