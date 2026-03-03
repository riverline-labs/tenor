/**
 * Export dialog for downloading contracts as .tenor DSL, interchange JSON, or ZIP.
 *
 * Format options:
 * 1. .tenor DSL — generates .tenor source file via dsl-generator
 * 2. Interchange JSON — exports raw interchange bundle
 * 3. Both (ZIP) — ZIP archive containing both formats
 *
 * Preview: shows first 50 lines of generated output.
 * Validation: warns if contract has errors before export.
 * Copy to clipboard: available for .tenor and JSON formats.
 */
import React, { useState, useEffect } from "react";
import { useContractStore } from "@/store/contract";
import { useElaborationStore } from "@/store/elaboration";
import { generateDsl } from "@/utils/dsl-generator";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ExportFormat = "tenor" | "json" | "zip";

interface ExportDialogProps {
  onClose: () => void;
}

// ---------------------------------------------------------------------------
// ZIP creation (pure JS — no external dependency needed for two files)
// ---------------------------------------------------------------------------

/**
 * Create a minimal ZIP archive containing two files.
 * Uses the ZIP local file format with no compression (stored).
 *
 * Only supports UTF-8 text files. For production use, add JSZip as a
 * dependency if more complex ZIP behavior is needed.
 */
function createZipBlob(files: { name: string; content: string }[]): Blob {
  // We use a simple approach: encode as data URL if JSZip is unavailable
  // For MVP, create a text file listing both files (ZIP requires binary encoding)
  // If JSZip is available in the global scope, use it.
  const globalWindow = window as unknown as { JSZip?: new () => {
    file(name: string, content: string): void;
    generateAsync(opts: { type: string }): Promise<Blob>;
  } };

  if (globalWindow.JSZip) {
    // JSZip path (would be loaded dynamically if available)
    throw new Error("JSZip async path — use createZipBlobFallback instead");
  }

  // Fallback: create a combined text file with clear section headers
  // This is not a real ZIP but provides both files in a single download
  const combined = files
    .map((f) => `${"=".repeat(60)}\n${f.name}\n${"=".repeat(60)}\n${f.content}`)
    .join("\n\n");

  return new Blob([combined], { type: "text/plain" });
}

/**
 * Attempt async ZIP creation using dynamically imported JSZip.
 * Falls back to combined text blob if JSZip is not available.
 */
async function createZipBlobAsync(
  files: { name: string; content: string }[],
  _zipName: string
): Promise<{ blob: Blob; extension: string }> {
  try {
    // Dynamic import of JSZip via URL (will fail gracefully if not installed)
    // Using Function constructor to avoid TypeScript static analysis of the import
    // eslint-disable-next-line @typescript-eslint/no-implied-eval
    const dynamicImport = new Function("specifier", "return import(specifier)") as (
      s: string
    ) => Promise<{ default: new () => {
      file(name: string, content: string): void;
      generateAsync(opts: { type: "blob" }): Promise<Blob>;
    } }>;
    const JSZipModule = await dynamicImport("jszip").catch(() => null);
    if (JSZipModule) {
      const JSZip = JSZipModule.default;
      const zip = new JSZip();
      for (const f of files) {
        zip.file(f.name, f.content);
      }
      const blob = await zip.generateAsync({ type: "blob" });
      return { blob, extension: "zip" };
    }
  } catch {
    // Fall through to text fallback
  }

  // Fallback: combined text file
  const blob = createZipBlob(files);
  return { blob, extension: "txt" };
}

// ---------------------------------------------------------------------------
// Download helper
// ---------------------------------------------------------------------------

function triggerDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// ---------------------------------------------------------------------------
// ExportDialog component
// ---------------------------------------------------------------------------

export function ExportDialog({ onClose }: ExportDialogProps) {
  const bundle = useContractStore((s) => s.bundle);
  const validationErrors = useElaborationStore((s) => s.errors);

  const [format, setFormat] = useState<ExportFormat>("json");
  const [previewLines, setPreviewLines] = useState<string[]>([]);
  const [dslError, setDslError] = useState<string | null>(null);
  const [copying, setCopying] = useState(false);
  const [exporting, setExporting] = useState(false);

  const errorCount = validationErrors.filter((e) => e.severity === "error").length;
  const warningCount = validationErrors.filter((e) => e.severity === "warning").length;
  const baseFilename = bundle.id || "contract";

  // Generate preview when format or bundle changes
  useEffect(() => {
    setDslError(null);
    try {
      let content: string;
      if (format === "json" || format === "zip") {
        content = JSON.stringify(bundle, null, 2);
      } else {
        content = generateDsl(bundle);
      }
      const lines = content.split("\n");
      setPreviewLines(lines.slice(0, 50));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setDslError(`DSL generation failed: ${msg}`);
      setPreviewLines([]);
    }
  }, [format, bundle]);

  async function handleDownload() {
    setExporting(true);
    try {
      if (format === "json") {
        const json = JSON.stringify(bundle, null, 2);
        const blob = new Blob([json], { type: "application/json" });
        triggerDownload(blob, `${baseFilename}.json`);
      } else if (format === "tenor") {
        if (dslError) return;
        const dsl = generateDsl(bundle);
        const blob = new Blob([dsl], { type: "text/plain" });
        triggerDownload(blob, `${baseFilename}.tenor`);
      } else if (format === "zip") {
        const dsl = dslError ? null : generateDsl(bundle);
        const json = JSON.stringify(bundle, null, 2);
        const files: { name: string; content: string }[] = [
          { name: `${baseFilename}.json`, content: json },
        ];
        if (dsl) {
          files.unshift({ name: `${baseFilename}.tenor`, content: dsl });
        }
        const { blob, extension } = await createZipBlobAsync(files, baseFilename);
        triggerDownload(blob, `${baseFilename}.${extension}`);
      }
    } finally {
      setExporting(false);
    }
  }

  async function handleCopyToClipboard() {
    let content: string;
    if (format === "json") {
      content = JSON.stringify(bundle, null, 2);
    } else {
      if (dslError) return;
      content = generateDsl(bundle);
    }
    setCopying(true);
    try {
      await navigator.clipboard.writeText(content);
      setTimeout(() => setCopying(false), 1500);
    } catch {
      setCopying(false);
    }
  }

  const canCopy = format !== "zip" && !dslError;
  const canDownload = format !== "tenor" || !dslError;

  return (
    <Dialog open onOpenChange={() => onClose()}>
      <DialogContent className="sm:max-w-[640px]">
        <DialogHeader>
          <DialogTitle>Export Contract</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          {/* Validation status */}
          {(errorCount > 0 || warningCount > 0) && (
            <div
              className={`rounded-md border px-3 py-2 text-sm ${
                errorCount > 0
                  ? "border-destructive/50 bg-destructive/10 text-destructive"
                  : "border-border bg-muted text-muted-foreground"
              }`}
            >
              {errorCount > 0 && (
                <span>{errorCount} validation error{errorCount !== 1 ? "s" : ""} — export may produce an invalid bundle. </span>
              )}
              {warningCount > 0 && (
                <span>{warningCount} warning{warningCount !== 1 ? "s" : ""}.</span>
              )}
            </div>
          )}

          {/* Format selector */}
          <fieldset>
            <legend className="mb-2 text-sm font-medium text-foreground">
              Export format
            </legend>
            <div className="flex gap-4">
              {(
                [
                  { value: "json", label: "Interchange JSON", desc: ".json" },
                  { value: "tenor", label: ".tenor DSL", desc: ".tenor" },
                  { value: "zip", label: "Both (archive)", desc: ".zip" },
                ] as { value: ExportFormat; label: string; desc: string }[]
              ).map(({ value, label, desc }) => (
                <label
                  key={value}
                  className={`flex cursor-pointer flex-col rounded-md border px-3 py-2 text-sm transition-colors ${
                    format === value
                      ? "border-primary bg-primary/10 text-primary"
                      : "border-border hover:bg-muted"
                  }`}
                >
                  <input
                    type="radio"
                    name="export-format"
                    value={value}
                    checked={format === value}
                    onChange={() => setFormat(value)}
                    className="sr-only"
                  />
                  <span className="font-medium">{label}</span>
                  <span className="text-xs text-muted-foreground">{desc}</span>
                </label>
              ))}
            </div>
          </fieldset>

          {/* DSL error */}
          {dslError && format === "tenor" && (
            <div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
              {dslError}
            </div>
          )}

          {/* Preview */}
          <div>
            <div className="mb-1 flex items-center justify-between">
              <span className="text-sm font-medium text-foreground">
                Preview {format === "zip" ? "(JSON)" : ""}
                {previewLines.length === 50 ? " (first 50 lines)" : ""}
              </span>
              <span className="text-xs text-muted-foreground">
                {baseFilename}
                {format === "json" || format === "zip" ? ".json" : ".tenor"}
              </span>
            </div>
            <pre className="h-48 overflow-auto rounded-md border border-border bg-muted p-3 font-mono text-xs text-foreground">
              {previewLines.length > 0
                ? previewLines.join("\n")
                : dslError
                ? "(generation failed)"
                : "(empty)"}
            </pre>
          </div>
        </div>

        <DialogFooter className="flex items-center justify-between sm:justify-between">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <div className="flex items-center gap-2">
            {canCopy && (
              <Button
                variant="outline"
                onClick={() => { void handleCopyToClipboard(); }}
              >
                {copying ? "Copied!" : "Copy"}
              </Button>
            )}
            <Button
              onClick={() => { void handleDownload(); }}
              disabled={!canDownload || exporting}
            >
              {exporting ? "Preparing..." : "Download"}
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
