/**
 * SystemEditor: CRUD editor for System constructs.
 *
 * Systems represent compositions of multiple contracts. Each system has:
 * - A list of member contracts (id + path)
 * - Shared personas across contracts
 * - Shared entities across contracts
 * - Cross-contract flow triggers
 *
 * This editor is intentionally lighter than other editors — systems are
 * a high-level composition concern, not the primary design target.
 */
import React, { useState } from "react";
import {
  useContractStore,
  useSystems,
  usePersonas,
} from "@/store/contract";
import type {
  SystemConstruct,
  SystemMember,
  SharedPersona,
  SharedEntity,
  SystemTrigger,
} from "@/types/interchange";
import { Button } from "@/components/ui/button";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newSystem(id: string): SystemConstruct {
  return {
    kind: "System",
    id,
    tenor: TENOR_VERSION,
    provenance: { file: "builder", line: 0 },
    members: [],
    shared_personas: [],
    shared_entities: [],
    triggers: [],
  };
}

function newMember(): SystemMember {
  return { id: "", path: "" };
}

function newSharedPersona(): SharedPersona {
  return { persona: "", contracts: [] };
}

function newSharedEntity(): SharedEntity {
  return { entity: "", contracts: [] };
}

function newTrigger(): SystemTrigger {
  return {
    on: "success",
    persona: "",
    source_contract: "",
    source_flow: "",
    target_contract: "",
    target_flow: "",
  };
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

interface SystemValidation {
  errors: string[];
  warnings: string[];
}

function validateSystem(system: SystemConstruct): SystemValidation {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!system.id.trim()) {
    errors.push("System ID cannot be empty.");
  }

  if (system.members.length === 0) {
    warnings.push("No member contracts — add at least one contract.");
  }

  const memberIds = new Set(system.members.map((m) => m.id));

  for (const sp of system.shared_personas) {
    if (!sp.persona.trim()) {
      errors.push("Shared persona ID cannot be empty.");
    }
    for (const contractId of sp.contracts) {
      if (contractId && !memberIds.has(contractId)) {
        warnings.push(`Shared persona "${sp.persona}": contract "${contractId}" not in members.`);
      }
    }
  }

  for (const se of system.shared_entities) {
    if (!se.entity.trim()) {
      errors.push("Shared entity ID cannot be empty.");
    }
  }

  for (const trigger of system.triggers) {
    if (trigger.source_contract && !memberIds.has(trigger.source_contract)) {
      warnings.push(`Trigger: source contract "${trigger.source_contract}" not in members.`);
    }
    if (trigger.target_contract && !memberIds.has(trigger.target_contract)) {
      warnings.push(`Trigger: target contract "${trigger.target_contract}" not in members.`);
    }
  }

  return { errors, warnings };
}

// ---------------------------------------------------------------------------
// MemberList editor
// ---------------------------------------------------------------------------

interface MemberListEditorProps {
  members: SystemMember[];
  onChange: (members: SystemMember[]) => void;
}

function MemberListEditor({ members, onChange }: MemberListEditorProps) {
  function addMember() {
    onChange([...members, newMember()]);
  }

  function updateMember(idx: number, updated: SystemMember) {
    const next = [...members];
    next[idx] = updated;
    onChange(next);
  }

  function removeMember(idx: number) {
    onChange(members.filter((_, i) => i !== idx));
  }

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">
          Member Contracts ({members.length})
        </span>
        <Button
          variant="outline"
          size="xs"
          onClick={addMember}
          className="border-dashed"
        >
          + Add Contract
        </Button>
      </div>
      {members.length === 0 && (
        <p className="text-xs text-muted-foreground">
          No member contracts — systems compose multiple contracts.
        </p>
      )}
      <div className="space-y-1.5">
        {members.map((member, idx) => (
          <div
            key={idx}
            className="flex items-center gap-1.5 rounded border border-border bg-muted p-2"
          >
            <div className="flex flex-col gap-0.5">
              <span className="text-xs text-muted-foreground">ID</span>
              <input
                type="text"
                value={member.id}
                onChange={(e) => updateMember(idx, { ...member, id: e.target.value })}
                className="w-28 rounded-md border border-border px-1.5 py-0.5 font-mono text-xs"
                placeholder="contract_id"
              />
            </div>
            <div className="flex flex-col gap-0.5">
              <span className="text-xs text-muted-foreground">Path</span>
              <input
                type="text"
                value={member.path}
                onChange={(e) => updateMember(idx, { ...member, path: e.target.value })}
                className="w-40 rounded-md border border-border px-1.5 py-0.5 font-mono text-xs"
                placeholder="./contracts/foo.tenor"
              />
            </div>
            <Button
              variant="ghost"
              size="xs"
              onClick={() => removeMember(idx)}
              className="ml-auto self-end text-destructive hover:bg-destructive/10"
              title="Remove"
            >
              ×
            </Button>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SharedPersonas editor
// ---------------------------------------------------------------------------

interface SharedPersonasEditorProps {
  sharedPersonas: SharedPersona[];
  memberIds: string[];
  knownPersonaIds: string[];
  onChange: (personas: SharedPersona[]) => void;
}

function SharedPersonasEditor({
  sharedPersonas,
  memberIds,
  knownPersonaIds,
  onChange,
}: SharedPersonasEditorProps) {
  function add() {
    onChange([...sharedPersonas, newSharedPersona()]);
  }

  function update(idx: number, updated: SharedPersona) {
    const next = [...sharedPersonas];
    next[idx] = updated;
    onChange(next);
  }

  function remove(idx: number) {
    onChange(sharedPersonas.filter((_, i) => i !== idx));
  }

  function toggleContract(idx: number, contractId: string) {
    const sp = sharedPersonas[idx];
    const contracts = sp.contracts.includes(contractId)
      ? sp.contracts.filter((c) => c !== contractId)
      : [...sp.contracts, contractId];
    update(idx, { ...sp, contracts });
  }

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">
          Shared Personas ({sharedPersonas.length})
        </span>
        <Button
          variant="outline"
          size="xs"
          onClick={add}
          className="border-dashed"
        >
          + Add
        </Button>
      </div>
      <div className="space-y-2">
        {sharedPersonas.map((sp, idx) => (
          <div
            key={idx}
            className="rounded border border-border bg-muted p-2"
          >
            <div className="mb-1.5 flex items-center gap-1.5">
              <select
                value={sp.persona}
                onChange={(e) => update(idx, { ...sp, persona: e.target.value })}
                className="rounded-md border border-border px-1.5 py-0.5 text-xs"
              >
                {knownPersonaIds.length === 0 && (
                  <option value="">— type persona ID —</option>
                )}
                {knownPersonaIds.map((pid) => (
                  <option key={pid} value={pid}>{pid}</option>
                ))}
              </select>
              {knownPersonaIds.length === 0 && (
                <input
                  type="text"
                  value={sp.persona}
                  onChange={(e) => update(idx, { ...sp, persona: e.target.value })}
                  className="flex-1 rounded-md border border-border px-1.5 py-0.5 font-mono text-xs"
                  placeholder="persona_id"
                />
              )}
              <Button
                variant="ghost"
                size="xs"
                onClick={() => remove(idx)}
                className="ml-auto text-destructive hover:bg-destructive/10"
              >
                ×
              </Button>
            </div>
            {memberIds.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {memberIds.map((cid) => (
                  <label
                    key={cid}
                    className={`flex cursor-pointer items-center gap-1 rounded border px-1.5 py-0.5 text-xs transition-colors ${
                      sp.contracts.includes(cid)
                        ? "border-primary/50 bg-muted text-primary"
                        : "border-border bg-card text-muted-foreground hover:bg-muted"
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={sp.contracts.includes(cid)}
                      onChange={() => toggleContract(idx, cid)}
                      className="h-3 w-3"
                    />
                    {cid}
                  </label>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SharedEntities editor
// ---------------------------------------------------------------------------

interface SharedEntitiesEditorProps {
  sharedEntities: SharedEntity[];
  memberIds: string[];
  onChange: (entities: SharedEntity[]) => void;
}

function SharedEntitiesEditor({ sharedEntities, memberIds, onChange }: SharedEntitiesEditorProps) {
  function add() {
    onChange([...sharedEntities, newSharedEntity()]);
  }

  function update(idx: number, updated: SharedEntity) {
    const next = [...sharedEntities];
    next[idx] = updated;
    onChange(next);
  }

  function remove(idx: number) {
    onChange(sharedEntities.filter((_, i) => i !== idx));
  }

  function toggleContract(idx: number, contractId: string) {
    const se = sharedEntities[idx];
    const contracts = se.contracts.includes(contractId)
      ? se.contracts.filter((c) => c !== contractId)
      : [...se.contracts, contractId];
    update(idx, { ...se, contracts });
  }

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">
          Shared Entities ({sharedEntities.length})
        </span>
        <Button
          variant="outline"
          size="xs"
          onClick={add}
          className="border-dashed"
        >
          + Add
        </Button>
      </div>
      <div className="space-y-2">
        {sharedEntities.map((se, idx) => (
          <div key={idx} className="rounded border border-border bg-muted p-2">
            <div className="mb-1.5 flex items-center gap-1.5">
              <input
                type="text"
                value={se.entity}
                onChange={(e) => update(idx, { ...se, entity: e.target.value })}
                className="flex-1 rounded-md border border-border px-1.5 py-0.5 font-mono text-xs"
                placeholder="entity_id"
              />
              <Button
                variant="ghost"
                size="xs"
                onClick={() => remove(idx)}
                className="text-destructive hover:bg-destructive/10"
              >
                ×
              </Button>
            </div>
            {memberIds.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {memberIds.map((cid) => (
                  <label
                    key={cid}
                    className={`flex cursor-pointer items-center gap-1 rounded border px-1.5 py-0.5 text-xs transition-colors ${
                      se.contracts.includes(cid)
                        ? "border-primary/50 bg-muted text-primary"
                        : "border-border bg-card text-muted-foreground hover:bg-muted"
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={se.contracts.includes(cid)}
                      onChange={() => toggleContract(idx, cid)}
                      className="h-3 w-3"
                    />
                    {cid}
                  </label>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Triggers editor
// ---------------------------------------------------------------------------

interface TriggersEditorProps {
  triggers: SystemTrigger[];
  memberIds: string[];
  onChange: (triggers: SystemTrigger[]) => void;
}

function TriggersEditor({ triggers, memberIds, onChange }: TriggersEditorProps) {
  function add() {
    onChange([...triggers, newTrigger()]);
  }

  function update(idx: number, updated: SystemTrigger) {
    const next = [...triggers];
    next[idx] = updated;
    onChange(next);
  }

  function remove(idx: number) {
    onChange(triggers.filter((_, i) => i !== idx));
  }

  return (
    <div>
      <div className="mb-2 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground">
          Cross-Contract Triggers ({triggers.length})
        </span>
        <Button
          variant="outline"
          size="xs"
          onClick={add}
          className="border-dashed"
        >
          + Add Trigger
        </Button>
      </div>
      <div className="space-y-2">
        {triggers.map((trigger, idx) => (
          <div
            key={idx}
            className="rounded border border-border bg-muted p-2"
          >
            <div className="mb-1 flex items-center justify-between">
              <div className="flex items-center gap-1">
                <span className="text-xs text-muted-foreground">on:</span>
                <select
                  value={trigger.on}
                  onChange={(e) =>
                    update(idx, {
                      ...trigger,
                      on: e.target.value as SystemTrigger["on"],
                    })
                  }
                  className="rounded-md border border-border px-1 py-0.5 text-xs"
                >
                  <option value="success">success</option>
                  <option value="failure">failure</option>
                  <option value="escalation">escalation</option>
                </select>
              </div>
              <Button
                variant="ghost"
                size="xs"
                onClick={() => remove(idx)}
                className="text-destructive hover:bg-destructive/10"
              >
                ×
              </Button>
            </div>
            <div className="grid grid-cols-2 gap-1">
              <div className="flex flex-col gap-0.5">
                <span className="text-xs text-muted-foreground">source contract</span>
                <select
                  value={trigger.source_contract}
                  onChange={(e) => update(idx, { ...trigger, source_contract: e.target.value })}
                  className="rounded-md border border-border px-1 py-0.5 text-xs"
                >
                  <option value="">— select —</option>
                  {memberIds.map((id) => (
                    <option key={id} value={id}>{id}</option>
                  ))}
                </select>
              </div>
              <div className="flex flex-col gap-0.5">
                <span className="text-xs text-muted-foreground">source flow</span>
                <input
                  type="text"
                  value={trigger.source_flow}
                  onChange={(e) => update(idx, { ...trigger, source_flow: e.target.value })}
                  className="rounded-md border border-border px-1 py-0.5 font-mono text-xs"
                  placeholder="flow_id"
                />
              </div>
              <div className="flex flex-col gap-0.5">
                <span className="text-xs text-muted-foreground">target contract</span>
                <select
                  value={trigger.target_contract}
                  onChange={(e) => update(idx, { ...trigger, target_contract: e.target.value })}
                  className="rounded-md border border-border px-1 py-0.5 text-xs"
                >
                  <option value="">— select —</option>
                  {memberIds.map((id) => (
                    <option key={id} value={id}>{id}</option>
                  ))}
                </select>
              </div>
              <div className="flex flex-col gap-0.5">
                <span className="text-xs text-muted-foreground">target flow</span>
                <input
                  type="text"
                  value={trigger.target_flow}
                  onChange={(e) => update(idx, { ...trigger, target_flow: e.target.value })}
                  className="rounded-md border border-border px-1 py-0.5 font-mono text-xs"
                  placeholder="flow_id"
                />
              </div>
            </div>
            <div className="mt-1 flex items-center gap-1">
              <span className="text-xs text-muted-foreground">persona:</span>
              <input
                type="text"
                value={trigger.persona}
                onChange={(e) => update(idx, { ...trigger, persona: e.target.value })}
                className="rounded-md border border-border px-1 py-0.5 font-mono text-xs"
                placeholder="persona_id"
              />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// System detail editor
// ---------------------------------------------------------------------------

interface SystemDetailProps {
  system: SystemConstruct;
  allSystemIds: string[];
  knownPersonaIds: string[];
  validation: SystemValidation;
  onUpdate: (id: string, updates: Partial<SystemConstruct>) => void;
  onDelete: (id: string) => void;
}

function SystemDetail({
  system,
  allSystemIds,
  knownPersonaIds,
  validation,
  onUpdate,
  onDelete,
}: SystemDetailProps) {
  const [idDraft, setIdDraft] = useState(system.id);
  const [idError, setIdError] = useState<string | null>(null);

  const memberIds = system.members.map((m) => m.id).filter(Boolean);

  function handleIdBlur() {
    const trimmed = idDraft.trim();
    if (!trimmed) {
      setIdError("ID cannot be empty.");
      setIdDraft(system.id);
      return;
    }
    if (trimmed !== system.id && allSystemIds.includes(trimmed)) {
      setIdError(`ID "${trimmed}" already exists.`);
      setIdDraft(system.id);
      return;
    }
    setIdError(null);
    if (trimmed !== system.id) {
      onUpdate(system.id, { id: trimmed });
    }
  }

  return (
    <div className="space-y-5 p-4">
      {/* Validation */}
      {validation.errors.length > 0 && (
        <div className="rounded border border-destructive/50 bg-destructive/10 p-2">
          {validation.errors.map((e, i) => (
            <p key={i} className="text-xs text-destructive">
              {e}
            </p>
          ))}
        </div>
      )}
      {validation.warnings.length > 0 && (
        <div className="rounded border border-border bg-muted p-2">
          {validation.warnings.map((w, i) => (
            <p key={i} className="text-xs text-muted-foreground">
              {w}
            </p>
          ))}
        </div>
      )}

      {/* System ID */}
      <div>
        <label className="block text-xs font-medium text-muted-foreground">
          System ID
        </label>
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

      {/* Member contracts */}
      <MemberListEditor
        members={system.members}
        onChange={(members) => onUpdate(system.id, { members })}
      />

      {/* Shared personas */}
      <SharedPersonasEditor
        sharedPersonas={system.shared_personas}
        memberIds={memberIds}
        knownPersonaIds={knownPersonaIds}
        onChange={(shared_personas) => onUpdate(system.id, { shared_personas })}
      />

      {/* Shared entities */}
      <SharedEntitiesEditor
        sharedEntities={system.shared_entities}
        memberIds={memberIds}
        onChange={(shared_entities) => onUpdate(system.id, { shared_entities })}
      />

      {/* Triggers */}
      <TriggersEditor
        triggers={system.triggers}
        memberIds={memberIds}
        onChange={(triggers) => onUpdate(system.id, { triggers })}
      />

      {/* Delete */}
      <div className="flex justify-end border-t border-border pt-3">
        <Button
          variant="destructive"
          size="xs"
          onClick={() => {
            if (confirm(`Delete system "${system.id}"?`)) {
              onDelete(system.id);
            }
          }}
        >
          Delete system
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SystemEditor (main export)
// ---------------------------------------------------------------------------

export function SystemEditor() {
  const systems = useSystems();
  const personas = usePersonas();
  const addConstruct = useContractStore((s) => s.addConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const [selectedId, setSelectedId] = useState<string | null>(null);

  const selectedSystem = systems.find((s) => s.id === selectedId) ?? null;
  const allSystemIds = systems.map((s) => s.id);
  const knownPersonaIds = personas.map((p) => p.id);

  const validationMap = new Map<string, SystemValidation>();
  for (const sys of systems) {
    validationMap.set(sys.id, validateSystem(sys));
  }

  function handleAdd() {
    const base = "new_system";
    let id = base;
    let i = 1;
    while (allSystemIds.includes(id)) id = `${base}_${i++}`;
    addConstruct(newSystem(id));
    setSelectedId(id);
  }

  function handleUpdate(id: string, updates: Partial<SystemConstruct>) {
    if (updates.id && updates.id !== id) {
      const existing = systems.find((s) => s.id === id);
      if (existing) {
        removeConstruct(id, "System");
        addConstruct({ ...existing, ...updates } as SystemConstruct);
        setSelectedId(updates.id);
      }
    } else {
      useContractStore.getState().updateConstruct(id, "System", updates);
    }
  }

  function handleDelete(id: string) {
    removeConstruct(id, "System");
    if (selectedId === id) setSelectedId(null);
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Sidebar */}
      <aside className="flex w-52 shrink-0 flex-col border-r border-border bg-muted">
        <div className="flex items-center justify-between border-b border-border px-3 py-2">
          <span className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            Systems
          </span>
          <Button size="xs" onClick={handleAdd}>
            +
          </Button>
        </div>
        <div className="flex-1 overflow-y-auto">
          {systems.length === 0 ? (
            <div className="p-3 text-center text-xs text-muted-foreground">
              No systems yet
            </div>
          ) : (
            systems.map((sys) => {
              const v = validationMap.get(sys.id);
              const hasError = (v?.errors.length ?? 0) > 0;
              return (
                <button
                  key={sys.id}
                  onClick={() => setSelectedId(sys.id)}
                  className={`w-full px-3 py-2 text-left text-xs transition-colors ${
                    selectedId === sys.id
                      ? "bg-muted font-medium text-primary"
                      : "text-muted-foreground hover:bg-muted"
                  } ${hasError ? "text-destructive" : ""}`}
                >
                  <div className="font-mono">{sys.id}</div>
                  <div className="truncate text-muted-foreground">
                    {sys.members.length} contract
                    {sys.members.length !== 1 ? "s" : ""}
                  </div>
                  {hasError && (
                    <span className="text-xs text-destructive">
                      {v?.errors.length} error{(v?.errors.length ?? 0) !== 1 ? "s" : ""}
                    </span>
                  )}
                </button>
              );
            })
          )}
        </div>
      </aside>

      {/* Detail panel */}
      <main className="flex-1 overflow-y-auto bg-card">
        {selectedSystem ? (
          <SystemDetail
            system={selectedSystem}
            allSystemIds={allSystemIds}
            knownPersonaIds={knownPersonaIds}
            validation={validationMap.get(selectedSystem.id) ?? { errors: [], warnings: [] }}
            onUpdate={handleUpdate}
            onDelete={handleDelete}
          />
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
            {systems.length === 0
              ? 'Click "+" to create your first system.'
              : "Select a system to edit"}
          </div>
        )}
      </main>
    </div>
  );
}
