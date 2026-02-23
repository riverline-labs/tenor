/**
 * Status bar manager for Tenor.
 *
 * Shows elaboration status in the VS Code status bar:
 * - Checkmark when the active .tenor file is valid
 * - X with error count when there are diagnostics
 * - Spinner while elaboration is in progress
 * - Hidden when no .tenor file is active
 */

import * as vscode from "vscode";

export class StatusBarManager {
  private readonly item: vscode.StatusBarItem;
  private readonly disposables: vscode.Disposable[] = [];

  constructor() {
    this.item = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    );

    // Show/hide based on active editor language
    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor && editor.document.languageId === "tenor") {
          this.item.show();
          this.updateFromDiagnostics(editor.document.uri);
        } else {
          this.hide();
        }
      })
    );

    // Update on diagnostic changes
    this.disposables.push(
      vscode.languages.onDidChangeDiagnostics((e) => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== "tenor") {
          return;
        }
        for (const uri of e.uris) {
          if (uri.toString() === editor.document.uri.toString()) {
            this.updateFromDiagnostics(uri);
            return;
          }
        }
      })
    );

    // Show immediately if a .tenor file is already active
    const activeEditor = vscode.window.activeTextEditor;
    if (activeEditor && activeEditor.document.languageId === "tenor") {
      this.item.show();
      this.updateFromDiagnostics(activeEditor.document.uri);
    }
  }

  /**
   * Show valid state: checkmark icon.
   */
  public showValid(): void {
    this.item.text = "$(check) Tenor";
    this.item.tooltip = "No errors";
    this.item.color = undefined;
    this.item.command = undefined;
    this.item.show();
  }

  /**
   * Show error state: X icon with error count.
   */
  public showErrors(count: number): void {
    this.item.text = `$(x) Tenor: ${count} error(s)`;
    this.item.tooltip = "Click to show errors";
    this.item.color = new vscode.ThemeColor("statusBarItem.errorForeground");
    this.item.command = "workbench.action.showErrorsWarnings";
    this.item.show();
  }

  /**
   * Show loading state: spinner icon.
   */
  public showLoading(): void {
    this.item.text = "$(sync~spin) Tenor";
    this.item.tooltip = "Elaborating...";
    this.item.color = undefined;
    this.item.command = undefined;
    this.item.show();
  }

  /**
   * Hide the status bar item.
   */
  public hide(): void {
    this.item.hide();
  }

  /**
   * Update the status bar from current diagnostics for a URI.
   */
  private updateFromDiagnostics(uri: vscode.Uri): void {
    const diagnostics = vscode.languages.getDiagnostics(uri);
    const errorCount = diagnostics.filter(
      (d) => d.severity === vscode.DiagnosticSeverity.Error
    ).length;

    if (errorCount > 0) {
      this.showErrors(errorCount);
    } else {
      this.showValid();
    }
  }

  /**
   * Dispose all resources.
   */
  public dispose(): void {
    this.item.dispose();
    for (const d of this.disposables) {
      d.dispose();
    }
    this.disposables.length = 0;
  }
}
