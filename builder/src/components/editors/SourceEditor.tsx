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
 *   - Validation: no duplicate IDs, valid protocol
 */
import React, { useState } from "react";
import {
  useContractStore,
  selectSources,
  selectFacts,
} from "@/store/contract";
import type { SourceConstruct } from "@/types/interchange";

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
          <label className="block text-xs text-gray-500">Source ID</label>
          <input
            type="text"
            value={idDraft}
            onChange={(e) => setIdDraft(e.target.value)}
            onBlur={handleIdBlur}
            className={`mt-0.5 w-full rounded border px-2 py-1 font-mono text-sm ${
              idError ? "border-red-400 bg-red-50" : "border-gray-300"
            }`}
          />
          {idError && <p className="mt-0.5 text-xs text-red-500">{idError}</p>}
        </div>
        <div className="pt-5">
          <button
            onClick={() => onDelete(source.id)}
            className="rounded border border-red-200 px-2 py-1 text-xs text-red-500 hover:bg-red-50"
          >
            Delete source
          </button>
        </div>
      </div>

      {/* Protocol */}
      <div>
        <label className="block text-xs font-medium text-gray-600">
          Protocol
        </label>
        <select
          value={source.protocol}
          onChange={(e) =>
            onUpdate(source.id, { protocol: e.target.value })
          }
          className="mt-0.5 rounded border border-gray-300 px-2 py-1 text-sm"
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
        <label className="block text-xs font-medium text-gray-600">
          {urlLabel(source.protocol as Protocol)}
        </label>
        <input
          type="text"
          value={source.description ?? ""}
          placeholder={urlPlaceholder(source.protocol as Protocol)}
          onChange={(e) =>
            onUpdate(source.id, { description: e.target.value || undefined })
          }
          className="mt-0.5 w-full rounded border border-gray-300 px-2 py-1 font-mono text-sm"
        />
        <p className="mt-0.5 text-xs text-gray-400">
          Stored in the description field of the Source construct.
        </p>
      </div>

      {/* Fields */}
      <div>
        <label className="block text-xs font-semibold text-gray-600 uppercase">
          Fields ({fieldEntries.length})
        </label>
        <p className="mb-2 text-xs text-gray-400">
          Map logical field names to data paths (e.g. JSON path, column name).
        </p>

        {fieldEntries.length > 0 && (
          <div className="mb-2 divide-y divide-gray-100 rounded border border-gray-200 bg-white">
            {fieldEntries.map(([name, path]) => (
              <div key={name} className="flex items-center gap-2 px-3 py-1.5">
                <input
                  type="text"
                  defaultValue={name}
                  onBlur={(e) => handleRenameField(name, e.target.value)}
                  className="w-32 rounded border border-gray-200 px-2 py-0.5 font-mono text-xs"
                  title="Field name"
                />
                <span className="text-gray-400">→</span>
                <input
                  type="text"
                  value={path}
                  onChange={(e) => handleUpdateField(name, e.target.value)}
                  className="flex-1 rounded border border-gray-200 px-2 py-0.5 font-mono text-xs"
                  title="Path / column / expression"
                  placeholder="$.path or column_name"
                />
                <button
                  onClick={() => handleDeleteField(name)}
                  className="rounded px-1 py-0.5 text-xs text-red-400 hover:bg-red-50"
                  title="Remove field"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}

        {/* Add field row */}
        <div className="flex items-center gap-2 rounded border border-dashed border-gray-300 px-3 py-2">
          <input
            type="text"
            value={newFieldName}
            onChange={(e) => setNewFieldName(e.target.value)}
            placeholder="field_name"
            className="w-32 rounded border border-gray-200 px-2 py-0.5 font-mono text-xs"
          />
          <span className="text-gray-400">→</span>
          <input
            type="text"
            value={newFieldPath}
            onChange={(e) => setNewFieldPath(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAddField();
            }}
            placeholder="$.path"
            className="flex-1 rounded border border-gray-200 px-2 py-0.5 font-mono text-xs"
          />
          <button
            onClick={handleAddField}
            disabled={!newFieldName.trim() || !newFieldPath.trim()}
            className="rounded bg-blue-500 px-2 py-0.5 text-xs text-white hover:bg-blue-600 disabled:opacity-40"
          >
            Add
          </button>
        </div>
      </div>

      {/* Referencing facts */}
      {referencingFacts.length > 0 && (
        <div>
          <label className="block text-xs font-semibold text-gray-600 uppercase">
            Referenced by facts
          </label>
          <div className="mt-1 flex flex-wrap gap-1">
            {referencingFacts.map((fid) => (
              <span
                key={fid}
                className="rounded border border-gray-200 bg-gray-50 px-2 py-0.5 font-mono text-xs text-gray-600"
              >
                {fid}
              </span>
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
  const sources = useContractStore(selectSources);
  const facts = useContractStore(selectFacts);
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
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-gray-200 bg-white">
        <div className="flex items-center justify-between border-b border-gray-100 px-3 py-2">
          <span className="text-sm font-semibold text-gray-700">Sources</span>
          <button
            onClick={handleAddSource}
            className="rounded bg-blue-500 px-2 py-0.5 text-xs text-white hover:bg-blue-600"
          >
            + Add
          </button>
        </div>
        <ul className="flex-1 overflow-y-auto">
          {sources.length === 0 && (
            <li className="px-3 py-4 text-center text-xs text-gray-400">
              No sources yet
            </li>
          )}
          {sources.map((source) => (
            <li key={source.id}>
              <button
                onClick={() => setSelectedId(source.id)}
                className={`group flex w-full items-center justify-between px-3 py-2 text-left text-sm transition-colors ${
                  selectedId === source.id
                    ? "bg-blue-50 font-medium text-blue-700"
                    : "text-gray-700 hover:bg-gray-50"
                }`}
              >
                <span className="truncate font-mono">{source.id}</span>
                <span className="ml-1 text-xs text-gray-400">
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
        <div className="flex flex-1 items-center justify-center text-sm text-gray-400">
          Select a source or click "+ Add" to create one.
        </div>
      )}
    </div>
  );
}
