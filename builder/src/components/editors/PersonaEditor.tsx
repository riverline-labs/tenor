/**
 * PersonaEditor: list management for Persona constructs.
 *
 * - Persona list with inline rename
 * - "Add Persona" button
 * - Delete with warning if persona is referenced by operations
 * - Usage count: operations that reference each persona
 * - Validation: no duplicate IDs
 */
import React, { useState } from "react";
import {
  useContractStore,
  selectPersonas,
  selectOperations,
} from "@/store/contract";
import type { PersonaConstruct } from "@/types/interchange";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newPersona(id: string): PersonaConstruct {
  return {
    id,
    kind: "Persona",
    provenance: { file: "builder", line: 0 },
    tenor: TENOR_VERSION,
  };
}

// ---------------------------------------------------------------------------
// Persona row
// ---------------------------------------------------------------------------

interface PersonaRowProps {
  persona: PersonaConstruct;
  usageCount: number;
  allPersonaIds: string[];
  onRename: (oldId: string, newId: string) => void;
  onDelete: (id: string) => void;
}

function PersonaRow({
  persona,
  usageCount,
  allPersonaIds,
  onRename,
  onDelete,
}: PersonaRowProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(persona.id);
  const [error, setError] = useState<string | null>(null);

  function handleCommit() {
    const trimmed = draft.trim();
    if (!trimmed) {
      setError("ID cannot be empty.");
      setDraft(persona.id);
      setEditing(false);
      return;
    }
    if (trimmed !== persona.id && allPersonaIds.includes(trimmed)) {
      setError(`"${trimmed}" already exists.`);
      setDraft(persona.id);
      setEditing(false);
      return;
    }
    setError(null);
    if (trimmed !== persona.id) {
      onRename(persona.id, trimmed);
    }
    setEditing(false);
  }

  function handleDelete() {
    if (usageCount > 0) {
      const proceed = window.confirm(
        `Persona "${persona.id}" is used in ${usageCount} operation(s). Delete anyway?`
      );
      if (!proceed) return;
    }
    onDelete(persona.id);
  }

  return (
    <li className="flex items-center gap-3 rounded border border-gray-200 bg-white px-3 py-2">
      {/* ID or edit input */}
      <div className="flex flex-1 items-center gap-2">
        {editing ? (
          <input
            autoFocus
            type="text"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={handleCommit}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCommit();
              if (e.key === "Escape") {
                setDraft(persona.id);
                setEditing(false);
                setError(null);
              }
            }}
            className="flex-1 rounded border border-blue-400 px-2 py-0.5 font-mono text-sm"
          />
        ) : (
          <span
            className="flex-1 cursor-text font-mono text-sm text-gray-800"
            onDoubleClick={() => {
              setDraft(persona.id);
              setEditing(true);
            }}
            title="Double-click to rename"
          >
            {persona.id}
          </span>
        )}

        {error && <span className="text-xs text-red-500">{error}</span>}
      </div>

      {/* Usage badge */}
      {usageCount > 0 ? (
        <span
          className="rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-600"
          title={`Used in ${usageCount} operation(s)`}
        >
          {usageCount} op{usageCount !== 1 ? "s" : ""}
        </span>
      ) : (
        <span className="text-xs text-gray-400">unused</span>
      )}

      {/* Actions */}
      <div className="flex items-center gap-1">
        <button
          onClick={() => {
            setDraft(persona.id);
            setEditing(true);
          }}
          className="rounded px-1.5 py-0.5 text-xs text-gray-400 hover:bg-gray-100 hover:text-gray-600"
          title="Rename"
        >
          rename
        </button>
        <button
          onClick={handleDelete}
          className="rounded px-1.5 py-0.5 text-xs text-red-400 hover:bg-red-50"
          title="Delete"
        >
          ×
        </button>
      </div>
    </li>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function PersonaEditor() {
  const personas = useContractStore(selectPersonas);
  const operations = useContractStore(selectOperations);
  const addConstruct = useContractStore((s) => s.addConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  // Build usage map: personaId -> number of operations that reference it
  const usageMap: Record<string, number> = {};
  for (const op of operations) {
    for (const p of op.allowed_personas) {
      usageMap[p] = (usageMap[p] ?? 0) + 1;
    }
  }

  function handleAddPersona() {
    const baseId = "new_persona";
    let id = baseId;
    let i = 1;
    while (personas.some((p) => p.id === id)) {
      id = `${baseId}_${i++}`;
    }
    addConstruct(newPersona(id));
  }

  function handleRename(oldId: string, newId: string) {
    // Renaming: remove old, add new with updated id
    const existing = personas.find((p) => p.id === oldId);
    if (!existing) return;
    removeConstruct(oldId, "Persona");
    addConstruct({ ...existing, id: newId });
  }

  function handleDelete(id: string) {
    removeConstruct(id, "Persona");
  }

  const allPersonaIds = personas.map((p) => p.id);

  // Validation: warn if operations reference personas with no duplicate check
  const hasOperations = operations.length > 0;
  const noPersonas = personas.length === 0;

  return (
    <div className="p-4">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-gray-700">
          Personas ({personas.length})
        </h2>
        <button
          onClick={handleAddPersona}
          className="rounded bg-blue-500 px-3 py-1 text-xs text-white hover:bg-blue-600"
        >
          + Add Persona
        </button>
      </div>

      {/* Validation notice */}
      {hasOperations && noPersonas && (
        <div className="mb-3 rounded border border-yellow-200 bg-yellow-50 px-3 py-2 text-xs text-yellow-700">
          This contract has operations but no personas. Operations require at
          least one persona via <code>allowed_personas</code>.
        </div>
      )}

      <p className="mb-2 text-xs text-gray-500">
        Double-click a persona name to rename it inline.
      </p>

      {personas.length === 0 ? (
        <div className="rounded border-2 border-dashed border-gray-200 py-12 text-center text-sm text-gray-400">
          No personas yet — click "+ Add Persona" to create one.
        </div>
      ) : (
        <ul className="space-y-2">
          {personas.map((persona) => (
            <PersonaRow
              key={persona.id}
              persona={persona}
              usageCount={usageMap[persona.id] ?? 0}
              allPersonaIds={allPersonaIds}
              onRename={handleRename}
              onDelete={handleDelete}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
