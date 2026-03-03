/**
 * Import dialog for loading contracts from file, URL, or pasted text.
 *
 * Import source tabs:
 * 1. File — drag-and-drop or file picker (.tenor and .json)
 * 2. URL — fetch from a URL or /.well-known/tenor endpoint
 * 3. Paste — paste JSON directly
 *
 * Post-import:
 * - Validates the bundle structure
 * - Shows construct count preview
 * - "Replace" replaces the current contract
 * - Warning before replacing existing data
 */
import React, { useState, useRef } from "react";
import {
  importInterchangeJson,
  importTenorFile,
  importFromUrl,
  validateImportedBundle,
  summarizeBundle,
  type ImportValidationResult,
  type ConstructSummary,
} from "@/utils/import";
import { useContractStore } from "@/store/contract";
import type { InterchangeBundle } from "@/types/interchange";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type ImportTab = "file" | "url" | "paste";

interface ImportDialogProps {
  onClose: () => void;
  onImported?: () => void;
}

interface ImportState {
  bundle: InterchangeBundle | null;
  validation: ImportValidationResult | null;
  summary: ConstructSummary | null;
  error: string | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function detectFormat(fileName: string, content: string): "json" | "tenor" {
  if (fileName.endsWith(".json")) return "json";
  if (fileName.endsWith(".tenor")) return "tenor";
  // Auto-detect by content
  return content.trimStart().startsWith("{") ? "json" : "tenor";
}

function initialImportState(): ImportState {
  return { bundle: null, validation: null, summary: null, error: null };
}

// ---------------------------------------------------------------------------
// ConstructSummaryView
// ---------------------------------------------------------------------------

function ConstructSummaryView({ summary }: { summary: ConstructSummary }) {
  const items = [
    { label: "Facts", count: summary.facts },
    { label: "Entities", count: summary.entities },
    { label: "Rules", count: summary.rules },
    { label: "Operations", count: summary.operations },
    { label: "Flows", count: summary.flows },
    { label: "Personas", count: summary.personas },
    { label: "Sources", count: summary.sources },
    { label: "Systems", count: summary.systems },
  ].filter((i) => i.count > 0);

  return (
    <div className="rounded-md border border-primary/30 bg-primary/10 p-3">
      <p className="mb-2 text-sm font-medium text-foreground">
        {summary.total} construct{summary.total !== 1 ? "s" : ""} to import:
      </p>
      <div className="flex flex-wrap gap-2">
        {items.map(({ label, count }) => (
          <Badge key={label} variant="secondary">
            {count} {label}
          </Badge>
        ))}
        {items.length === 0 && (
          <span className="text-xs text-muted-foreground">No constructs</span>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ValidationView
// ---------------------------------------------------------------------------

function ValidationView({ validation }: { validation: ImportValidationResult }) {
  if (validation.valid && validation.warnings.length === 0) {
    return (
      <div className="rounded-md border border-border bg-muted px-3 py-2 text-sm text-muted-foreground">
        Bundle is valid.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {validation.errors.length > 0 && (
        <div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2">
          <p className="mb-1 text-sm font-medium text-destructive">
            {validation.errors.length} error{validation.errors.length !== 1 ? "s" : ""}:
          </p>
          <ul className="list-inside list-disc text-xs text-destructive">
            {validation.errors.map((e, i) => <li key={i}>{e}</li>)}
          </ul>
        </div>
      )}
      {validation.warnings.length > 0 && (
        <div className="rounded-md border border-border bg-muted px-3 py-2">
          <p className="mb-1 text-sm font-medium text-muted-foreground">
            {validation.warnings.length} warning{validation.warnings.length !== 1 ? "s" : ""}:
          </p>
          <ul className="list-inside list-disc text-xs text-muted-foreground">
            {validation.warnings.map((w, i) => <li key={i}>{w}</li>)}
          </ul>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// File tab
// ---------------------------------------------------------------------------

function FileTab({
  onLoaded,
}: {
  onLoaded: (state: ImportState) => void;
}) {
  const [dragging, setDragging] = useState(false);
  const [fileName, setFileName] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  function processFile(file: File) {
    setFileName(file.name);
    setLoading(true);
    const reader = new FileReader();
    reader.onload = (ev) => {
      const content = ev.target?.result as string;
      const format = detectFormat(file.name, content);
      void (async () => {
        let state: ImportState;
        try {
          let bundle: InterchangeBundle;
          if (format === "json") {
            bundle = importInterchangeJson(content);
          } else {
            bundle = await importTenorFile(content, file.name);
          }
          const validation = validateImportedBundle(bundle);
          const summary = summarizeBundle(bundle);
          state = { bundle, validation, summary, error: null };
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          state = { bundle: null, validation: null, summary: null, error: msg };
        }
        setLoading(false);
        onLoaded(state);
      })();
    };
    reader.onerror = () => {
      setLoading(false);
      onLoaded({
        bundle: null,
        validation: null,
        summary: null,
        error: "Failed to read file.",
      });
    };
    reader.readAsText(file);
  }

  function handleFileChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (file) processFile(file);
  }

  function handleDrop(e: React.DragEvent) {
    e.preventDefault();
    setDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (file) processFile(file);
  }

  return (
    <div className="flex flex-col gap-3">
      {/* Drop zone */}
      <div
        onDragOver={(e) => { e.preventDefault(); setDragging(true); }}
        onDragLeave={() => setDragging(false)}
        onDrop={handleDrop}
        className={`flex flex-col items-center justify-center rounded-lg border-2 border-dashed p-8 transition-colors ${
          dragging ? "border-primary bg-primary/10" : "border-border bg-muted"
        }`}
      >
        <p className="mb-2 text-sm text-muted-foreground">
          Drop a <code className="font-mono">.tenor</code> or <code className="font-mono">.json</code> file here
        </p>
        <p className="mb-3 text-xs text-muted-foreground">&mdash; or &mdash;</p>
        <Button
          variant="outline"
          size="sm"
          onClick={() => inputRef.current?.click()}
        >
          Browse files
        </Button>
        <input
          ref={inputRef}
          type="file"
          accept=".tenor,.json"
          className="hidden"
          onChange={handleFileChange}
        />
      </div>

      {loading && (
        <p className="text-sm text-muted-foreground">Reading {fileName ?? "file"}...</p>
      )}

      {!loading && fileName && (
        <p className="text-xs text-muted-foreground">Selected: {fileName}</p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// URL tab
// ---------------------------------------------------------------------------

function UrlTab({
  onLoaded,
}: {
  onLoaded: (state: ImportState) => void;
}) {
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleFetch() {
    setLoading(true);
    let state: ImportState;
    try {
      const bundle = await importFromUrl(url);
      const validation = validateImportedBundle(bundle);
      const summary = summarizeBundle(bundle);
      state = { bundle, validation, summary, error: null };
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      state = { bundle: null, validation: null, summary: null, error: msg };
    }
    setLoading(false);
    onLoaded(state);
  }

  return (
    <div className="flex flex-col gap-3">
      <label className="text-sm font-medium text-foreground">Contract URL</label>
      <div className="flex gap-2">
        <Input
          type="url"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://example.com/contracts/my-contract"
          className="flex-1"
          onKeyDown={(e) => {
            if (e.key === "Enter" && url.trim()) void handleFetch();
          }}
        />
        <Button
          onClick={() => { void handleFetch(); }}
          disabled={!url.trim() || loading}
        >
          {loading ? "Fetching..." : "Fetch"}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        Appends <code className="font-mono">/.well-known/tenor</code> if no <code className="font-mono">.json</code> extension.
        Requires the server to set CORS headers.
      </p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Paste tab
// ---------------------------------------------------------------------------

function PasteTab({
  onLoaded,
}: {
  onLoaded: (state: ImportState) => void;
}) {
  const [text, setText] = useState("");

  async function handleImport() {
    const content = text.trim();
    if (!content) return;

    let state: ImportState;
    try {
      let bundle: InterchangeBundle;
      if (content.startsWith("{")) {
        bundle = importInterchangeJson(content);
      } else {
        bundle = await importTenorFile(content);
      }
      const validation = validateImportedBundle(bundle);
      const summary = summarizeBundle(bundle);
      state = { bundle, validation, summary, error: null };
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      state = { bundle: null, validation: null, summary: null, error: msg };
    }
    onLoaded(state);
  }

  return (
    <div className="flex flex-col gap-3">
      <label className="text-sm font-medium text-foreground">
        Paste interchange JSON or .tenor source
      </label>
      <Textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder='{"kind": "Bundle", "id": "my-contract", ...}'
        className="h-40 resize-y font-mono text-xs"
      />
      <p className="text-xs text-muted-foreground">
        JSON (starts with <code className="font-mono">{"{"}</code>) is parsed as interchange bundle.
        All other text is treated as .tenor DSL.
      </p>
      <Button
        onClick={() => { void handleImport(); }}
        disabled={!text.trim()}
        className="self-start"
      >
        Parse
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// ImportDialog component
// ---------------------------------------------------------------------------

export function ImportDialog({ onClose, onImported }: ImportDialogProps) {
  const [activeTab, setActiveTab] = useState<ImportTab>("file");
  const [importState, setImportState] = useState<ImportState>(initialImportState());
  const [confirming, setConfirming] = useState(false);

  const bundle = useContractStore((s) => s.bundle);
  const loadBundle = useContractStore((s) => s.loadBundle);
  const hasExistingData = bundle.constructs.length > 0;

  function handleLoaded(state: ImportState) {
    setImportState(state);
    setConfirming(false);
  }

  function handleTabChange(tab: string) {
    setActiveTab(tab as ImportTab);
    setImportState(initialImportState());
    setConfirming(false);
  }

  function handleConfirmImport() {
    if (!importState.bundle) return;
    loadBundle(importState.bundle);
    onImported?.();
    onClose();
  }

  const canImport =
    importState.bundle !== null &&
    importState.validation?.valid === true;

  return (
    <Dialog open onOpenChange={() => onClose()}>
      <DialogContent className="sm:max-w-[580px]">
        <DialogHeader>
          <DialogTitle>Import Contract</DialogTitle>
        </DialogHeader>

        <Tabs value={activeTab} onValueChange={handleTabChange}>
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="file">File</TabsTrigger>
            <TabsTrigger value="url">URL</TabsTrigger>
            <TabsTrigger value="paste">Paste</TabsTrigger>
          </TabsList>
          <TabsContent value="file" className="mt-4">
            <FileTab onLoaded={handleLoaded} />
          </TabsContent>
          <TabsContent value="url" className="mt-4">
            <UrlTab onLoaded={handleLoaded} />
          </TabsContent>
          <TabsContent value="paste" className="mt-4">
            <PasteTab onLoaded={handleLoaded} />
          </TabsContent>
        </Tabs>

        {/* Import results */}
        <div className="flex flex-col gap-3">
          {importState.error && (
            <div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive whitespace-pre-wrap">
              {importState.error}
            </div>
          )}

          {importState.validation && (
            <ValidationView validation={importState.validation} />
          )}

          {importState.summary && (
            <ConstructSummaryView summary={importState.summary} />
          )}

          {/* Replace warning */}
          {canImport && hasExistingData && !confirming && (
            <div className="rounded-md border border-border bg-muted px-3 py-2 text-sm text-muted-foreground">
              This will replace your current contract ({bundle.constructs.length} construct
              {bundle.constructs.length !== 1 ? "s" : ""}). This action cannot be undone.
            </div>
          )}
        </div>

        <DialogFooter className="flex items-center justify-between sm:justify-between">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>

          {canImport && (
            hasExistingData && !confirming ? (
              <Button
                variant="destructive"
                onClick={() => setConfirming(true)}
              >
                Replace Contract
              </Button>
            ) : (
              <Button onClick={handleConfirmImport}>
                {confirming ? "Confirm Replace" : "Import"}
              </Button>
            )
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
