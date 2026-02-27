/**
 * FactInputPanel: Type-aware fact value input panel for simulation.
 *
 * Renders type-appropriate controls for every declared fact, with entity
 * state override dropdowns at the bottom.  Calls the simulation store
 * evaluate() when the user clicks "Evaluate".
 */
import React, { useCallback } from "react";
import { useContractStore } from "@/store/contract";
import { useSimulationStore } from "@/store/simulation";
import type { BaseType, FactConstruct, EntityConstruct } from "@/types/interchange";
import type { FactValue } from "@/store/simulation";

// ---------------------------------------------------------------------------
// Recursive type-appropriate input component
// ---------------------------------------------------------------------------

interface TypeInputProps {
  type: BaseType;
  value: FactValue;
  onChange: (v: FactValue) => void;
  compact?: boolean;
}

function TypeInput({ type, value, onChange, compact }: TypeInputProps) {
  const inputCls = `w-full rounded border border-gray-300 px-2 py-1 text-sm focus:border-blue-400 focus:outline-none${compact ? " py-0.5" : ""}`;

  switch (type.base) {
    case "Bool": {
      const checked = value === true || value === "true";
      return (
        <label className="flex cursor-pointer items-center gap-2">
          <input
            type="checkbox"
            checked={checked}
            onChange={(e) => onChange(e.target.checked)}
            className="h-4 w-4 rounded border-gray-300 text-blue-600"
          />
          <span className="text-sm text-gray-600">{checked ? "true" : "false"}</span>
        </label>
      );
    }

    case "Int": {
      const intType = type as { base: "Int"; min?: number; max?: number };
      return (
        <input
          type="number"
          step={1}
          min={intType.min}
          max={intType.max}
          value={typeof value === "number" ? value : 0}
          onChange={(e) => onChange(parseInt(e.target.value, 10) || 0)}
          className={inputCls}
        />
      );
    }

    case "Decimal": {
      const decType = type as { base: "Decimal"; precision: number; scale: number };
      const step = decType.scale > 0 ? `0.${"0".repeat(decType.scale - 1)}1` : "1";
      return (
        <input
          type="text"
          inputMode="decimal"
          placeholder={`0.${"0".repeat(decType.scale)}`}
          value={typeof value === "string" ? value : "0"}
          onChange={(e) => onChange(e.target.value)}
          className={inputCls}
          title={`Decimal(${decType.precision},${decType.scale}) — step ${step}`}
        />
      );
    }

    case "Money": {
      const moneyType = type as { base: "Money"; currency: string };
      const moneyVal =
        typeof value === "object" && value !== null && !Array.isArray(value)
          ? (value as { amount?: string; currency?: string })
          : { amount: "0.00", currency: moneyType.currency };
      return (
        <div className="flex items-center gap-1">
          <input
            type="text"
            inputMode="decimal"
            placeholder="0.00"
            value={moneyVal.amount ?? "0.00"}
            onChange={(e) =>
              onChange({ amount: e.target.value, currency: moneyType.currency })
            }
            className={`flex-1 ${inputCls}`}
          />
          <span className="text-sm font-medium text-gray-500">
            {moneyType.currency}
          </span>
        </div>
      );
    }

    case "Text": {
      const textType = type as { base: "Text"; max_length?: number };
      return (
        <input
          type="text"
          maxLength={textType.max_length}
          value={typeof value === "string" ? value : ""}
          onChange={(e) => onChange(e.target.value)}
          placeholder={textType.max_length ? `max ${textType.max_length} chars` : ""}
          className={inputCls}
        />
      );
    }

    case "Date":
      return (
        <input
          type="date"
          value={typeof value === "string" ? value : "2024-01-01"}
          onChange={(e) => onChange(e.target.value)}
          className={inputCls}
        />
      );

    case "DateTime":
      return (
        <input
          type="datetime-local"
          value={
            typeof value === "string"
              ? value.replace("Z", "")
              : "2024-01-01T00:00"
          }
          onChange={(e) => onChange(e.target.value + "Z")}
          className={inputCls}
        />
      );

    case "Duration":
      return (
        <input
          type="number"
          step={1}
          min={0}
          value={typeof value === "number" ? value : 0}
          onChange={(e) => onChange(parseInt(e.target.value, 10) || 0)}
          className={inputCls}
        />
      );

    case "Enum": {
      const enumType = type as { base: "Enum"; values: string[] };
      return (
        <select
          value={typeof value === "string" ? value : enumType.values[0] ?? ""}
          onChange={(e) => onChange(e.target.value)}
          className={inputCls}
        >
          {enumType.values.map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      );
    }

    case "List": {
      const listType = type as { base: "List"; element_type: BaseType; max?: number };
      const arr = Array.isArray(value) ? (value as FactValue[]) : [];
      function updateItem(i: number, v: FactValue) {
        const next = [...arr];
        next[i] = v;
        onChange(next as unknown[]);
      }
      function addItem() {
        if (listType.max && arr.length >= listType.max) return;
        onChange([...arr, null] as unknown[]);
      }
      function removeItem(i: number) {
        const next = arr.filter((_, idx) => idx !== i);
        onChange(next as unknown[]);
      }
      return (
        <div className="space-y-1">
          {arr.map((item, i) => (
            <div key={i} className="flex items-start gap-1">
              <div className="flex-1">
                <TypeInput
                  type={listType.element_type}
                  value={item}
                  onChange={(v) => updateItem(i, v)}
                  compact
                />
              </div>
              <button
                onClick={() => removeItem(i)}
                className="mt-0.5 rounded px-1 py-0.5 text-xs text-red-500 hover:bg-red-50"
                title="Remove item"
              >
                ×
              </button>
            </div>
          ))}
          <button
            onClick={addItem}
            disabled={!!(listType.max && arr.length >= listType.max)}
            className="rounded border border-dashed border-gray-300 px-2 py-0.5 text-xs text-gray-500 hover:border-blue-400 hover:text-blue-600 disabled:opacity-40"
          >
            + Add item
          </button>
          {listType.max && (
            <span className="text-xs text-gray-400">
              {arr.length}/{listType.max} items
            </span>
          )}
        </div>
      );
    }

    case "Record": {
      const recType = type as { base: "Record"; fields: Record<string, BaseType> };
      const rec =
        typeof value === "object" && value !== null && !Array.isArray(value)
          ? (value as Record<string, FactValue>)
          : {};
      function updateField(k: string, v: FactValue) {
        onChange({ ...rec, [k]: v });
      }
      return (
        <div className="space-y-1 rounded border border-gray-200 p-2">
          {Object.entries(recType.fields).map(([k, fieldType]) => (
            <div key={k} className="flex items-center gap-2">
              <span className="w-24 shrink-0 text-xs font-medium text-gray-500">
                {k}
              </span>
              <div className="flex-1">
                <TypeInput
                  type={fieldType}
                  value={rec[k] ?? null}
                  onChange={(v) => updateField(k, v)}
                  compact
                />
              </div>
            </div>
          ))}
        </div>
      );
    }

    case "TaggedUnion":
      return (
        <span className="text-xs text-gray-400 italic">
          (TaggedUnion — JSON edit not supported)
        </span>
      );

    default:
      return (
        <input
          type="text"
          value={typeof value === "string" ? value : JSON.stringify(value)}
          onChange={(e) => {
            try {
              onChange(JSON.parse(e.target.value));
            } catch {
              onChange(e.target.value);
            }
          }}
          className={inputCls}
        />
      );
  }
}

// ---------------------------------------------------------------------------
// Single fact row
// ---------------------------------------------------------------------------

interface FactRowProps {
  fact: FactConstruct;
  value: FactValue;
  onChange: (v: FactValue) => void;
}

function FactRow({ fact, value, onChange }: FactRowProps) {
  return (
    <div className="group rounded border border-gray-100 bg-white p-3 hover:border-blue-200">
      <div className="mb-1.5 flex items-center justify-between">
        <span className="font-mono text-sm font-medium text-gray-800">
          {fact.id}
        </span>
        <span className="text-xs text-gray-400">{fact.type.base}</span>
      </div>
      <TypeInput type={fact.type} value={value} onChange={onChange} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main FactInputPanel
// ---------------------------------------------------------------------------

export interface FactInputPanelProps {
  compact?: boolean;
  onEvaluate?: () => void;
}

export function FactInputPanel({ compact, onEvaluate }: FactInputPanelProps) {
  const facts = useContractStore((s) => s.facts());
  const entities = useContractStore((s) => s.entities());
  const factValues = useSimulationStore((s) => s.factValues);
  const entityStates = useSimulationStore((s) => s.entityStates);
  const isEvaluating = useSimulationStore((s) => s.isEvaluating);
  const evaluationError = useSimulationStore((s) => s.evaluationError);
  const setFactValue = useSimulationStore((s) => s.setFactValue);
  const setEntityState = useSimulationStore((s) => s.setEntityState);
  const evaluate = useSimulationStore((s) => s.evaluate);
  const initFromContract = useSimulationStore((s) => s.initFromContract);

  const handleEvaluate = useCallback(async () => {
    await evaluate();
    onEvaluate?.();
  }, [evaluate, onEvaluate]);

  const handleReset = useCallback(() => {
    initFromContract();
  }, [initFromContract]);

  if (facts.length === 0 && entities.length === 0) {
    return (
      <div className="flex h-40 items-center justify-center rounded border-2 border-dashed border-gray-200 text-sm text-gray-400">
        No facts or entities declared in this contract.
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* Fact inputs */}
      <div className={`flex-1 overflow-y-auto ${compact ? "space-y-2 p-2" : "space-y-2 p-3"}`}>
        {facts.length > 0 && (
          <section>
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
              Fact Values
            </h3>
            <div className="space-y-2">
              {facts.map((fact) => (
                <FactRow
                  key={fact.id}
                  fact={fact}
                  value={factValues[fact.id] ?? null}
                  onChange={(v) => setFactValue(fact.id, v)}
                />
              ))}
            </div>
          </section>
        )}

        {entities.length > 0 && (
          <section className="mt-4">
            <h3 className="mb-2 text-xs font-semibold uppercase tracking-wide text-gray-400">
              Entity State Overrides
            </h3>
            <div className="space-y-2">
              {entities.map((entity: EntityConstruct) => (
                <div
                  key={entity.id}
                  className="flex items-center gap-3 rounded border border-gray-100 bg-white p-2"
                >
                  <span className="w-32 shrink-0 font-mono text-sm text-gray-700">
                    {entity.id}
                  </span>
                  <select
                    value={entityStates[entity.id] ?? entity.initial}
                    onChange={(e) => setEntityState(entity.id, e.target.value)}
                    className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
                  >
                    {entity.states.map((s) => (
                      <option key={s} value={s}>
                        {s}
                        {s === entity.initial ? " (initial)" : ""}
                      </option>
                    ))}
                  </select>
                </div>
              ))}
            </div>
          </section>
        )}
      </div>

      {/* Error message */}
      {evaluationError && (
        <div className="mx-3 mb-2 rounded border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
          {evaluationError}
        </div>
      )}

      {/* Action buttons */}
      <div className={`flex gap-2 border-t border-gray-100 bg-gray-50 ${compact ? "p-2" : "p-3"}`}>
        <button
          onClick={handleEvaluate}
          disabled={isEvaluating}
          className="flex-1 rounded bg-blue-600 px-3 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
        >
          {isEvaluating ? "Evaluating..." : "Evaluate"}
        </button>
        <button
          onClick={handleReset}
          className="rounded border border-gray-200 bg-white px-3 py-2 text-sm text-gray-600 hover:bg-gray-50"
          title="Reset to defaults"
        >
          Reset
        </button>
      </div>
    </div>
  );
}
