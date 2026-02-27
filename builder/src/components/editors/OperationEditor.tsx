/**
 * OperationEditor: CRUD editor for Operation constructs.
 *
 * Layout:
 * - Top: AuthorityMatrix visualization (personas x operations)
 * - Left panel: operation list
 * - Right panel: operation detail editor
 *
 * Each operation has:
 * - ID, allowed_personas (multi-select), precondition (PredicateBuilder)
 * - effects: list of (entity_id, from_state, to_state) tuples
 * - error_contract: multi-select standard error types
 * - outcomes: optional outcome labels
 *
 * Validation:
 * - At least one persona
 * - Effects reference valid entity transitions
 * - error_contract non-empty
 */
import React, { useState, useMemo } from "react";
import {
  useContractStore,
  selectOperations,
  selectPersonas,
  selectFacts,
  selectRules,
  selectEntities,
} from "@/store/contract";
import type {
  OperationConstruct,
  PersonaConstruct,
  EntityConstruct,
  Effect,
  PredicateExpression,
} from "@/types/interchange";
import { PredicateBuilder, ExprTypeDropdown } from "@/components/shared/PredicateBuilder";
import { AuthorityMatrix } from "@/components/visualizations/AuthorityMatrix";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

const STANDARD_ERROR_TYPES = [
  "precondition_failed",
  "persona_rejected",
  "entity_state_mismatch",
  "timeout",
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newOperation(id: string): OperationConstruct {
  return {
    id,
    kind: "Operation",
    provenance: { file: "builder", line: 0 },
    tenor: TENOR_VERSION,
    allowed_personas: [],
    effects: [],
    error_contract: ["precondition_failed"],
    precondition: {
      left: { fact_ref: "fact" },
      op: "=",
      right: { literal: true, type: { base: "Bool" } },
    } as PredicateExpression,
  };
}

function isValidTransition(entity: EntityConstruct, from: string, to: string): boolean {
  return entity.transitions.some((t) => t.from === from && t.to === to);
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

interface OpValidation {
  errors: string[];
  warnings: string[];
}

function validateOperation(
  op: OperationConstruct,
  entities: EntityConstruct[],
  personas: PersonaConstruct[]
): OpValidation {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (op.allowed_personas.length === 0) {
    errors.push("At least one persona must be authorized.");
  }

  const undefinedPersonas = op.allowed_personas.filter(
    (pid) => !personas.some((p) => p.id === pid)
  );
  if (undefinedPersonas.length > 0) {
    warnings.push(
      `Persona(s) not found in contract: ${undefinedPersonas.join(", ")}`
    );
  }

  if (op.error_contract.length === 0) {
    errors.push("Error contract must list at least one error type.");
  }

  for (const effect of op.effects) {
    const entity = entities.find((e) => e.id === effect.entity_id);
    if (!entity) {
      errors.push(`Effect references unknown entity "${effect.entity_id}".`);
    } else if (!isValidTransition(entity, effect.from, effect.to)) {
      errors.push(
        `Effect on "${effect.entity_id}": transition ${effect.from} → ${effect.to} not declared.`
      );
    }
  }

  return { errors, warnings };
}

// ---------------------------------------------------------------------------
// Effect editor row
// ---------------------------------------------------------------------------

interface EffectRowProps {
  effect: Effect;
  entities: EntityConstruct[];
  outcomeLabels: string[];
  onChange: (updated: Effect) => void;
  onDelete: () => void;
}

function EffectRow({ effect, entities, outcomeLabels, onChange, onDelete }: EffectRowProps) {
  const selectedEntity = entities.find((e) => e.id === effect.entity_id) ?? null;
  const availableFromStates = selectedEntity?.states ?? [];
  const availableToStates =
    selectedEntity?.states.filter((s) => s !== effect.from) ?? [];

  // Check if the current transition is valid
  const isValid =
    selectedEntity && isValidTransition(selectedEntity, effect.from, effect.to);

  return (
    <div
      className={`flex flex-wrap items-center gap-2 rounded border p-2 ${
        isValid === false ? "border-red-300 bg-red-50" : "border-gray-200 bg-gray-50"
      }`}
    >
      {/* Entity */}
      <div className="flex flex-col gap-0.5">
        <span className="text-xs text-gray-500">Entity</span>
        <select
          value={effect.entity_id}
          onChange={(e) => {
            const newEnt = entities.find((en) => en.id === e.target.value);
            const newFrom = newEnt?.initial ?? newEnt?.states[0] ?? "";
            const newTo = newEnt?.states.find((s) => s !== newFrom) ?? "";
            onChange({
              ...effect,
              entity_id: e.target.value,
              from: newFrom,
              to: newTo,
            });
          }}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        >
          {entities.length === 0 && (
            <option value="">— no entities —</option>
          )}
          {entities.map((en) => (
            <option key={en.id} value={en.id}>
              {en.id}
            </option>
          ))}
        </select>
      </div>

      {/* From */}
      <div className="flex flex-col gap-0.5">
        <span className="text-xs text-gray-500">From state</span>
        <select
          value={effect.from}
          onChange={(e) => onChange({ ...effect, from: e.target.value })}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        >
          {availableFromStates.length === 0 && (
            <option value="">— no states —</option>
          )}
          {availableFromStates.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </div>

      {/* Arrow */}
      <span className="self-end pb-1.5 text-gray-400">→</span>

      {/* To */}
      <div className="flex flex-col gap-0.5">
        <span className="text-xs text-gray-500">To state</span>
        <select
          value={effect.to}
          onChange={(e) => onChange({ ...effect, to: e.target.value })}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        >
          {availableToStates.length === 0 && (
            <option value="">— (same state) —</option>
          )}
          {availableToStates.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
      </div>

      {/* Outcome association (optional) */}
      {outcomeLabels.length > 0 && (
        <div className="flex flex-col gap-0.5">
          <span className="text-xs text-gray-500">Outcome (optional)</span>
          <select
            value={effect.outcome ?? ""}
            onChange={(e) =>
              onChange({
                ...effect,
                outcome: e.target.value || undefined,
              })
            }
            className="rounded border border-gray-300 px-2 py-1 text-sm"
          >
            <option value="">— any outcome —</option>
            {outcomeLabels.map((o) => (
              <option key={o} value={o}>
                {o}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Validation indicator */}
      {isValid === false && (
        <span className="rounded bg-red-100 px-1 py-0.5 text-xs text-red-600">
          invalid transition
        </span>
      )}

      {/* Delete */}
      <button
        onClick={onDelete}
        className="ml-auto self-end rounded border border-gray-300 px-2 py-1 text-xs text-red-400 hover:bg-red-50"
        title="Remove effect"
      >
        ×
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Operation detail editor
// ---------------------------------------------------------------------------

interface OperationDetailProps {
  op: OperationConstruct;
  allOpIds: string[];
  personas: PersonaConstruct[];
  entities: EntityConstruct[];
  availableFacts: ReturnType<typeof selectFacts>;
  availableVerdicts: string[];
  validation: OpValidation;
  onUpdate: (id: string, updates: Partial<OperationConstruct>) => void;
  onDelete: (id: string) => void;
}

function OperationDetail({
  op,
  allOpIds,
  personas,
  entities,
  availableFacts,
  availableVerdicts,
  validation,
  onUpdate,
  onDelete,
}: OperationDetailProps) {
  const [idDraft, setIdDraft] = useState(op.id);
  const [idError, setIdError] = useState<string | null>(null);

  const outcomeLabels = op.outcomes ?? [];

  function handleIdBlur() {
    const trimmed = idDraft.trim();
    if (!trimmed) {
      setIdError("ID cannot be empty.");
      setIdDraft(op.id);
      return;
    }
    if (trimmed !== op.id && allOpIds.includes(trimmed)) {
      setIdError(`ID "${trimmed}" already exists.`);
      setIdDraft(op.id);
      return;
    }
    setIdError(null);
    if (trimmed !== op.id) {
      onUpdate(op.id, { id: trimmed });
    }
  }

  function togglePersona(personaId: string) {
    const current = op.allowed_personas;
    const next = current.includes(personaId)
      ? current.filter((p) => p !== personaId)
      : [...current, personaId];
    onUpdate(op.id, { allowed_personas: next });
  }

  function handlePreconditionChange(precondition: PredicateExpression) {
    onUpdate(op.id, { precondition });
  }

  function addEffect() {
    const entity = entities[0];
    if (!entity) return;
    const newEffect: Effect = {
      entity_id: entity.id,
      from: entity.initial,
      to: entity.states.find((s) => s !== entity.initial) ?? entity.initial,
    };
    onUpdate(op.id, { effects: [...op.effects, newEffect] });
  }

  function updateEffect(idx: number, updated: Effect) {
    const next = [...op.effects];
    next[idx] = updated;
    onUpdate(op.id, { effects: next });
  }

  function removeEffect(idx: number) {
    onUpdate(op.id, { effects: op.effects.filter((_, i) => i !== idx) });
  }

  function toggleErrorType(errType: string) {
    const current = op.error_contract;
    const next = current.includes(errType)
      ? current.filter((e) => e !== errType)
      : [...current, errType];
    onUpdate(op.id, { error_contract: next });
  }

  function addOutcome() {
    const label = `outcome_${(outcomeLabels.length + 1)}`;
    onUpdate(op.id, { outcomes: [...outcomeLabels, label] });
  }

  function updateOutcome(idx: number, label: string) {
    const next = [...outcomeLabels];
    next[idx] = label;
    onUpdate(op.id, { outcomes: next });
  }

  function removeOutcome(idx: number) {
    onUpdate(op.id, { outcomes: outcomeLabels.filter((_, i) => i !== idx) });
  }

  return (
    <div className="space-y-4 p-4">
      {/* Validation */}
      {validation.errors.length > 0 && (
        <div className="rounded border border-red-300 bg-red-50 p-2">
          {validation.errors.map((e, i) => (
            <p key={i} className="text-xs text-red-700">
              {e}
            </p>
          ))}
        </div>
      )}
      {validation.warnings.length > 0 && (
        <div className="rounded border border-amber-300 bg-amber-50 p-2">
          {validation.warnings.map((w, i) => (
            <p key={i} className="text-xs text-amber-700">
              {w}
            </p>
          ))}
        </div>
      )}

      {/* ID */}
      <div>
        <label className="block text-xs font-medium text-gray-600">
          Operation ID
        </label>
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

      {/* Allowed personas */}
      <div>
        <label className="block text-xs font-medium text-gray-600">
          Allowed Personas
        </label>
        {personas.length === 0 ? (
          <p className="mt-1 text-xs text-gray-400">
            No personas defined — add personas first.
          </p>
        ) : (
          <div className="mt-1 flex flex-wrap gap-2">
            {personas.map((persona) => {
              const checked = op.allowed_personas.includes(persona.id);
              return (
                <label
                  key={persona.id}
                  className={`flex cursor-pointer items-center gap-1.5 rounded border px-2 py-1 text-xs transition-colors ${
                    checked
                      ? "border-blue-300 bg-blue-50 text-blue-700"
                      : "border-gray-200 bg-gray-50 text-gray-600 hover:bg-gray-100"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={checked}
                    onChange={() => togglePersona(persona.id)}
                    className="h-3 w-3"
                  />
                  {persona.id}
                </label>
              );
            })}
          </div>
        )}
      </div>

      {/* Precondition */}
      <div>
        <div className="mb-1 flex items-center justify-between">
          <label className="text-xs font-medium text-gray-600">
            Precondition (when)
          </label>
          <ExprTypeDropdown
            facts={availableFacts}
            verdicts={availableVerdicts}
            onAdd={handlePreconditionChange}
          />
        </div>
        <PredicateBuilder
          value={op.precondition}
          onChange={handlePreconditionChange}
          availableFacts={availableFacts}
          availableVerdicts={availableVerdicts}
          mode="operation"
        />
      </div>

      {/* Effects */}
      <div>
        <div className="mb-1 flex items-center justify-between">
          <label className="text-xs font-medium text-gray-600">
            Effects ({op.effects.length})
          </label>
          <button
            onClick={addEffect}
            disabled={entities.length === 0}
            className="rounded border border-dashed border-gray-300 px-2 py-0.5 text-xs text-gray-500 hover:bg-gray-50 disabled:opacity-40"
            title={entities.length === 0 ? "Define entities first" : "Add effect"}
          >
            + Add effect
          </button>
        </div>
        {entities.length === 0 && (
          <p className="text-xs text-gray-400">
            No entities defined — add entities to specify state transitions.
          </p>
        )}
        <div className="space-y-1.5">
          {op.effects.map((effect, idx) => (
            <EffectRow
              key={idx}
              effect={effect}
              entities={entities}
              outcomeLabels={outcomeLabels}
              onChange={(updated) => updateEffect(idx, updated)}
              onDelete={() => removeEffect(idx)}
            />
          ))}
        </div>
      </div>

      {/* Outcomes (optional) */}
      <div>
        <div className="mb-1 flex items-center justify-between">
          <label className="text-xs font-medium text-gray-600">
            Outcomes (optional)
          </label>
          <button
            onClick={addOutcome}
            className="rounded border border-dashed border-gray-300 px-2 py-0.5 text-xs text-gray-500 hover:bg-gray-50"
          >
            + Add outcome
          </button>
        </div>
        {outcomeLabels.length > 0 && (
          <div className="space-y-1">
            {outcomeLabels.map((label, idx) => (
              <div key={idx} className="flex gap-1">
                <input
                  type="text"
                  value={label}
                  onChange={(e) => updateOutcome(idx, e.target.value)}
                  className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
                  placeholder={`outcome_${idx + 1}`}
                />
                <button
                  onClick={() => removeOutcome(idx)}
                  className="rounded border border-gray-300 px-2 py-1 text-xs text-red-400 hover:bg-red-50"
                  title="Remove outcome"
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Error contract */}
      <div>
        <label className="block text-xs font-medium text-gray-600">
          Error Contract
        </label>
        <div className="mt-1 flex flex-wrap gap-2">
          {STANDARD_ERROR_TYPES.map((errType) => {
            const checked = op.error_contract.includes(errType);
            return (
              <label
                key={errType}
                className={`flex cursor-pointer items-center gap-1.5 rounded border px-2 py-1 text-xs transition-colors ${
                  checked
                    ? "border-red-300 bg-red-50 text-red-700"
                    : "border-gray-200 bg-gray-50 text-gray-600 hover:bg-gray-100"
                }`}
              >
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={() => toggleErrorType(errType)}
                  className="h-3 w-3"
                />
                {errType}
              </label>
            );
          })}
        </div>
        {op.error_contract.length === 0 && (
          <p className="mt-0.5 text-xs text-red-500">
            Select at least one error type.
          </p>
        )}
      </div>

      {/* Delete */}
      <div className="flex justify-end pt-2">
        <button
          onClick={() => {
            if (
              confirm(
                `Delete operation "${op.id}"? This cannot be undone without Undo.`
              )
            ) {
              onDelete(op.id);
            }
          }}
          className="rounded border border-red-200 bg-red-50 px-3 py-1 text-xs text-red-600 hover:bg-red-100"
        >
          Delete operation
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// OperationEditor (main export)
// ---------------------------------------------------------------------------

export function OperationEditor() {
  const operations = useContractStore(selectOperations);
  const personas = useContractStore(selectPersonas);
  const facts = useContractStore(selectFacts);
  const rules = useContractStore(selectRules);
  const entities = useContractStore(selectEntities);

  const addConstruct = useContractStore((s) => s.addConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selectedOp = operations.find((o) => o.id === selectedId) ?? null;
  const allOpIds = operations.map((o) => o.id);

  // All verdicts from all rules are available in operation mode
  const availableVerdicts = useMemo(
    () => rules.map((r) => r.body.produce.verdict_type).filter(Boolean),
    [rules]
  );

  // Validation map
  const validationMap = useMemo(() => {
    const map = new Map<string, OpValidation>();
    for (const op of operations) {
      map.set(op.id, validateOperation(op, entities, personas));
    }
    return map;
  }, [operations, entities, personas]);

  function handleAdd() {
    const baseId = "new_operation";
    let id = baseId;
    let i = 1;
    while (allOpIds.includes(id)) id = `${baseId}_${i++}`;
    addConstruct(newOperation(id));
    setSelectedId(id);
  }

  function handleUpdate(id: string, updates: Partial<OperationConstruct>) {
    if (updates.id && updates.id !== id) {
      const existing = operations.find((o) => o.id === id);
      if (existing) {
        removeConstruct(id, "Operation");
        addConstruct({ ...existing, ...updates } as OperationConstruct);
        setSelectedId(updates.id);
      }
    } else {
      useContractStore.getState().updateConstruct(id, "Operation", updates);
    }
  }

  function handleDelete(id: string) {
    removeConstruct(id, "Operation");
    if (selectedId === id) setSelectedId(null);
  }

  return (
    <div className="flex h-full flex-col">
      {/* Authority matrix at top */}
      <div className="shrink-0 border-b border-gray-200 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-600">
            Authority Matrix
          </h3>
          <button
            onClick={handleAdd}
            className="rounded bg-blue-500 px-3 py-1 text-xs text-white hover:bg-blue-600"
          >
            + Add Operation
          </button>
        </div>
        <AuthorityMatrix personas={personas} operations={operations} />
      </div>

      {/* Body: list + detail */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <aside className="flex w-48 shrink-0 flex-col overflow-y-auto border-r border-gray-200 bg-gray-50">
          {operations.length === 0 ? (
            <div className="p-3 text-center text-xs text-gray-400">
              No operations yet
            </div>
          ) : (
            operations.map((op) => {
              const v = validationMap.get(op.id);
              const hasError = (v?.errors.length ?? 0) > 0;
              return (
                <button
                  key={op.id}
                  onClick={() => setSelectedId(op.id)}
                  className={`w-full px-3 py-2 text-left text-xs transition-colors ${
                    selectedId === op.id
                      ? "bg-blue-100 font-medium text-blue-700"
                      : "text-gray-600 hover:bg-gray-100"
                  } ${hasError ? "text-red-600" : ""}`}
                >
                  <span className="font-mono">{op.id}</span>
                  {hasError && " ⚠"}
                  <div className="truncate text-gray-400">
                    {op.allowed_personas.length > 0
                      ? op.allowed_personas.join(", ")
                      : "no personas"}
                  </div>
                  <div className="truncate text-gray-400">
                    {op.effects.length} effect
                    {op.effects.length !== 1 ? "s" : ""}
                  </div>
                </button>
              );
            })
          )}
        </aside>

        {/* Detail panel */}
        <main className="flex-1 overflow-y-auto bg-white">
          {selectedOp ? (
            <OperationDetail
              op={selectedOp}
              allOpIds={allOpIds}
              personas={personas}
              entities={entities}
              availableFacts={facts}
              availableVerdicts={availableVerdicts}
              validation={validationMap.get(selectedOp.id) ?? { errors: [], warnings: [] }}
              onUpdate={handleUpdate}
              onDelete={handleDelete}
            />
          ) : (
            <div className="flex h-full items-center justify-center text-sm text-gray-400">
              {operations.length === 0
                ? 'Click "+ Add Operation" to create your first operation.'
                : "Select an operation to edit"}
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
