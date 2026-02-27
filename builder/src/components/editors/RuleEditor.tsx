/**
 * RuleEditor: CRUD editor for Rule constructs.
 *
 * Layout:
 * - Top: StratumView showing all rules by stratum
 * - Left panel: rule list organized by stratum (collapsible)
 * - Right panel: detail editor for the selected rule
 *
 * Each rule has:
 * - ID, stratum, condition (PredicateBuilder), produce (verdict_id + type + value)
 * - Validation: same-stratum verdict_present, unique verdict IDs
 */
import React, { useState, useMemo } from "react";
import {
  useContractStore,
  selectRules,
  selectFacts,
} from "@/store/contract";
import type {
  RuleConstruct,
  PredicateExpression,
  BaseType,
  ProduceClause,
} from "@/types/interchange";
import { PredicateBuilder, ExprTypeDropdown } from "@/components/shared/PredicateBuilder";
import { TypePicker } from "@/components/shared/TypePicker";
import { StratumView } from "@/components/visualizations/StratumView";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newRule(id: string, stratum: number): RuleConstruct {
  return {
    id,
    kind: "Rule",
    provenance: { file: "builder", line: 0 },
    stratum,
    tenor: TENOR_VERSION,
    body: {
      when: {
        left: { fact_ref: "fact" },
        op: "=",
        right: { literal: true, type: { base: "Bool" } },
      } as PredicateExpression,
      produce: {
        verdict_type: `${id}_verdict`,
        payload: {
          type: { base: "Bool" },
          value: true,
        },
      },
    },
  };
}

function defaultProduceValue(type: BaseType): boolean | number | string {
  switch (type.base) {
    case "Bool":
      return true;
    case "Int":
      return 0;
    case "Decimal":
      return "0.00";
    case "Money":
      return "0.00";
    case "Text":
      return "";
    case "Date":
      return "";
    case "DateTime":
      return "";
    case "Enum":
      return type.values[0] ?? "";
    default:
      return "";
  }
}

/**
 * Collect verdict IDs referenced via verdict_present in a predicate expression.
 */
function collectVerdictRefs(expr: PredicateExpression | null | undefined): Set<string> {
  const refs = new Set<string>();
  if (!expr) return refs;

  function walk(e: unknown) {
    if (!e || typeof e !== "object") return;
    const obj = e as Record<string, unknown>;
    if ("verdict_present" in obj && typeof obj.verdict_present === "string") {
      refs.add(obj.verdict_present);
      return;
    }
    if ("left" in obj) walk(obj.left);
    if ("right" in obj) walk(obj.right);
    if ("operand" in obj) walk(obj.operand);
    if ("body" in obj) walk(obj.body);
  }

  walk(expr);
  return refs;
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

interface RuleValidation {
  errors: string[];
  warnings: string[];
}

function validateRules(rules: RuleConstruct[]): Map<string, RuleValidation> {
  const result = new Map<string, RuleValidation>();

  // Build verdict_id -> (rule, stratum) map
  const verdictMap = new Map<string, { ruleId: string; stratum: number }>();
  for (const rule of rules) {
    const vid = rule.body.produce.verdict_type;
    if (vid) {
      if (verdictMap.has(vid)) {
        const prev = verdictMap.get(vid)!;
        const errs = result.get(rule.id) ?? { errors: [], warnings: [] };
        errs.errors.push(`Duplicate verdict ID "${vid}" (also in ${prev.ruleId})`);
        result.set(rule.id, errs);
      } else {
        verdictMap.set(vid, { ruleId: rule.id, stratum: rule.stratum });
      }
    }
  }

  // Check same-stratum verdict_present references
  for (const rule of rules) {
    const refs = collectVerdictRefs(rule.body.when);
    for (const vid of refs) {
      const producer = verdictMap.get(vid);
      if (producer && producer.stratum === rule.stratum) {
        const errs = result.get(rule.id) ?? { errors: [], warnings: [] };
        errs.errors.push(
          `verdict_present("${vid}") references same stratum ${rule.stratum} — must use lower stratum`
        );
        result.set(rule.id, errs);
      }
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Produce value editor
// ---------------------------------------------------------------------------

interface ProduceValueEditorProps {
  type: BaseType;
  value: boolean | number | string | object;
  onChange: (v: boolean | number | string) => void;
}

function ProduceValueEditor({ type, value, onChange }: ProduceValueEditorProps) {
  switch (type.base) {
    case "Bool":
      return (
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={Boolean(value)}
            onChange={(e) => onChange(e.target.checked)}
          />
          <span className="text-xs text-gray-600">
            {Boolean(value) ? "true" : "false"}
          </span>
        </label>
      );
    case "Int":
      return (
        <input
          type="number"
          value={typeof value === "number" ? value : 0}
          onChange={(e) => onChange(Number(e.target.value))}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    case "Decimal":
    case "Money":
      return (
        <input
          type="text"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          placeholder="0.00"
        />
      );
    case "Text":
      return (
        <input
          type="text"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    case "Date":
      return (
        <input
          type="date"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    case "DateTime":
      return (
        <input
          type="datetime-local"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    case "Enum":
      return (
        <select
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        >
          {type.values.map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      );
    default:
      return (
        <p className="text-xs text-gray-400">
          No value editor for {type.base}
        </p>
      );
  }
}

// ---------------------------------------------------------------------------
// Rule detail editor
// ---------------------------------------------------------------------------

interface RuleDetailProps {
  rule: RuleConstruct;
  allRuleIds: string[];
  allRules: RuleConstruct[];
  availableFacts: ReturnType<typeof selectFacts>;
  validation: RuleValidation | undefined;
  onUpdate: (id: string, updates: Partial<RuleConstruct>) => void;
  onDelete: (id: string) => void;
}

function RuleDetail({
  rule,
  allRuleIds,
  allRules,
  availableFacts,
  validation,
  onUpdate,
  onDelete,
}: RuleDetailProps) {
  const [idDraft, setIdDraft] = useState(rule.id);
  const [idError, setIdError] = useState<string | null>(null);

  // Verdicts available for conditions in this rule: rules with stratum < this rule's stratum
  const availableVerdicts = useMemo(
    () =>
      allRules
        .filter((r) => r.stratum < rule.stratum)
        .map((r) => r.body.produce.verdict_type)
        .filter(Boolean),
    [allRules, rule.stratum]
  );

  function handleIdBlur() {
    const trimmed = idDraft.trim();
    if (!trimmed) {
      setIdError("ID cannot be empty.");
      setIdDraft(rule.id);
      return;
    }
    if (trimmed !== rule.id && allRuleIds.includes(trimmed)) {
      setIdError(`ID "${trimmed}" already exists.`);
      setIdDraft(rule.id);
      return;
    }
    setIdError(null);
    if (trimmed !== rule.id) {
      onUpdate(rule.id, { id: trimmed });
    }
  }

  function handleStratumChange(stratum: number) {
    if (stratum < 0 || !Number.isInteger(stratum)) return;
    onUpdate(rule.id, { stratum });
  }

  function handleWhenChange(when: PredicateExpression) {
    onUpdate(rule.id, { body: { ...rule.body, when } });
  }

  function handleProduceVerdictId(verdict_type: string) {
    onUpdate(rule.id, {
      body: { ...rule.body, produce: { ...rule.body.produce, verdict_type } },
    });
  }

  function handleProduceTypeChange(type: BaseType) {
    const produce: ProduceClause = {
      ...rule.body.produce,
      payload: {
        type,
        value: defaultProduceValue(type),
      },
    };
    onUpdate(rule.id, { body: { ...rule.body, produce } });
  }

  function handleProduceValueChange(value: boolean | number | string) {
    const produce: ProduceClause = {
      ...rule.body.produce,
      payload: { ...rule.body.produce.payload, value },
    };
    onUpdate(rule.id, { body: { ...rule.body, produce } });
  }

  const errors = validation?.errors ?? [];

  return (
    <div className="space-y-4 p-4">
      {/* Validation errors */}
      {errors.length > 0 && (
        <div className="rounded border border-red-300 bg-red-50 p-2">
          {errors.map((e, i) => (
            <p key={i} className="text-xs text-red-700">
              {e}
            </p>
          ))}
        </div>
      )}

      {/* ID + Stratum */}
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="block text-xs font-medium text-gray-600">
            Rule ID
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
          {idError && (
            <p className="mt-0.5 text-xs text-red-500">{idError}</p>
          )}
        </div>
        <div>
          <label className="block text-xs font-medium text-gray-600">
            Stratum (non-negative integer)
          </label>
          <input
            type="number"
            min={0}
            step={1}
            value={rule.stratum}
            onChange={(e) => handleStratumChange(Math.max(0, Math.floor(Number(e.target.value))))}
            className="mt-0.5 w-full rounded border border-gray-300 px-2 py-1 text-sm"
          />
          {rule.stratum > 0 && availableVerdicts.length > 0 && (
            <p className="mt-0.5 text-xs text-gray-500">
              Available verdicts from lower strata: {availableVerdicts.join(", ")}
            </p>
          )}
        </div>
      </div>

      {/* Condition (when) */}
      <div>
        <div className="mb-1 flex items-center justify-between">
          <label className="text-xs font-medium text-gray-600">
            Condition (when)
          </label>
          <ExprTypeDropdown
            facts={availableFacts}
            verdicts={availableVerdicts}
            onAdd={handleWhenChange}
          />
        </div>
        <PredicateBuilder
          value={rule.body.when}
          onChange={handleWhenChange}
          availableFacts={availableFacts}
          availableVerdicts={availableVerdicts}
          mode="rule"
        />
      </div>

      {/* Produce */}
      <div className="rounded border border-gray-200 bg-gray-50 p-3">
        <h4 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-600">
          Produce
        </h4>
        <div className="space-y-2">
          <div>
            <label className="block text-xs font-medium text-gray-600">
              Verdict ID
            </label>
            <input
              type="text"
              value={rule.body.produce.verdict_type}
              onChange={(e) => handleProduceVerdictId(e.target.value)}
              className="mt-0.5 w-full rounded border border-gray-300 px-2 py-1 font-mono text-sm"
              placeholder="verdict_id"
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-gray-600">
              Payload type
            </label>
            <div className="mt-0.5">
              <TypePicker
                value={rule.body.produce.payload.type}
                onChange={handleProduceTypeChange}
              />
            </div>
          </div>
          <div>
            <label className="block text-xs font-medium text-gray-600">
              Payload value
            </label>
            <div className="mt-0.5">
              <ProduceValueEditor
                type={rule.body.produce.payload.type}
                value={rule.body.produce.payload.value}
                onChange={handleProduceValueChange}
              />
            </div>
          </div>
        </div>
      </div>

      {/* Delete */}
      <div className="flex justify-end pt-2">
        <button
          onClick={() => {
            if (
              confirm(
                `Delete rule "${rule.id}"? This cannot be undone without Undo.`
              )
            ) {
              onDelete(rule.id);
            }
          }}
          className="rounded border border-red-200 bg-red-50 px-3 py-1 text-xs text-red-600 hover:bg-red-100"
        >
          Delete rule
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add rule dialog
// ---------------------------------------------------------------------------

interface AddRuleFormProps {
  existingIds: string[];
  existingStrata: number[];
  onAdd: (id: string, stratum: number) => void;
  onCancel: () => void;
}

function AddRuleForm({
  existingIds,
  existingStrata,
  onAdd,
  onCancel,
}: AddRuleFormProps) {
  const defaultStratum =
    existingStrata.length > 0 ? Math.max(...existingStrata) : 0;
  const [id, setId] = useState(() => {
    let base = "new_rule";
    let i = 1;
    while (existingIds.includes(base)) base = `new_rule_${i++}`;
    return base;
  });
  const [stratum, setStratum] = useState(defaultStratum);
  const [idError, setIdError] = useState<string | null>(null);

  function handleSubmit() {
    const trimmed = id.trim();
    if (!trimmed) {
      setIdError("ID is required.");
      return;
    }
    if (existingIds.includes(trimmed)) {
      setIdError(`ID "${trimmed}" already exists.`);
      return;
    }
    onAdd(trimmed, Math.max(0, stratum));
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-20">
      <div className="w-72 rounded-lg border border-gray-200 bg-white p-4 shadow-xl">
        <h3 className="mb-3 text-sm font-semibold text-gray-700">
          Add Rule
        </h3>
        <div className="space-y-2">
          <div>
            <label className="block text-xs font-medium text-gray-600">
              Rule ID
            </label>
            <input
              type="text"
              value={id}
              onChange={(e) => {
                setId(e.target.value);
                setIdError(null);
              }}
              autoFocus
              className={`mt-0.5 w-full rounded border px-2 py-1 font-mono text-sm ${
                idError ? "border-red-400 bg-red-50" : "border-gray-300"
              }`}
            />
            {idError && (
              <p className="mt-0.5 text-xs text-red-500">{idError}</p>
            )}
          </div>
          <div>
            <label className="block text-xs font-medium text-gray-600">
              Stratum
            </label>
            <input
              type="number"
              min={0}
              step={1}
              value={stratum}
              onChange={(e) =>
                setStratum(Math.max(0, Math.floor(Number(e.target.value))))
              }
              className="mt-0.5 w-full rounded border border-gray-300 px-2 py-1 text-sm"
            />
          </div>
        </div>
        <div className="mt-3 flex justify-end gap-2">
          <button
            onClick={onCancel}
            className="rounded border border-gray-300 px-3 py-1 text-xs text-gray-600 hover:bg-gray-50"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            className="rounded bg-blue-500 px-3 py-1 text-xs text-white hover:bg-blue-600"
          >
            Add
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// RuleEditor (main export)
// ---------------------------------------------------------------------------

export function RuleEditor() {
  const rules = useContractStore(selectRules);
  const facts = useContractStore(selectFacts);
  const addConstruct = useContractStore((s) => s.addConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);

  const selectedRule = rules.find((r) => r.id === selectedId) ?? null;
  const allRuleIds = rules.map((r) => r.id);
  const existingStrata = [...new Set(rules.map((r) => r.stratum))];

  const validationMap = useMemo(() => validateRules(rules), [rules]);

  function handleAdd(id: string, stratum: number) {
    addConstruct(newRule(id, stratum));
    setSelectedId(id);
    setShowAddForm(false);
  }

  function handleUpdate(id: string, updates: Partial<RuleConstruct>) {
    // If ID changed, remove old and add new
    if (updates.id && updates.id !== id) {
      const existing = rules.find((r) => r.id === id);
      if (existing) {
        removeConstruct(id, "Rule");
        addConstruct({ ...existing, ...updates } as RuleConstruct);
        setSelectedId(updates.id);
      }
    } else {
      useContractStore
        .getState()
        .updateConstruct(id, "Rule", updates);
    }
  }

  function handleDelete(id: string) {
    removeConstruct(id, "Rule");
    if (selectedId === id) setSelectedId(null);
  }

  // Group rules by stratum for the sidebar list
  const rulesByStratum = useMemo(() => {
    const map = new Map<number, RuleConstruct[]>();
    for (const rule of rules) {
      const s = rule.stratum ?? 0;
      if (!map.has(s)) map.set(s, []);
      map.get(s)!.push(rule);
    }
    return map;
  }, [rules]);

  const sortedStrata = useMemo(
    () => Array.from(rulesByStratum.keys()).sort((a, b) => a - b),
    [rulesByStratum]
  );

  return (
    <div className="flex h-full flex-col">
      {/* Stratum view at the top */}
      <div className="shrink-0 border-b border-gray-200 bg-white p-3">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-600">
            Rule Strata
          </h3>
          <button
            onClick={() => setShowAddForm(true)}
            className="rounded bg-blue-500 px-3 py-1 text-xs text-white hover:bg-blue-600"
          >
            + Add Rule
          </button>
        </div>
        <StratumView
          rules={rules}
          selectedRuleId={selectedId}
          compact={rules.length > 8}
          onSelectRule={setSelectedId}
        />
      </div>

      {/* Body: list + detail */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar list */}
        <aside className="flex w-48 shrink-0 flex-col overflow-y-auto border-r border-gray-200 bg-gray-50">
          {rules.length === 0 ? (
            <div className="p-3 text-center text-xs text-gray-400">
              No rules yet
            </div>
          ) : (
            sortedStrata.map((stratum) => {
              const stratumRules = rulesByStratum.get(stratum) ?? [];
              return (
                <div key={stratum}>
                  <div className="sticky top-0 bg-gray-100 px-3 py-1 text-xs font-semibold text-gray-600">
                    Stratum {stratum}
                  </div>
                  {stratumRules.map((rule) => {
                    const hasError =
                      (validationMap.get(rule.id)?.errors.length ?? 0) > 0;
                    return (
                      <button
                        key={rule.id}
                        onClick={() => setSelectedId(rule.id)}
                        className={`w-full px-3 py-2 text-left text-xs transition-colors ${
                          selectedId === rule.id
                            ? "bg-blue-100 font-medium text-blue-700"
                            : "text-gray-600 hover:bg-gray-100"
                        } ${hasError ? "text-red-600" : ""}`}
                      >
                        <span className="font-mono">{rule.id}</span>
                        {hasError && " ⚠"}
                        <div className="truncate text-gray-400">
                          → {rule.body.produce.verdict_type || "verdict"}
                        </div>
                      </button>
                    );
                  })}
                </div>
              );
            })
          )}
        </aside>

        {/* Detail panel */}
        <main className="flex-1 overflow-y-auto bg-white">
          {selectedRule ? (
            <RuleDetail
              rule={selectedRule}
              allRuleIds={allRuleIds}
              allRules={rules}
              availableFacts={facts}
              validation={validationMap.get(selectedRule.id)}
              onUpdate={handleUpdate}
              onDelete={handleDelete}
            />
          ) : (
            <div className="flex h-full items-center justify-center text-sm text-gray-400">
              {rules.length === 0
                ? 'Click "+ Add Rule" to create your first rule.'
                : "Select a rule to edit"}
            </div>
          )}
        </main>
      </div>

      {/* Add rule modal */}
      {showAddForm && (
        <AddRuleForm
          existingIds={allRuleIds}
          existingStrata={existingStrata}
          onAdd={handleAdd}
          onCancel={() => setShowAddForm(false)}
        />
      )}
    </div>
  );
}
