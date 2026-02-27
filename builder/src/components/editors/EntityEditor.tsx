/**
 * EntityEditor: full CRUD editor for Entity constructs.
 *
 * Left panel: entity list (click to select, "+" to add).
 * Right panel (entity selected):
 *   - Entity ID input
 *   - State machine visualization (editable)
 *   - State toolbar: Add State, Delete State, Set Initial
 *   - Transition management: Add Transition mode, Delete Transition
 *   - Transition list
 *   - Inline validation errors
 */
import React, { useState } from "react";
import {
  useContractStore,
  selectEntities,
} from "@/store/contract";
import type { EntityConstruct, Transition } from "@/types/interchange";
import { StateMachine } from "@/components/visualizations/StateMachine";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

function newEntity(id: string): EntityConstruct {
  return {
    id,
    initial: "initial",
    kind: "Entity",
    provenance: { file: "builder", line: 0 },
    states: ["initial"],
    tenor: TENOR_VERSION,
    transitions: [],
  };
}

function validateEntity(entity: EntityConstruct): string[] {
  const errors: string[] = [];
  if (!entity.initial) {
    errors.push("Entity must have an initial state.");
  }
  if (!entity.states.includes(entity.initial)) {
    errors.push(`Initial state "${entity.initial}" is not in the states list.`);
  }
  // Orphan states: states with no transitions in or out (other than initial)
  const connected = new Set<string>();
  entity.transitions.forEach((t) => {
    connected.add(t.from);
    connected.add(t.to);
  });
  const orphans = entity.states.filter(
    (s) => s !== entity.initial && !connected.has(s)
  );
  if (orphans.length > 0) {
    errors.push(`Orphan states (no transitions): ${orphans.join(", ")}`);
  }
  // Duplicate transitions
  const seen = new Set<string>();
  for (const t of entity.transitions) {
    const key = `${t.from}->${t.to}`;
    if (seen.has(key)) {
      errors.push(`Duplicate transition: ${t.from} -> ${t.to}`);
      break;
    }
    seen.add(key);
  }
  return errors;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function EntityEditor() {
  const entities = useContractStore(selectEntities);
  const addConstruct = useContractStore((s) => s.addConstruct);
  const updateConstruct = useContractStore((s) => s.updateConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [addTransitionMode, setAddTransitionMode] = useState(false);
  const [selectedState, setSelectedState] = useState<string | null>(null);
  const [selectedTransition, setSelectedTransition] = useState<
    [string, string] | null
  >(null);
  const [newStateName, setNewStateName] = useState("");
  const [addingState, setAddingState] = useState(false);
  const [deleteWarning, setDeleteWarning] = useState<string | null>(null);

  const selectedEntity = entities.find((e) => e.id === selectedId) ?? null;

  // ---------------------------------------------------------------------------
  // Entity list actions
  // ---------------------------------------------------------------------------

  function handleAddEntity() {
    const baseId = "new_entity";
    let id = baseId;
    let i = 1;
    while (entities.some((e) => e.id === id)) {
      id = `${baseId}_${i++}`;
    }
    addConstruct(newEntity(id));
    setSelectedId(id);
    setSelectedState(null);
    setSelectedTransition(null);
  }

  function handleSelectEntity(id: string) {
    setSelectedId(id);
    setSelectedState(null);
    setSelectedTransition(null);
    setAddTransitionMode(false);
    setAddingState(false);
    setDeleteWarning(null);
  }

  function handleDeleteEntity(id: string) {
    removeConstruct(id, "Entity");
    if (selectedId === id) {
      setSelectedId(null);
    }
  }

  // ---------------------------------------------------------------------------
  // Entity detail actions
  // ---------------------------------------------------------------------------

  function update(updates: Partial<EntityConstruct>) {
    if (!selectedEntity) return;
    updateConstruct(selectedEntity.id, "Entity", updates);
  }

  function handleRenameEntity(newId: string) {
    if (!selectedEntity) return;
    // Rename: remove old, add new
    const updated = { ...selectedEntity, id: newId };
    removeConstruct(selectedEntity.id, "Entity");
    addConstruct(updated);
    setSelectedId(newId);
  }

  // ---------------------------------------------------------------------------
  // State actions
  // ---------------------------------------------------------------------------

  function handleAddState() {
    if (!selectedEntity || !newStateName.trim()) return;
    const name = newStateName.trim();
    if (selectedEntity.states.includes(name)) {
      alert(`State "${name}" already exists.`);
      return;
    }
    update({ states: [...selectedEntity.states, name] });
    setNewStateName("");
    setAddingState(false);
  }

  function handleDeleteState() {
    if (!selectedEntity || !selectedState) return;
    const state = selectedState;

    // Cannot delete initial state
    if (state === selectedEntity.initial) {
      setDeleteWarning(
        `Cannot delete "${state}" — it is the initial state. Set a different initial state first.`
      );
      return;
    }

    // Warn if has transitions
    const hasTransitions = selectedEntity.transitions.some(
      (t) => t.from === state || t.to === state
    );
    if (hasTransitions) {
      const proceed = window.confirm(
        `State "${state}" has transitions. Deleting it will also remove those transitions. Continue?`
      );
      if (!proceed) return;
    }

    const newStates = selectedEntity.states.filter((s) => s !== state);
    const newTransitions = selectedEntity.transitions.filter(
      (t) => t.from !== state && t.to !== state
    );
    update({ states: newStates, transitions: newTransitions });
    setSelectedState(null);
    setDeleteWarning(null);
  }

  function handleSetInitial() {
    if (!selectedEntity || !selectedState) return;
    update({ initial: selectedState });
    setDeleteWarning(null);
  }

  // ---------------------------------------------------------------------------
  // Transition actions
  // ---------------------------------------------------------------------------

  function handleAddTransition(from: string, to: string) {
    if (!selectedEntity) return;
    // Check for duplicate
    const exists = selectedEntity.transitions.some(
      (t) => t.from === from && t.to === to
    );
    if (!exists) {
      update({
        transitions: [...selectedEntity.transitions, { from, to }],
      });
    }
    setAddTransitionMode(false);
  }

  function handleDeleteTransition() {
    if (!selectedEntity || !selectedTransition) return;
    const [from, to] = selectedTransition;
    update({
      transitions: selectedEntity.transitions.filter(
        (t) => !(t.from === from && t.to === to)
      ),
    });
    setSelectedTransition(null);
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  const errors = selectedEntity ? validateEntity(selectedEntity) : [];

  return (
    <div className="flex h-full">
      {/* Left: entity list */}
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-gray-200 bg-white">
        <div className="flex items-center justify-between border-b border-gray-100 px-3 py-2">
          <span className="text-sm font-semibold text-gray-700">Entities</span>
          <button
            onClick={handleAddEntity}
            className="rounded bg-blue-500 px-2 py-0.5 text-xs text-white hover:bg-blue-600"
            title="Add entity"
          >
            + Add
          </button>
        </div>
        <ul className="flex-1 overflow-y-auto">
          {entities.length === 0 && (
            <li className="px-3 py-4 text-center text-xs text-gray-400">
              No entities yet
            </li>
          )}
          {entities.map((entity) => (
            <li key={entity.id}>
              <button
                onClick={() => handleSelectEntity(entity.id)}
                className={`group flex w-full items-center justify-between px-3 py-2 text-left text-sm transition-colors ${
                  selectedId === entity.id
                    ? "bg-blue-50 font-medium text-blue-700"
                    : "text-gray-700 hover:bg-gray-50"
                }`}
              >
                <span className="truncate font-mono">{entity.id}</span>
                <span className="ml-1 text-xs text-gray-400">
                  {entity.states.length}st
                </span>
              </button>
            </li>
          ))}
        </ul>
      </aside>

      {/* Right: entity detail */}
      {selectedEntity ? (
        <main className="flex flex-1 flex-col overflow-y-auto p-4">
          {/* Header */}
          <div className="mb-4 flex items-start gap-4">
            <div className="flex-1">
              <label className="block text-xs text-gray-500">Entity ID</label>
              <input
                type="text"
                value={selectedEntity.id}
                onChange={(e) => handleRenameEntity(e.target.value)}
                className="mt-0.5 w-full rounded border border-gray-300 px-2 py-1 font-mono text-sm focus:border-blue-500 focus:outline-none"
              />
            </div>
            <div className="pt-5">
              <button
                onClick={() => handleDeleteEntity(selectedEntity.id)}
                className="rounded border border-red-200 px-2 py-1 text-xs text-red-500 hover:bg-red-50"
              >
                Delete entity
              </button>
            </div>
          </div>

          {/* Validation errors */}
          {errors.length > 0 && (
            <div className="mb-3 rounded border border-yellow-200 bg-yellow-50 px-3 py-2">
              {errors.map((err, i) => (
                <p key={i} className="text-xs text-yellow-700">
                  {err}
                </p>
              ))}
            </div>
          )}
          {deleteWarning && (
            <div className="mb-3 rounded border border-red-200 bg-red-50 px-3 py-2 text-xs text-red-700">
              {deleteWarning}
            </div>
          )}

          {/* State machine visualization */}
          <div className="mb-3">
            <StateMachine
              states={selectedEntity.states}
              transitions={selectedEntity.transitions.map(
                (t) => [t.from, t.to] as [string, string]
              )}
              initialState={selectedEntity.initial}
              editable={true}
              selectedState={selectedState}
              selectedTransition={selectedTransition}
              onStateClick={(state) => {
                setSelectedState(state);
                setSelectedTransition(null);
                setDeleteWarning(null);
              }}
              onTransitionClick={(from, to) => {
                setSelectedTransition([from, to]);
                setSelectedState(null);
                setDeleteWarning(null);
              }}
              onAddTransition={
                addTransitionMode ? handleAddTransition : undefined
              }
            />
          </div>

          {/* State toolbar */}
          <div className="mb-3 flex flex-wrap gap-2">
            <button
              onClick={() => {
                setAddingState(true);
                setAddTransitionMode(false);
              }}
              className="rounded border border-gray-300 px-3 py-1 text-xs text-gray-600 hover:bg-gray-50"
            >
              Add State
            </button>
            <button
              onClick={handleDeleteState}
              disabled={!selectedState}
              className="rounded border border-gray-300 px-3 py-1 text-xs text-gray-600 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40"
            >
              Delete State
              {selectedState ? ` (${selectedState})` : ""}
            </button>
            <button
              onClick={handleSetInitial}
              disabled={!selectedState || selectedState === selectedEntity.initial}
              className="rounded border border-gray-300 px-3 py-1 text-xs text-gray-600 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40"
            >
              Set Initial
              {selectedState ? ` (${selectedState})` : ""}
            </button>
            <span className="mx-1 text-gray-200">|</span>
            <button
              onClick={() => {
                setAddTransitionMode((m) => !m);
                setAddingState(false);
              }}
              className={`rounded border px-3 py-1 text-xs ${
                addTransitionMode
                  ? "border-blue-400 bg-blue-50 text-blue-600"
                  : "border-gray-300 text-gray-600 hover:bg-gray-50"
              }`}
            >
              {addTransitionMode ? "Cancel Add Transition" : "Add Transition"}
            </button>
            <button
              onClick={handleDeleteTransition}
              disabled={!selectedTransition}
              className="rounded border border-gray-300 px-3 py-1 text-xs text-gray-600 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40"
            >
              Delete Transition
              {selectedTransition
                ? ` (${selectedTransition[0]} → ${selectedTransition[1]})`
                : ""}
            </button>
          </div>

          {/* Add state inline input */}
          {addingState && (
            <div className="mb-3 flex items-center gap-2 rounded border border-blue-200 bg-blue-50 p-2">
              <input
                type="text"
                autoFocus
                value={newStateName}
                onChange={(e) => setNewStateName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleAddState();
                  if (e.key === "Escape") {
                    setAddingState(false);
                    setNewStateName("");
                  }
                }}
                placeholder="State name..."
                className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
              />
              <button
                onClick={handleAddState}
                disabled={!newStateName.trim()}
                className="rounded bg-blue-500 px-2 py-1 text-xs text-white hover:bg-blue-600 disabled:opacity-40"
              >
                Add
              </button>
              <button
                onClick={() => {
                  setAddingState(false);
                  setNewStateName("");
                }}
                className="rounded border border-gray-300 px-2 py-1 text-xs text-gray-500 hover:bg-gray-50"
              >
                Cancel
              </button>
            </div>
          )}

          {addTransitionMode && (
            <p className="mb-3 text-xs text-blue-600">
              Click a source state in the diagram, then click the target state.
            </p>
          )}

          {/* Transition list */}
          <div>
            <h3 className="mb-1 text-xs font-semibold text-gray-500 uppercase">
              Transitions ({selectedEntity.transitions.length})
            </h3>
            {selectedEntity.transitions.length === 0 ? (
              <p className="text-xs text-gray-400">No transitions defined.</p>
            ) : (
              <ul className="divide-y divide-gray-100 rounded border border-gray-200 bg-white">
                {selectedEntity.transitions.map((t, i) => {
                  const isSel =
                    selectedTransition?.[0] === t.from &&
                    selectedTransition?.[1] === t.to;
                  return (
                    <li
                      key={i}
                      className={`flex items-center justify-between px-3 py-1.5 text-sm transition-colors ${
                        isSel ? "bg-red-50" : "hover:bg-gray-50"
                      }`}
                      onClick={() => {
                        setSelectedTransition([t.from, t.to]);
                        setSelectedState(null);
                      }}
                    >
                      <span className="font-mono text-xs">
                        {t.from}{" "}
                        <span className="text-gray-400">→</span>{" "}
                        {t.to}
                      </span>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          update({
                            transitions: selectedEntity.transitions.filter(
                              (_, idx) => idx !== i
                            ),
                          });
                          if (isSel) setSelectedTransition(null);
                        }}
                        className="rounded px-1 py-0.5 text-xs text-red-400 hover:bg-red-50"
                        title="Delete transition"
                      >
                        ×
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>

          {/* States list */}
          <div className="mt-4">
            <h3 className="mb-1 text-xs font-semibold text-gray-500 uppercase">
              States ({selectedEntity.states.length})
            </h3>
            <div className="flex flex-wrap gap-1">
              {selectedEntity.states.map((state) => (
                <button
                  key={state}
                  onClick={() => {
                    setSelectedState(state);
                    setSelectedTransition(null);
                    setDeleteWarning(null);
                  }}
                  className={`rounded px-2 py-0.5 text-xs font-mono transition-colors ${
                    state === selectedEntity.initial
                      ? "border border-blue-300 bg-blue-100 text-blue-700"
                      : selectedState === state
                      ? "border border-red-300 bg-red-100 text-red-700"
                      : "border border-gray-200 bg-gray-100 text-gray-600 hover:bg-gray-200"
                  }`}
                >
                  {state}
                  {state === selectedEntity.initial && (
                    <span className="ml-1 text-[10px] opacity-60">initial</span>
                  )}
                </button>
              ))}
            </div>
          </div>
        </main>
      ) : (
        <div className="flex flex-1 items-center justify-center text-sm text-gray-400">
          Select an entity or click "+ Add" to create one.
        </div>
      )}
    </div>
  );
}
