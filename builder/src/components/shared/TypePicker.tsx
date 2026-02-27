/**
 * TypePicker: reusable component for selecting Tenor base types.
 *
 * Supports all BaseType variants. Parameterized types show sub-fields:
 * - Money: currency text input
 * - Enum: list of value strings with add/remove
 * - List: element type (recursive TypePicker) + max number input
 * - Record: field list with name + type (recursive TypePicker) per field
 * - TaggedUnion: variant list with name + type per variant
 * - Decimal: precision + scale inputs
 */
import React from "react";
import type { BaseType } from "@/types/interchange";

type BaseTypeName = BaseType["base"];

const BASE_TYPES: BaseTypeName[] = [
  "Bool",
  "Int",
  "Decimal",
  "Money",
  "Text",
  "Date",
  "DateTime",
  "Duration",
  "Enum",
  "List",
  "Record",
  "TaggedUnion",
];

interface TypePickerProps {
  value: BaseType;
  onChange: (type: BaseType) => void;
  label?: string;
  depth?: number;
}

const MAX_DEPTH = 4; // prevent infinite recursion in nested types

export function TypePicker({
  value,
  onChange,
  label,
  depth = 0,
}: TypePickerProps) {
  function handleBaseChange(base: BaseTypeName) {
    onChange(defaultForBase(base));
  }

  return (
    <div className="space-y-2">
      {label && (
        <label className="block text-sm font-medium text-gray-700">
          {label}
        </label>
      )}
      <select
        value={value.base}
        onChange={(e) => handleBaseChange(e.target.value as BaseTypeName)}
        className="w-full rounded border border-gray-300 bg-white px-3 py-1.5 text-sm focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
      >
        {BASE_TYPES.map((t) => (
          <option key={t} value={t}>
            {t}
          </option>
        ))}
      </select>

      {/* Parameterized type sub-fields */}
      {value.base === "Decimal" && (
        <DecimalParams
          precision={value.precision}
          scale={value.scale}
          onChange={(precision, scale) =>
            onChange({ ...value, precision, scale })
          }
        />
      )}

      {value.base === "Money" && (
        <MoneyParams
          currency={value.currency}
          onChange={(currency) => onChange({ ...value, currency })}
        />
      )}

      {value.base === "Text" && (
        <TextParams
          maxLength={value.max_length}
          onChange={(maxLength) =>
            onChange({ ...value, max_length: maxLength })
          }
        />
      )}

      {value.base === "Int" && (
        <IntParams
          min={value.min}
          max={value.max}
          onChange={(min, max) => onChange({ ...value, min, max })}
        />
      )}

      {value.base === "Enum" && (
        <EnumParams
          values={value.values}
          onChange={(values) => onChange({ ...value, values })}
        />
      )}

      {value.base === "List" && depth < MAX_DEPTH && (
        <ListParams
          elementType={value.element_type}
          max={value.max}
          onChange={(elementType, max) =>
            onChange({ ...value, element_type: elementType, max })
          }
          depth={depth + 1}
        />
      )}

      {value.base === "Record" && depth < MAX_DEPTH && (
        <RecordParams
          fields={value.fields}
          onChange={(fields) => onChange({ ...value, fields })}
          depth={depth + 1}
        />
      )}

      {value.base === "TaggedUnion" && depth < MAX_DEPTH && (
        <TaggedUnionParams
          variants={value.variants}
          onChange={(variants) => onChange({ ...value, variants })}
          depth={depth + 1}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Sub-field components
// ---------------------------------------------------------------------------

function DecimalParams({
  precision,
  scale,
  onChange,
}: {
  precision: number;
  scale: number;
  onChange: (precision: number, scale: number) => void;
}) {
  return (
    <div className="flex gap-2">
      <label className="flex flex-1 flex-col gap-1">
        <span className="text-xs text-gray-500">Precision</span>
        <input
          type="number"
          min={1}
          value={precision}
          onChange={(e) => onChange(Number(e.target.value), scale)}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      </label>
      <label className="flex flex-1 flex-col gap-1">
        <span className="text-xs text-gray-500">Scale</span>
        <input
          type="number"
          min={0}
          value={scale}
          onChange={(e) => onChange(precision, Number(e.target.value))}
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      </label>
    </div>
  );
}

function MoneyParams({
  currency,
  onChange,
}: {
  currency: string;
  onChange: (currency: string) => void;
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs text-gray-500">Currency (ISO 4217)</span>
      <input
        type="text"
        value={currency}
        placeholder="USD"
        maxLength={3}
        onChange={(e) => onChange(e.target.value.toUpperCase())}
        className="rounded border border-gray-300 px-2 py-1 text-sm uppercase"
      />
    </label>
  );
}

function TextParams({
  maxLength,
  onChange,
}: {
  maxLength?: number;
  onChange: (maxLength: number | undefined) => void;
}) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-xs text-gray-500">Max length (optional)</span>
      <input
        type="number"
        min={1}
        value={maxLength ?? ""}
        placeholder="unlimited"
        onChange={(e) =>
          onChange(e.target.value ? Number(e.target.value) : undefined)
        }
        className="rounded border border-gray-300 px-2 py-1 text-sm"
      />
    </label>
  );
}

function IntParams({
  min,
  max,
  onChange,
}: {
  min?: number;
  max?: number;
  onChange: (min: number | undefined, max: number | undefined) => void;
}) {
  return (
    <div className="flex gap-2">
      <label className="flex flex-1 flex-col gap-1">
        <span className="text-xs text-gray-500">Min (optional)</span>
        <input
          type="number"
          value={min ?? ""}
          placeholder="none"
          onChange={(e) =>
            onChange(
              e.target.value ? Number(e.target.value) : undefined,
              max
            )
          }
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      </label>
      <label className="flex flex-1 flex-col gap-1">
        <span className="text-xs text-gray-500">Max (optional)</span>
        <input
          type="number"
          value={max ?? ""}
          placeholder="none"
          onChange={(e) =>
            onChange(
              min,
              e.target.value ? Number(e.target.value) : undefined
            )
          }
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      </label>
    </div>
  );
}

function EnumParams({
  values,
  onChange,
}: {
  values: string[];
  onChange: (values: string[]) => void;
}) {
  function addValue() {
    onChange([...values, ""]);
  }

  function updateValue(idx: number, v: string) {
    const next = [...values];
    next[idx] = v;
    onChange(next);
  }

  function removeValue(idx: number) {
    onChange(values.filter((_, i) => i !== idx));
  }

  return (
    <div className="space-y-1">
      <span className="text-xs text-gray-500">Values</span>
      {values.map((v, idx) => (
        <div key={idx} className="flex gap-1">
          <input
            type="text"
            value={v}
            placeholder={`value_${idx + 1}`}
            onChange={(e) => updateValue(idx, e.target.value)}
            className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
          />
          <button
            onClick={() => removeValue(idx)}
            className="rounded border border-gray-300 px-2 py-1 text-xs text-red-500 hover:bg-red-50"
            title="Remove"
          >
            ×
          </button>
        </div>
      ))}
      <button
        onClick={addValue}
        className="rounded border border-dashed border-gray-300 px-2 py-1 text-xs text-gray-500 hover:bg-gray-50"
      >
        + Add value
      </button>
    </div>
  );
}

function ListParams({
  elementType,
  max,
  onChange,
  depth,
}: {
  elementType: BaseType;
  max?: number;
  onChange: (elementType: BaseType, max: number | undefined) => void;
  depth: number;
}) {
  return (
    <div className="space-y-2 border-l-2 border-gray-200 pl-3">
      <TypePicker
        value={elementType}
        onChange={(t) => onChange(t, max)}
        label="Element type"
        depth={depth}
      />
      <label className="flex flex-col gap-1">
        <span className="text-xs text-gray-500">Max length (optional)</span>
        <input
          type="number"
          min={1}
          value={max ?? ""}
          placeholder="unlimited"
          onChange={(e) =>
            onChange(
              elementType,
              e.target.value ? Number(e.target.value) : undefined
            )
          }
          className="rounded border border-gray-300 px-2 py-1 text-sm"
        />
      </label>
    </div>
  );
}

function RecordParams({
  fields,
  onChange,
  depth,
}: {
  fields: Record<string, BaseType>;
  onChange: (fields: Record<string, BaseType>) => void;
  depth: number;
}) {
  const entries = Object.entries(fields);

  function addField() {
    const name = `field_${entries.length + 1}`;
    onChange({ ...fields, [name]: { base: "Text" } });
  }

  function updateFieldName(oldName: string, newName: string) {
    const next: Record<string, BaseType> = {};
    for (const [k, v] of Object.entries(fields)) {
      next[k === oldName ? newName : k] = v;
    }
    onChange(next);
  }

  function updateFieldType(name: string, type: BaseType) {
    onChange({ ...fields, [name]: type });
  }

  function removeField(name: string) {
    const next = { ...fields };
    delete next[name];
    onChange(next);
  }

  return (
    <div className="space-y-2">
      <span className="text-xs text-gray-500">Fields</span>
      {entries.map(([name, type]) => (
        <div
          key={name}
          className="space-y-1 rounded border border-gray-200 p-2"
        >
          <div className="flex gap-1">
            <input
              type="text"
              value={name}
              placeholder="field_name"
              onChange={(e) => updateFieldName(name, e.target.value)}
              className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm font-mono"
            />
            <button
              onClick={() => removeField(name)}
              className="rounded border border-gray-300 px-2 py-1 text-xs text-red-500 hover:bg-red-50"
              title="Remove field"
            >
              ×
            </button>
          </div>
          <TypePicker
            value={type}
            onChange={(t) => updateFieldType(name, t)}
            depth={depth}
          />
        </div>
      ))}
      <button
        onClick={addField}
        className="rounded border border-dashed border-gray-300 px-2 py-1 text-xs text-gray-500 hover:bg-gray-50"
      >
        + Add field
      </button>
    </div>
  );
}

function TaggedUnionParams({
  variants,
  onChange,
  depth,
}: {
  variants: Record<string, BaseType>;
  onChange: (variants: Record<string, BaseType>) => void;
  depth: number;
}) {
  const entries = Object.entries(variants);

  function addVariant() {
    const name = `variant_${entries.length + 1}`;
    onChange({ ...variants, [name]: { base: "Bool" } });
  }

  function updateVariantName(oldName: string, newName: string) {
    const next: Record<string, BaseType> = {};
    for (const [k, v] of Object.entries(variants)) {
      next[k === oldName ? newName : k] = v;
    }
    onChange(next);
  }

  function updateVariantType(name: string, type: BaseType) {
    onChange({ ...variants, [name]: type });
  }

  function removeVariant(name: string) {
    const next = { ...variants };
    delete next[name];
    onChange(next);
  }

  return (
    <div className="space-y-2">
      <span className="text-xs text-gray-500">Variants</span>
      {entries.map(([name, type]) => (
        <div
          key={name}
          className="space-y-1 rounded border border-gray-200 p-2"
        >
          <div className="flex gap-1">
            <input
              type="text"
              value={name}
              placeholder="variant_name"
              onChange={(e) => updateVariantName(name, e.target.value)}
              className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm font-mono"
            />
            <button
              onClick={() => removeVariant(name)}
              className="rounded border border-gray-300 px-2 py-1 text-xs text-red-500 hover:bg-red-50"
              title="Remove variant"
            >
              ×
            </button>
          </div>
          <TypePicker
            value={type}
            onChange={(t) => updateVariantType(name, t)}
            depth={depth}
          />
        </div>
      ))}
      <button
        onClick={addVariant}
        className="rounded border border-dashed border-gray-300 px-2 py-1 text-xs text-gray-500 hover:bg-gray-50"
      >
        + Add variant
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Default type factory
// ---------------------------------------------------------------------------

function defaultForBase(base: BaseTypeName): BaseType {
  switch (base) {
    case "Bool":
      return { base: "Bool" };
    case "Int":
      return { base: "Int" };
    case "Decimal":
      return { base: "Decimal", precision: 10, scale: 2 };
    case "Money":
      return { base: "Money", currency: "USD" };
    case "Text":
      return { base: "Text" };
    case "Date":
      return { base: "Date" };
    case "DateTime":
      return { base: "DateTime" };
    case "Duration":
      return { base: "Duration" };
    case "Enum":
      return { base: "Enum", values: ["value_1"] };
    case "List":
      return { base: "List", element_type: { base: "Text" } };
    case "Record":
      return { base: "Record", fields: {} };
    case "TaggedUnion":
      return { base: "TaggedUnion", variants: {} };
  }
}
