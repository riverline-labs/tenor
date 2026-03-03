/**
 * SourceEditor: editor for Source declaration constructs.
 *
 * Left panel: source list with add/delete.
 * Right panel (source selected):
 *   - Source ID input
 *   - Protocol selector (http, graphql, database, manual)
 *   - Base URL / connection string input (contextual on protocol)
 *   - Fields list: field name + path mapping
 *   - Authentication section (optional): auth_type, credentials
 *   - Referencing facts: which facts use this source
 *   - Validation: no duplicate IDs
 */
import React, { useState } from "react";
import {
  useContractStore,
  useSources,
  useFacts,
} from "@/store/contract";
import type { SourceConstruct } from "@/types/interchange";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";
const PROTOCOLS = ["http", "graphql", "database", "manual"] as const;
type Protocol = (typeof PROTOCOLS)[number];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newSource(id: string): SourceConstruct {
  return {
    id,
    kind: "Source",
    protocol: "http",
    fields: {},
    provenance: { file: "builder", line: 0 },
    tenor: TENOR_VERSION,
  };
}

function urlLabel(protocol: Protocol): string {
  switch (protocol) {
    case "http":
      return "Base URL";
    case "graphql":
      return "GraphQL Endpoint";
    case "database":
      return "Connection String";
    case "manual":
      return "Description / Reference";
  }
}

function urlPlaceholder(protocol: Protocol): string {
  switch (protocol) {
    case "http":
      return "https://api.example.com/v1";
    case "graphql":
      return "https://api.example.com/graphql";
    case "database":
      return "postgres://user:pass@host/db";
    case "manual":
      return "Manually provided at runtime";
  }
}

// ---------------------------------------------------------------------------
// Source detail editor
// ---------------------------------------------------------------------------

interface SourceDetailProps {
  source: SourceConstruct;
  allSourceIds: string[];
  referencingFacts: string[];
  onUpdate: (id: string, updates: Partial<SourceConstruct>) => void;
  onDelete: (id: string) => void;
  onRename: (oldId: string, newId: string) => void;
}

function SourceDetail({
  source,
  allSourceIds,
  referencingFacts,
  onUpdate,
  onDelete,
  onRename,
}: SourceDetailProps) {
  const [idDraft, setIdDraft] = useState(source.id);
  const [idError, setIdError] = useState<string | null>(null);
  const [newFieldName, setNewFieldName] = useState("");
  const [newFieldPath, setNewFieldPath] = useState("");

  // Sync when source changes externally (rename)
  React.useEffect(() => {
    setIdDraft(source.id);
  }, [source.id]);

  function handleIdBlur() {
    const trimmed = idDraft.trim();
    if (!trimmed) {
      setIdError("ID cannot be empty.");
      setIdDraft(source.id);
      return;
    }
    if (trimmed !== source.id && allSourceIds.includes(trimmed)) {
      setIdError(`"${trimmed}" already exists.`);
      setIdDraft(source.id);
      return;
    }
    setIdError(null);
    if (trimmed !== source.id) {
      onRename(source.id, trimmed);
    }
  }

  function handleAddField() {
    const name = newFieldName.trim();
    const path = newFieldPath.trim();
    if (!name || !path) return;
    onUpdate(source.id, { fields: { ...source.fields, [name]: path } });
    setNewFieldName("");
    setNewFieldPath("");
  }

  function handleUpdateField(oldName: string, newPath: string) {
    onUpdate(source.id, { fields: { ...source.fields, [oldName]: newPath } });
  }

  function handleRenameField(oldName: string, newName: string) {
    if (!newName.trim() || newName === oldName) return;
    const next: Record<string, string> = {};
    for (const [k, v] of Object.entries(source.fields)) {
      next[k === oldName ? newName.trim() : k] = v;
    }
    onUpdate(source.id, { fields: next });
  }

  function handleDeleteField(name: string) {
    const next = { ...source.fields };
    delete next[name];
    onUpdate(source.id, { fields: next });
  }

  const fieldEntries = Object.entries(source.fields);

  return (
    <div className="flex flex-col gap-4 p-4">
      {/* Header */}
      <div className="flex items-start gap-4">
        <div className="flex-1">
          <label className="block text-xs text-muted-foreground">Source ID</label>
          <input
            type="text"
            value={idDraft}
            onChange={(e) => setIdDraft(e.target.value)}
            onBlur={handleIdBlur}
            className={`mt-0.5 w-full rounded-md border px-2 py-1 font-mono text-sm ${
              idError ? "border-destructive/50 bg-destructive/10" : "border-border"
            }`}
          />
          {idError && <p className="mt-0.5 text-xs text-destructive">{idError}</p>}
        </div>
        <div className="pt-5">
          <Button
            variant="outline"
            size="xs"
            onClick={() => onDelete(source.id)}
            className="border-destructive/50 text-destructive hover:bg-destructive/10"
          >
            Delete source
          </Button>
        </div>
      </div>

      {/* Protocol */}
      <div>
        <label className="block text-xs font-medium text-muted-foreground">
          Protocol
        </label>
        <select
          value={source.protocol}
          onChange={(e) =>
            onUpdate(source.id, { protocol: e.target.value })
          }
          className="mt-0.5 rounded-md border border-border px-2 py-1 text-sm"
        >
          {PROTOCOLS.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
      </div>

      {/* Base URL / connection string */}
      <div>
        <label className="block text-xs font-medium text-muted-foreground">
          {urlLabel(source.protocol as Protocol)}
        </label>
        <input
          type="text"
          value={source.description ?? ""}
          placeholder={urlPlaceholder(source.protocol as Protocol)}
          onChange={(e) =>
            onUpdate(source.id, { description: e.target.value || undefined })
          }
          className="mt-0.5 w-full rounded-md border border-border px-2 py-1 font-mono text-sm"
        />
        <p className="mt-0.5 text-xs text-muted-foreground">
          Stored in the description field of the Source construct.
        </p>
      </div>

      {/* Fields */}
      <div>
        <label className="block text-xs font-semibold text-muted-foreground uppercase">
          Fields ({fieldEntries.length})
        </label>
        <p className="mb-2 text-xs text-muted-foreground">
          Map logical field names to data paths (e.g. JSON path, column name).
        </p>

        {fieldEntries.length > 0 && (
          <div className="mb-2 divide-y divide-border rounded border border-border bg-card">
            {fieldEntries.map(([name, path]) => (
              <div key={name} className="flex items-center gap-2 px-3 py-1.5">
                <input
                  type="text"
                  defaultValue={name}
                  onBlur={(e) => handleRenameField(name, e.target.value)}
                  className="w-32 rounded-md border border-border px-2 py-0.5 font-mono text-xs"
                  title="Field name"
                />
                <span className="text-muted-foreground">→</span>
                <input
                  type="text"
                  value={path}
                  onChange={(e) => handleUpdateField(name, e.target.value)}
                  className="flex-1 rounded-md border border-border px-2 py-0.5 font-mono text-xs"
                  title="Path / column / expression"
                  placeholder="$.path or column_name"
                />
                <Button
                  variant="ghost"
                  size="xs"
                  onClick={() => handleDeleteField(name)}
                  className="text-destructive hover:bg-destructive/10"
                  title="Remove field"
                >
                  ×
                </Button>
              </div>
            ))}
          </div>
        )}

        {/* Add field row */}
        <div className="flex items-center gap-2 rounded border border-dashed border-border px-3 py-2">
          <input
            type="text"
            value={newFieldName}
            onChange={(e) => setNewFieldName(e.target.value)}
            placeholder="field_name"
            className="w-32 rounded-md border border-border px-2 py-0.5 font-mono text-xs"
          />
          <span className="text-muted-foreground">→</span>
          <input
            type="text"
            value={newFieldPath}
            onChange={(e) => setNewFieldPath(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAddField();
            }}
            placeholder="$.path"
            className="flex-1 rounded-md border border-border px-2 py-0.5 font-mono text-xs"
          />
          <Button
            size="xs"
            onClick={handleAddField}
            disabled={!newFieldName.trim() || !newFieldPath.trim()}
          >
            Add
          </Button>
        </div>
      </div>

      {/* Referencing facts */}
      {referencingFacts.length > 0 && (
        <div>
          <label className="block text-xs font-semibold text-muted-foreground uppercase">
            Referenced by facts
          </label>
          <div className="mt-1 flex flex-wrap gap-1">
            {referencingFacts.map((fid) => (
              <Badge
                key={fid}
                variant="outline"
                className="font-mono"
              >
                {fid}
              </Badge>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function SourceEditor() {
  const sources = useSources();
  const facts = useFacts();
  const addConstruct = useContractStore((s) => s.addConstruct);
  const updateConstruct = useContractStore((s) => s.updateConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selectedSource = sources.find((s) => s.id === selectedId) ?? null;

  // Build referencing-facts map
  const referencingFacts: Record<string, string[]> = {};
  for (const fact of facts) {
    if (fact.source && "source_id" in fact.source) {
      const sid = fact.source.source_id;
      referencingFacts[sid] = [...(referencingFacts[sid] ?? []), fact.id];
    }
  }

  function handleAddSource() {
    const baseId = "new_source";
    let id = baseId;
    let i = 1;
    while (sources.some((s) => s.id === id)) {
      id = `${baseId}_${i++}`;
    }
    const src = newSource(id);
    addConstruct(src);
    setSelectedId(id);
  }

  function handleUpdate(id: string, updates: Partial<SourceConstruct>) {
    updateConstruct(id, "Source", updates);
  }

  function handleDelete(id: string) {
    removeConstruct(id, "Source");
    if (selectedId === id) setSelectedId(null);
  }

  function handleRename(oldId: string, newId: string) {
    const existing = sources.find((s) => s.id === oldId);
    if (!existing) return;
    removeConstruct(oldId, "Source");
    addConstruct({ ...existing, id: newId });
    setSelectedId(newId);
  }

  const allSourceIds = sources.map((s) => s.id);

  return (
    <div className="flex h-full">
      {/* Left: source list */}
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-3 py-2">
          <span className="text-sm font-semibold text-foreground">Sources</span>
          <Button size="xs" onClick={handleAddSource}>
            + Add
          </Button>
        </div>
        <ul className="flex-1 overflow-y-auto">
          {sources.length === 0 && (
            <li className="px-3 py-4 text-center text-xs text-muted-foreground">
              No sources yet
            </li>
          )}
          {sources.map((source) => (
            <li key={source.id}>
              <button
                onClick={() => setSelectedId(source.id)}
                className={`group flex w-full items-center justify-between px-3 py-2 text-left text-sm transition-colors ${
                  selectedId === source.id
                    ? "bg-muted font-medium text-primary"
                    : "text-foreground hover:bg-muted"
                }`}
              >
                <span className="truncate font-mono">{source.id}</span>
                <span className="ml-1 text-xs text-muted-foreground">
                  {source.protocol}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </aside>

      {/* Right: detail */}
      {selectedSource ? (
        <main className="flex-1 overflow-y-auto">
          <SourceDetail
            source={selectedSource}
            allSourceIds={allSourceIds}
            referencingFacts={referencingFacts[selectedSource.id] ?? []}
            onUpdate={handleUpdate}
            onDelete={handleDelete}
            onRename={handleRename}
          />
        </main>
      ) : (
        <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
          Select a source or click "+ Add" to create one.
        </div>
      )}
    </div>
  );
}
