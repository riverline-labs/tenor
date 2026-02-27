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
    <div className="rounded border border-blue-200 bg-blue-50 p-3">
      <p className="mb-2 text-sm font-medium text-blue-800">
        {summary.total} construct{summary.total !== 1 ? "s" : ""} to import:
      </p>
      <div className="flex flex-wrap gap-2">
        {items.map(({ label, count }) => (
          <span
            key={label}
            className="rounded bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700"
          >
            {count} {label}
          </span>
        ))}
        {items.length === 0 && (
          <span className="text-xs text-blue-600">No constructs</span>
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
      <div className="rounded border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-700">
        Bundle is valid.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      {validation.errors.length > 0 && (
        <div className="rounded border border-red-200 bg-red-50 px-3 py-2">
          <p className="mb-1 text-sm font-medium text-red-700">
            {validation.errors.length} error{validation.errors.length !== 1 ? "s" : ""}:
          </p>
          <ul className="list-inside list-disc text-xs text-red-600">
            {validation.errors.map((e, i) => <li key={i}>{e}</li>)}
          </ul>
        </div>
      )}
      {validation.warnings.length > 0 && (
        <div className="rounded border border-yellow-200 bg-yellow-50 px-3 py-2">
          <p className="mb-1 text-sm font-medium text-yellow-700">
            {validation.warnings.length} warning{validation.warnings.length !== 1 ? "s" : ""}:
          </p>
          <ul className="list-inside list-disc text-xs text-yellow-600">
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
      let state: ImportState;
      try {
        let bundle: InterchangeBundle;
        if (format === "json") {
          bundle = importInterchangeJson(content);
        } else {
          bundle = importTenorFile(content);
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
          dragging ? "border-blue-400 bg-blue-50" : "border-gray-300 bg-gray-50"
        }`}
      >
        <p className="mb-2 text-sm text-gray-600">
          Drop a <code>.tenor</code> or <code>.json</code> file here
        </p>
        <p className="mb-3 text-xs text-gray-400">— or —</p>
        <button
          onClick={() => inputRef.current?.click()}
          className="rounded border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50"
        >
          Browse files
        </button>
        <input
          ref={inputRef}
          type="file"
          accept=".tenor,.json"
          className="hidden"
          onChange={handleFileChange}
        />
      </div>

      {loading && (
        <p className="text-sm text-gray-500">Reading {fileName ?? "file"}...</p>
      )}

      {!loading && fileName && (
        <p className="text-xs text-gray-500">Selected: {fileName}</p>
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
      <label className="text-sm font-medium text-gray-700">Contract URL</label>
      <div className="flex gap-2">
        <input
          type="url"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://example.com/contracts/my-contract"
          className="flex-1 rounded border border-gray-300 px-3 py-2 text-sm focus:border-blue-400 focus:outline-none"
          onKeyDown={(e) => {
            if (e.key === "Enter" && url.trim()) void handleFetch();
          }}
        />
        <button
          onClick={() => { void handleFetch(); }}
          disabled={!url.trim() || loading}
          className="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? "Fetching..." : "Fetch"}
        </button>
      </div>
      <p className="text-xs text-gray-500">
        Appends <code>/.well-known/tenor</code> if no <code>.json</code> extension.
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

  function handleImport() {
    const content = text.trim();
    if (!content) return;

    let state: ImportState;
    try {
      let bundle: InterchangeBundle;
      if (content.startsWith("{")) {
        bundle = importInterchangeJson(content);
      } else {
        bundle = importTenorFile(content);
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
      <label className="text-sm font-medium text-gray-700">
        Paste interchange JSON or .tenor source
      </label>
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder='{"kind": "Bundle", "id": "my-contract", ...}'
        className="h-40 resize-y rounded border border-gray-300 px-3 py-2 font-mono text-xs focus:border-blue-400 focus:outline-none"
      />
      <p className="text-xs text-gray-500">
        JSON (starts with <code>{"{"}</code>) is parsed as interchange bundle.
        All other text is treated as .tenor DSL.
      </p>
      <button
        onClick={handleImport}
        disabled={!text.trim()}
        className="self-start rounded bg-blue-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
      >
        Parse
      </button>
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

  function handleTabChange(tab: ImportTab) {
    setActiveTab(tab);
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

  const tabs: { id: ImportTab; label: string }[] = [
    { id: "file", label: "File" },
    { id: "url", label: "URL" },
    { id: "paste", label: "Paste" },
  ];

  return (
    /* Backdrop */
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {/* Dialog */}
      <div className="flex w-[580px] max-w-full flex-col rounded-lg bg-white shadow-xl">
        {/* Header */}
        <div className="flex items-center justify-between border-b border-gray-200 px-6 py-4">
          <h2 className="text-lg font-semibold text-gray-900">Import Contract</h2>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-gray-600"
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        {/* Tabs */}
        <div className="flex border-b border-gray-200">
          {tabs.map(({ id, label }) => (
            <button
              key={id}
              onClick={() => handleTabChange(id)}
              className={`px-5 py-2.5 text-sm font-medium transition-colors ${
                activeTab === id
                  ? "border-b-2 border-blue-600 text-blue-600"
                  : "text-gray-500 hover:text-gray-700"
              }`}
            >
              {label}
            </button>
          ))}
        </div>

        {/* Body */}
        <div className="flex flex-col gap-4 px-6 py-4">
          {/* Tab content */}
          {activeTab === "file" && <FileTab onLoaded={handleLoaded} />}
          {activeTab === "url" && <UrlTab onLoaded={handleLoaded} />}
          {activeTab === "paste" && <PasteTab onLoaded={handleLoaded} />}

          {/* Import results */}
          {importState.error && (
            <div className="rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700 whitespace-pre-wrap">
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
            <div className="rounded border border-orange-200 bg-orange-50 px-3 py-2 text-sm text-orange-700">
              This will replace your current contract ({bundle.constructs.length} construct
              {bundle.constructs.length !== 1 ? "s" : ""}). This action cannot be undone.
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between border-t border-gray-200 px-6 py-4">
          <button
            onClick={onClose}
            className="rounded px-3 py-1.5 text-sm text-gray-600 hover:bg-gray-100"
          >
            Cancel
          </button>

          {canImport && (
            hasExistingData && !confirming ? (
              <button
                onClick={() => setConfirming(true)}
                className="rounded bg-orange-500 px-4 py-1.5 text-sm font-medium text-white hover:bg-orange-600"
              >
                Replace Contract
              </button>
            ) : (
              <button
                onClick={handleConfirmImport}
                className="rounded bg-blue-600 px-4 py-1.5 text-sm font-medium text-white hover:bg-blue-700"
              >
                {confirming ? "Confirm Replace" : "Import"}
              </button>
            )
          )}
        </div>
      </div>
    </div>
  );
}
