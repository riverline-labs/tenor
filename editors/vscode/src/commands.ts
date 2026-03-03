/**
 * Command palette commands for Tenor.
 *
 * Registers all user-facing commands beyond the Agent Capabilities panel:
 * - Elaborate File: runs elaboration and shows output
 * - Validate Project: elaborates all .tenor files in workspace
 * - New Tenor File: scaffolds from templates
 * - Show Elaboration Output (JSON): displays interchange JSON
 * - Run Conformance Tests: executes test suite in terminal
 * - Open Documentation: opens docs in browser or editor
 */

import * as vscode from "vscode";
import { exec } from "child_process";
import { promisify } from "util";

const execAsync = promisify(exec);

/**
 * Get the configured tenor binary path.
 */
function getTenorCommand(): string {
  const config = vscode.workspace.getConfiguration("tenor");
  const customPath = config.get<string>("path", "");
  return customPath || "tenor";
}

/**
 * Register all Tenor commands on the extension context.
 */
export function registerCommands(
  context: vscode.ExtensionContext
): void {
  const outputChannel = vscode.window.createOutputChannel(
    "Tenor Commands"
  );
  context.subscriptions.push(outputChannel);

  // tenor.elaborateFile
  context.subscriptions.push(
    vscode.commands.registerCommand("tenor.elaborateFile", async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "tenor") {
        vscode.window.showWarningMessage(
          "Open a .tenor file first to elaborate."
        );
        return;
      }

      const filePath = editor.document.uri.fsPath;
      const cmd = getTenorCommand();

      try {
        const { stdout, stderr } = await execAsync(
          `${cmd} elaborate "${filePath}"`
        );
        const content = stdout || stderr;
        const doc = await vscode.workspace.openTextDocument({
          content,
          language: "text",
        });
        await vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside);
      } catch (err) {
        const message =
          err instanceof Error ? err.message : "Elaboration failed";
        vscode.window.showErrorMessage(`Tenor elaborate failed: ${message}`);
      }
    })
  );

  // tenor.validateProject
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "tenor.validateProject",
      async () => {
        const tenorFiles = await vscode.workspace.findFiles(
          "**/*.tenor",
          "**/node_modules/**"
        );

        if (tenorFiles.length === 0) {
          vscode.window.showInformationMessage(
            "No .tenor files found in workspace."
          );
          return;
        }

        outputChannel.clear();
        outputChannel.show(true);
        outputChannel.appendLine(
          `Validating ${tenorFiles.length} .tenor file(s)...\n`
        );

        const cmd = getTenorCommand();
        let passed = 0;
        let failed = 0;

        for (const file of tenorFiles) {
          const filePath = file.fsPath;
          const relativePath =
            vscode.workspace.asRelativePath(filePath);

          try {
            await execAsync(`${cmd} elaborate "${filePath}"`);
            outputChannel.appendLine(`  PASS  ${relativePath}`);
            passed++;
          } catch (err) {
            const message =
              err instanceof Error ? err.message : "Unknown error";
            outputChannel.appendLine(`  FAIL  ${relativePath}`);
            outputChannel.appendLine(`        ${message}\n`);
            failed++;
          }
        }

        outputChannel.appendLine("");
        outputChannel.appendLine(
          `Results: ${passed} passed, ${failed} failed, ${tenorFiles.length} total`
        );

        if (failed === 0) {
          vscode.window.showInformationMessage(
            `All ${passed} .tenor files are valid.`
          );
        } else {
          vscode.window.showWarningMessage(
            `${failed} of ${tenorFiles.length} .tenor files have errors. See Tenor Commands output.`
          );
        }
      }
    )
  );

  // tenor.newTenorFile
  context.subscriptions.push(
    vscode.commands.registerCommand("tenor.newTenorFile", async () => {
      const choice = await vscode.window.showQuickPick(
        [
          {
            label: "Empty contract",
            description: "Minimal .tenor file with a single fact",
            template: TEMPLATE_EMPTY,
          },
          {
            label: "Entity + Operation",
            description: "Entity with states and an operation",
            template: TEMPLATE_ENTITY_OP,
          },
          {
            label: "Full contract skeleton",
            description:
              "Complete contract with entity, facts, rules, operations, and flow",
            template: TEMPLATE_FULL,
          },
        ],
        { placeHolder: "Select a template for the new .tenor file" }
      );

      if (!choice) {
        return;
      }

      const doc = await vscode.workspace.openTextDocument({
        content: choice.template,
        language: "tenor",
      });
      await vscode.window.showTextDocument(doc);
    })
  );

  // tenor.showElaborationOutput
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "tenor.showElaborationOutput",
      async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor || editor.document.languageId !== "tenor") {
          vscode.window.showWarningMessage(
            "Open a .tenor file first to show elaboration output."
          );
          return;
        }

        const filePath = editor.document.uri.fsPath;
        const cmd = getTenorCommand();

        try {
          const { stdout } = await execAsync(
            `${cmd} elaborate "${filePath}"`
          );
          const doc = await vscode.workspace.openTextDocument({
            content: stdout,
            language: "json",
          });
          await vscode.window.showTextDocument(
            doc,
            vscode.ViewColumn.Beside
          );
        } catch (err) {
          const message =
            err instanceof Error ? err.message : "Elaboration failed";
          vscode.window.showErrorMessage(
            `Tenor elaborate failed: ${message}`
          );
        }
      }
    )
  );

  // tenor.runConformanceTests
  context.subscriptions.push(
    vscode.commands.registerCommand(
      "tenor.runConformanceTests",
      () => {
        const terminal = vscode.window.createTerminal("Tenor Tests");
        const cmd = getTenorCommand();
        terminal.show();
        terminal.sendText(`${cmd} test conformance`);
      }
    )
  );

  // tenor.openDocs
  context.subscriptions.push(
    vscode.commands.registerCommand("tenor.openDocs", async () => {
      // Try local docs first
      const localDocs = await vscode.workspace.findFiles(
        "docs/author-guide.md",
        null,
        1
      );

      if (localDocs.length > 0) {
        const doc = await vscode.workspace.openTextDocument(localDocs[0]);
        await vscode.window.showTextDocument(doc);
      } else {
        void vscode.env.openExternal(
          vscode.Uri.parse("https://tenor-lang.org/docs")
        );
      }
    })
  );
}

// ── Templates ──────────────────────────────────────────────────────────

const TEMPLATE_EMPTY = `// New Tenor contract

fact amount : Int
`;

const TEMPLATE_ENTITY_OP = `// Entity + Operation template

persona buyer
persona seller

entity Order {
  states: [pending, confirmed, completed]
  initial_state: pending
}

fact order_total : Int

operation confirm_order {
  allowed_personas: [seller]
  precondition: order_total > 0
  effects: {
    Order: pending -> confirmed
  }
}
`;

const TEMPLATE_FULL = `// Full contract skeleton

persona buyer
persona seller

entity Order {
  states: [pending, confirmed, shipped, delivered, cancelled]
  initial_state: pending
}

fact order_total : Int
fact is_paid : Bool = false

rule payment_required {
  when order_total > 0 and not is_paid
  then Reject("Payment is required before confirmation")
}

operation confirm_order {
  allowed_personas: [seller]
  precondition: is_paid
  effects: {
    Order: pending -> confirmed
  }
}

operation ship_order {
  allowed_personas: [seller]
  precondition: is_paid
  effects: {
    Order: confirmed -> shipped
  }
}

operation deliver_order {
  allowed_personas: [buyer]
  effects: {
    Order: shipped -> delivered
  }
}

flow order_flow {
  entry_point: confirm
  steps: {
    confirm: OperationStep {
      operation: confirm_order
      on_success: ship
    }
    ship: OperationStep {
      operation: ship_order
      on_success: deliver
    }
    deliver: OperationStep {
      operation: deliver_order
      on_success: Terminate
    }
  }
}
`;
