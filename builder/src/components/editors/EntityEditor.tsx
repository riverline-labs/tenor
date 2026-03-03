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
  useEntities,
} from "@/store/contract";
import type { EntityConstruct, Transition } from "@/types/interchange";
import { StateMachine } from "@/components/visualizations/StateMachine";
import { Button } from "@/components/ui/button";

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
  const entities = useEntities();
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
      <aside className="flex w-52 flex-shrink-0 flex-col border-r border-border bg-card">
        <div className="flex items-center justify-between border-b border-border px-3 py-2">
          <span className="text-sm font-semibold text-foreground">Entities</span>
          <Button
            onClick={handleAddEntity}
            size="xs"
            title="Add entity"
          >
            + Add
          </Button>
        </div>
        <ul className="flex-1 overflow-y-auto">
          {entities.length === 0 && (
            <li className="px-3 py-4 text-center text-xs text-muted-foreground">
              No entities yet
            </li>
          )}
          {entities.map((entity) => (
            <li key={entity.id}>
              <button
                onClick={() => handleSelectEntity(entity.id)}
                className={`group flex w-full items-center justify-between px-3 py-2 text-left text-sm transition-colors ${
                  selectedId === entity.id
                    ? "bg-muted font-medium text-primary"
                    : "text-foreground hover:bg-muted"
                }`}
              >
                <span className="truncate font-mono">{entity.id}</span>
                <span className="ml-1 text-xs text-muted-foreground">
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
              <label className="block text-xs text-muted-foreground">Entity ID</label>
              <input
                type="text"
                value={selectedEntity.id}
                onChange={(e) => handleRenameEntity(e.target.value)}
                className="mt-0.5 w-full rounded-md border border-border px-2 py-1 font-mono text-sm focus:border-ring focus:outline-none"
              />
            </div>
            <div className="pt-5">
              <Button
                onClick={() => handleDeleteEntity(selectedEntity.id)}
                variant="outline"
                size="xs"
                className="border-destructive/50 text-destructive hover:bg-destructive/10"
              >
                Delete entity
              </Button>
            </div>
          </div>

          {/* Validation errors */}
          {errors.length > 0 && (
            <div className="mb-3 rounded-md border border-border bg-muted px-3 py-2">
              {errors.map((err, i) => (
                <p key={i} className="text-xs text-muted-foreground">
                  {err}
                </p>
              ))}
            </div>
          )}
          {deleteWarning && (
            <div className="mb-3 rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-xs text-destructive">
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
            <Button
              onClick={() => {
                setAddingState(true);
                setAddTransitionMode(false);
              }}
              variant="outline"
              size="xs"
            >
              Add State
            </Button>
            <Button
              onClick={handleDeleteState}
              disabled={!selectedState}
              variant="outline"
              size="xs"
            >
              Delete State
              {selectedState ? ` (${selectedState})` : ""}
            </Button>
            <Button
              onClick={handleSetInitial}
              disabled={!selectedState || selectedState === selectedEntity.initial}
              variant="outline"
              size="xs"
            >
              Set Initial
              {selectedState ? ` (${selectedState})` : ""}
            </Button>
            <span className="mx-1 text-border">|</span>
            <Button
              onClick={() => {
                setAddTransitionMode((m) => !m);
                setAddingState(false);
              }}
              variant={addTransitionMode ? "secondary" : "outline"}
              size="xs"
              className={addTransitionMode ? "border border-primary/50 text-primary" : ""}
            >
              {addTransitionMode ? "Cancel Add Transition" : "Add Transition"}
            </Button>
            <Button
              onClick={handleDeleteTransition}
              disabled={!selectedTransition}
              variant="outline"
              size="xs"
            >
              Delete Transition
              {selectedTransition
                ? ` (${selectedTransition[0]} → ${selectedTransition[1]})`
                : ""}
            </Button>
          </div>

          {/* Add state inline input */}
          {addingState && (
            <div className="mb-3 flex items-center gap-2 rounded-md border border-primary/50 bg-muted p-2">
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
                className="flex-1 rounded-md border border-border px-2 py-1 text-sm"
              />
              <Button
                onClick={handleAddState}
                disabled={!newStateName.trim()}
                size="xs"
              >
                Add
              </Button>
              <Button
                onClick={() => {
                  setAddingState(false);
                  setNewStateName("");
                }}
                variant="outline"
                size="xs"
              >
                Cancel
              </Button>
            </div>
          )}

          {addTransitionMode && (
            <p className="mb-3 text-xs text-primary">
              Click a source state in the diagram, then click the target state.
            </p>
          )}

          {/* Transition list */}
          <div>
            <h3 className="mb-1 text-xs font-semibold text-muted-foreground uppercase">
              Transitions ({selectedEntity.transitions.length})
            </h3>
            {selectedEntity.transitions.length === 0 ? (
              <p className="text-xs text-muted-foreground">No transitions defined.</p>
            ) : (
              <ul className="divide-y divide-border rounded-md border border-border bg-card">
                {selectedEntity.transitions.map((t, i) => {
                  const isSel =
                    selectedTransition?.[0] === t.from &&
                    selectedTransition?.[1] === t.to;
                  return (
                    <li
                      key={i}
                      className={`flex items-center justify-between px-3 py-1.5 text-sm transition-colors ${
                        isSel ? "bg-destructive/10" : "hover:bg-muted"
                      }`}
                      onClick={() => {
                        setSelectedTransition([t.from, t.to]);
                        setSelectedState(null);
                      }}
                    >
                      <span className="font-mono text-xs">
                        {t.from}{" "}
                        <span className="text-muted-foreground">→</span>{" "}
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
                        className="rounded-md px-1 py-0.5 text-xs text-destructive hover:bg-destructive/10"
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
            <h3 className="mb-1 text-xs font-semibold text-muted-foreground uppercase">
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
                  className={`rounded-md px-2 py-0.5 text-xs font-mono transition-colors ${
                    state === selectedEntity.initial
                      ? "border border-primary/50 bg-muted text-primary"
                      : selectedState === state
                      ? "border border-destructive/50 bg-destructive/10 text-destructive"
                      : "border border-border bg-muted text-muted-foreground hover:bg-muted"
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
        <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
          Select an entity or click "+ Add" to create one.
        </div>
      )}
    </div>
  );
}
