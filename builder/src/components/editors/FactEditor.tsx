/**
 * FactEditor: full CRUD editor for Fact constructs.
 *
 * Fact table with expandable detail rows. Each fact supports:
 * - ID input (unique validation)
 * - TypePicker for all Tenor BaseType variants
 * - Default value: type-appropriate control producing correct interchange JSON
 * - Source: freetext (system.field) or structured (source_id + path)
 * - Delete
 */
import React, { useState } from "react";
import { useContractStore, selectFacts, selectSources } from "@/store/contract";
import type {
  FactConstruct,
  BaseType,
  FactDefault,
  FactSource,
  FreetextSource,
  StructuredSource,
  DecimalValue,
  MoneyValue,
  BoolLiteral,
} from "@/types/interchange";
import { TypePicker } from "@/components/shared/TypePicker";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TENOR_VERSION = "1.0";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function newFact(id: string): FactConstruct {
  return {
    id,
    kind: "Fact",
    provenance: { file: "builder", line: 0 },
    tenor: TENOR_VERSION,
    type: { base: "Bool" },
  };
}

function typeLabel(type: BaseType): string {
  switch (type.base) {
    case "Bool":
      return "Bool";
    case "Int":
      return "Int";
    case "Decimal":
      return `Decimal(${type.precision},${type.scale})`;
    case "Money":
      return `Money(${type.currency})`;
    case "Text":
      return type.max_length ? `Text(${type.max_length})` : "Text";
    case "Date":
      return "Date";
    case "DateTime":
      return "DateTime";
    case "Duration":
      return "Duration";
    case "Enum":
      return `Enum(${type.values.join("|")})`;
    case "List":
      return `List<${type.element_type.base}>`;
    case "Record":
      return `Record{${Object.keys(type.fields).join(",")}}`;
    case "TaggedUnion":
      return `Union{${Object.keys(type.variants).join("|")}}`;
  }
}

function sourceLabel(source?: FactSource): string {
  if (!source) return "—";
  if ("system" in source) return `${source.system}.${source.field}`;
  return `${source.source_id}:${source.path}`;
}

function defaultLabel(def?: FactDefault): string {
  if (def === undefined || def === null) return "—";
  if (typeof def === "boolean") return def ? "true" : "false";
  if (typeof def === "number") return String(def);
  if (typeof def === "string") return `"${def}"`;
  if (typeof def === "object" && "kind" in def) {
    if (def.kind === "bool_literal") return def.value ? "true" : "false";
    if (def.kind === "decimal_value") return def.value;
    if (def.kind === "money_value")
      return `${def.currency} ${def.amount.value}`;
  }
  return JSON.stringify(def);
}

// ---------------------------------------------------------------------------
// Default value editor
// ---------------------------------------------------------------------------

interface DefaultEditorProps {
  type: BaseType;
  value: FactDefault | undefined;
  onChange: (val: FactDefault | undefined) => void;
}

function DefaultEditor({ type, value, onChange }: DefaultEditorProps) {
  switch (type.base) {
    case "Bool": {
      const boolVal =
        value === undefined
          ? false
          : typeof value === "object" && "kind" in value && value.kind === "bool_literal"
          ? value.value
          : Boolean(value);
      return (
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={boolVal}
            onChange={(e) =>
              onChange({ kind: "bool_literal", value: e.target.checked } as BoolLiteral)
            }
          />
          <span className="text-gray-600">{boolVal ? "true" : "false"}</span>
          <button
            onClick={() => onChange(undefined)}
            className="ml-auto text-xs text-gray-400 hover:text-gray-600"
          >
            clear
          </button>
        </label>
      );
    }

    case "Int": {
      const intVal =
        value === undefined ? "" : typeof value === "number" ? String(value) : "";
      return (
        <div className="flex items-center gap-2">
          <input
            type="number"
            value={intVal}
            placeholder="No default"
            onChange={(e) =>
              e.target.value
                ? onChange(Number(e.target.value))
                : onChange(undefined)
            }
            className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
          />
        </div>
      );
    }

    case "Decimal": {
      const decVal =
        value !== undefined &&
        typeof value === "object" &&
        "kind" in value &&
        value.kind === "decimal_value"
          ? (value as DecimalValue).value
          : "";
      return (
        <div className="flex items-center gap-2">
          <input
            type="text"
            value={decVal}
            placeholder={`e.g. 1.23 (precision=${type.precision}, scale=${type.scale})`}
            onChange={(e) => {
              const v = e.target.value;
              if (!v) {
                onChange(undefined);
              } else {
                onChange({
                  kind: "decimal_value",
                  precision: type.precision,
                  scale: type.scale,
                  value: v,
                } as DecimalValue);
              }
            }}
            className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          />
        </div>
      );
    }

    case "Money": {
      const moneyVal =
        value !== undefined &&
        typeof value === "object" &&
        "kind" in value &&
        value.kind === "money_value"
          ? (value as MoneyValue)
          : null;
      const amountStr = moneyVal?.amount?.value ?? "";
      const currencyStr = moneyVal?.currency ?? type.currency;

      function handleChange(amt: string, cur: string) {
        if (!amt) {
          onChange(undefined);
          return;
        }
        onChange({
          kind: "money_value",
          currency: cur,
          amount: {
            kind: "decimal_value",
            precision: 18,
            scale: 2,
            value: amt,
          },
        } as MoneyValue);
      }

      return (
        <div className="flex items-center gap-2">
          <input
            type="text"
            value={currencyStr}
            maxLength={3}
            onChange={(e) =>
              handleChange(amountStr, e.target.value.toUpperCase())
            }
            className="w-14 rounded border border-gray-300 px-2 py-1 text-sm uppercase"
            placeholder="USD"
          />
          <input
            type="text"
            value={amountStr}
            onChange={(e) => handleChange(e.target.value, currencyStr)}
            placeholder="Amount"
            className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          />
        </div>
      );
    }

    case "Text": {
      const textVal = typeof value === "string" ? value : "";
      return (
        <input
          type="text"
          value={textVal}
          maxLength={type.max_length}
          placeholder="Default text"
          onChange={(e) =>
            e.target.value ? onChange(e.target.value) : onChange(undefined)
          }
          className="w-full rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    }

    case "Date": {
      const dateVal = typeof value === "string" ? value : "";
      return (
        <input
          type="date"
          value={dateVal}
          onChange={(e) =>
            e.target.value ? onChange(e.target.value) : onChange(undefined)
          }
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    }

    case "DateTime": {
      const dtVal = typeof value === "string" ? value : "";
      return (
        <input
          type="datetime-local"
          value={dtVal}
          onChange={(e) =>
            e.target.value ? onChange(e.target.value) : onChange(undefined)
          }
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );
    }

    case "Enum": {
      const enumVal = typeof value === "string" ? value : "";
      return (
        <select
          value={enumVal}
          onChange={(e) =>
            e.target.value ? onChange(e.target.value) : onChange(undefined)
          }
          className="w-full rounded border border-gray-300 px-2 py-1 text-sm"
        >
          <option value="">— no default —</option>
          {type.values.map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      );
    }

    default:
      return (
        <p className="text-xs text-gray-400">
          No default editor for {type.base} type.
        </p>
      );
  }
}

// ---------------------------------------------------------------------------
// Source editor
// ---------------------------------------------------------------------------

interface SourceEditorProps {
  value: FactSource | undefined;
  sourceIds: string[];
  onChange: (source: FactSource | undefined) => void;
}

type SourceMode = "none" | "freetext" | "structured";

function FactSourceEditor({ value, sourceIds, onChange }: SourceEditorProps) {
  const mode: SourceMode =
    !value
      ? "none"
      : "system" in value
      ? "freetext"
      : "structured";

  function handleModeChange(m: SourceMode) {
    if (m === "none") onChange(undefined);
    else if (m === "freetext")
      onChange({ system: "system", field: "field" } as FreetextSource);
    else onChange({ source_id: sourceIds[0] ?? "", path: "$.field" } as StructuredSource);
  }

  return (
    <div className="space-y-1">
      <select
        value={mode}
        onChange={(e) => handleModeChange(e.target.value as SourceMode)}
        className="w-full rounded border border-gray-300 px-2 py-1 text-sm"
      >
        <option value="none">No source</option>
        <option value="freetext">Freetext (system.field)</option>
        <option value="structured">Structured (source + path)</option>
      </select>

      {mode === "freetext" && value && "system" in value && (
        <div className="flex gap-1">
          <input
            type="text"
            value={(value as FreetextSource).system}
            placeholder="system"
            onChange={(e) =>
              onChange({ ...(value as FreetextSource), system: e.target.value })
            }
            className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          />
          <span className="self-center text-gray-400">.</span>
          <input
            type="text"
            value={(value as FreetextSource).field}
            placeholder="field"
            onChange={(e) =>
              onChange({ ...(value as FreetextSource), field: e.target.value })
            }
            className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          />
        </div>
      )}

      {mode === "structured" && value && "source_id" in value && (
        <div className="flex gap-1">
          <select
            value={(value as StructuredSource).source_id}
            onChange={(e) =>
              onChange({ ...(value as StructuredSource), source_id: e.target.value })
            }
            className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
          >
            {sourceIds.length === 0 && (
              <option value="">— no sources defined —</option>
            )}
            {sourceIds.map((id) => (
              <option key={id} value={id}>
                {id}
              </option>
            ))}
          </select>
          <input
            type="text"
            value={(value as StructuredSource).path}
            placeholder="$.field"
            onChange={(e) =>
              onChange({ ...(value as StructuredSource), path: e.target.value })
            }
            className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Fact row
// ---------------------------------------------------------------------------

interface FactRowProps {
  fact: FactConstruct;
  allFactIds: string[];
  sourceIds: string[];
  onUpdate: (id: string, updates: Partial<FactConstruct>) => void;
  onDelete: (id: string) => void;
}

function FactRow({ fact, allFactIds, sourceIds, onUpdate, onDelete }: FactRowProps) {
  const [expanded, setExpanded] = useState(false);
  const [idDraft, setIdDraft] = useState(fact.id);
  const [idError, setIdError] = useState<string | null>(null);

  function handleIdBlur() {
    const trimmed = idDraft.trim();
    if (!trimmed) {
      setIdError("ID cannot be empty.");
      setIdDraft(fact.id);
      return;
    }
    if (trimmed !== fact.id && allFactIds.includes(trimmed)) {
      setIdError(`ID "${trimmed}" already exists.`);
      setIdDraft(fact.id);
      return;
    }
    setIdError(null);
    if (trimmed !== fact.id) {
      onUpdate(fact.id, { id: trimmed });
    }
  }

  function handleTypeChange(newType: BaseType) {
    // Clear default when type changes
    onUpdate(fact.id, { type: newType, default: undefined });
  }

  return (
    <>
      {/* Summary row */}
      <tr
        className={`cursor-pointer transition-colors ${
          expanded ? "bg-blue-50" : "hover:bg-gray-50"
        }`}
        onClick={() => setExpanded((e) => !e)}
      >
        <td className="px-3 py-2">
          <span className="font-mono text-sm">{fact.id}</span>
        </td>
        <td className="px-3 py-2 text-xs text-gray-600">{typeLabel(fact.type)}</td>
        <td className="px-3 py-2 text-xs text-gray-500">{sourceLabel(fact.source)}</td>
        <td className="px-3 py-2 text-xs text-gray-500">{defaultLabel(fact.default)}</td>
        <td className="px-3 py-2 text-right">
          <button
            onClick={(e) => {
              e.stopPropagation();
              onDelete(fact.id);
            }}
            className="rounded px-1.5 py-0.5 text-xs text-red-400 hover:bg-red-50"
          >
            ×
          </button>
        </td>
      </tr>

      {/* Detail row */}
      {expanded && (
        <tr className="bg-blue-50">
          <td colSpan={5} className="px-4 py-3">
            <div className="grid grid-cols-2 gap-4">
              {/* Left: ID + Type */}
              <div className="space-y-3">
                <div>
                  <label className="block text-xs font-medium text-gray-600">
                    Fact ID
                  </label>
                  <input
                    type="text"
                    value={idDraft}
                    onChange={(e) => setIdDraft(e.target.value)}
                    onBlur={handleIdBlur}
                    className={`mt-0.5 w-full rounded border px-2 py-1 font-mono text-sm ${
                      idError
                        ? "border-red-400 bg-red-50"
                        : "border-gray-300"
                    }`}
                  />
                  {idError && (
                    <p className="mt-0.5 text-xs text-red-500">{idError}</p>
                  )}
                </div>

                <div>
                  <label className="block text-xs font-medium text-gray-600">
                    Type
                  </label>
                  <div className="mt-0.5">
                    <TypePicker
                      value={fact.type}
                      onChange={handleTypeChange}
                    />
                  </div>
                </div>
              </div>

              {/* Right: Default + Source */}
              <div className="space-y-3">
                <div>
                  <label className="block text-xs font-medium text-gray-600">
                    Default value
                  </label>
                  <div className="mt-0.5">
                    <DefaultEditor
                      type={fact.type}
                      value={fact.default}
                      onChange={(val) => onUpdate(fact.id, { default: val })}
                    />
                  </div>
                </div>

                <div>
                  <label className="block text-xs font-medium text-gray-600">
                    Source
                  </label>
                  <div className="mt-0.5">
                    <FactSourceEditor
                      value={fact.source}
                      sourceIds={sourceIds}
                      onChange={(src) => onUpdate(fact.id, { source: src })}
                    />
                  </div>
                </div>
              </div>
            </div>
          </td>
        </tr>
      )}
    </>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function FactEditor() {
  const facts = useContractStore(selectFacts);
  const sources = useContractStore(selectSources);
  const addConstruct = useContractStore((s) => s.addConstruct);
  const updateConstruct = useContractStore((s) => s.updateConstruct);
  const removeConstruct = useContractStore((s) => s.removeConstruct);

  const sourceIds = sources.map((s) => s.id);

  function handleAddFact() {
    const baseId = "new_fact";
    let id = baseId;
    let i = 1;
    while (facts.some((f) => f.id === id)) {
      id = `${baseId}_${i++}`;
    }
    addConstruct(newFact(id));
  }

  function handleUpdate(id: string, updates: Partial<FactConstruct>) {
    // If ID changed, we need to re-add under new ID
    if (updates.id && updates.id !== id) {
      const existing = facts.find((f) => f.id === id);
      if (existing) {
        removeConstruct(id, "Fact");
        addConstruct({ ...existing, ...updates });
      }
    } else {
      updateConstruct(id, "Fact", updates);
    }
  }

  function handleDelete(id: string) {
    removeConstruct(id, "Fact");
  }

  const allFactIds = facts.map((f) => f.id);

  return (
    <div className="flex flex-col p-4">
      <div className="mb-3 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-gray-700">
          Facts ({facts.length})
        </h2>
        <button
          onClick={handleAddFact}
          className="rounded bg-blue-500 px-3 py-1 text-xs text-white hover:bg-blue-600"
        >
          + Add Fact
        </button>
      </div>

      {facts.length === 0 ? (
        <div className="rounded border-2 border-dashed border-gray-200 py-12 text-center text-sm text-gray-400">
          No facts yet — click "+ Add Fact" to create one.
        </div>
      ) : (
        <div className="overflow-x-auto rounded border border-gray-200 bg-white">
          <table className="min-w-full text-sm">
            <thead>
              <tr className="border-b border-gray-200 bg-gray-50 text-left text-xs font-semibold uppercase text-gray-500">
                <th className="px-3 py-2">ID</th>
                <th className="px-3 py-2">Type</th>
                <th className="px-3 py-2">Source</th>
                <th className="px-3 py-2">Default</th>
                <th className="px-3 py-2" />
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {facts.map((fact) => (
                <FactRow
                  key={fact.id}
                  fact={fact}
                  allFactIds={allFactIds}
                  sourceIds={sourceIds}
                  onUpdate={handleUpdate}
                  onDelete={handleDelete}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
