/**
 * PredicateBuilder: visual expression builder for Tenor predicate expressions.
 *
 * Constructs valid interchange JSON predicate nodes:
 * - Compare: left op right (fact_ref, literal, or field_ref operands)
 * - And: binary and
 * - Or: binary or
 * - Not: negated sub-expression
 * - VerdictPresent: check if a verdict exists
 * - ForAll / Exists: quantified expressions over List facts
 *
 * Props:
 *   value         - current PredicateExpression (or null for empty)
 *   onChange      - called when expression changes
 *   availableFacts  - facts from which fact_refs can be chosen
 *   availableVerdicts - verdict IDs that can appear in VerdictPresent
 *   mode          - "rule" restricts available verdicts; "operation" allows all
 */
import React, { useState } from "react";
import type {
  PredicateExpression,
  ExpressionOperand,
  FactConstruct,
  BaseType,
  CompareExpr,
  AndExpr,
  OrExpr,
  NotExpr,
  ForallExpr,
  ExistsExpr,
  VerdictPresentExpr,
  FactRefOperand,
  LiteralOperand,
} from "@/types/interchange";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type PredicateMode = "rule" | "operation";

export interface PredicateBuilderProps {
  value: PredicateExpression | null;
  onChange: (expr: PredicateExpression) => void;
  availableFacts: FactConstruct[];
  availableVerdicts: string[];
  mode: PredicateMode;
}

type ExprType =
  | "Compare"
  | "And"
  | "Or"
  | "Not"
  | "VerdictPresent"
  | "ForAll"
  | "Exists";

// ---------------------------------------------------------------------------
// Default constructors
// ---------------------------------------------------------------------------

function defaultFactRef(facts: FactConstruct[]): FactRefOperand {
  return { fact_ref: facts[0]?.id ?? "fact" };
}

function defaultLiteral(type: BaseType): LiteralOperand {
  switch (type.base) {
    case "Bool":
      return { literal: true, type };
    case "Int":
      return { literal: 0, type };
    case "Decimal":
      return { literal: "0.00", type };
    case "Money":
      return { literal: "0.00", type };
    case "Text":
      return { literal: "", type };
    case "Date":
      return { literal: "", type };
    case "DateTime":
      return { literal: "", type };
    default:
      return { literal: "", type };
  }
}

function defaultCompare(facts: FactConstruct[]): CompareExpr {
  const firstFact = facts[0];
  const left: FactRefOperand = { fact_ref: firstFact?.id ?? "fact" };
  const right: LiteralOperand = firstFact
    ? defaultLiteral(firstFact.type)
    : { literal: "", type: { base: "Text" } };
  return { left, op: "=", right };
}

function defaultAnd(facts: FactConstruct[]): AndExpr {
  return {
    left: defaultCompare(facts),
    op: "and",
    right: defaultCompare(facts),
  };
}

function defaultOr(facts: FactConstruct[]): OrExpr {
  return {
    left: defaultCompare(facts),
    op: "or",
    right: defaultCompare(facts),
  };
}

function defaultNot(facts: FactConstruct[]): NotExpr {
  return { op: "not", operand: defaultCompare(facts) };
}

function defaultVerdictPresent(verdicts: string[]): VerdictPresentExpr {
  return { verdict_present: verdicts[0] ?? "verdict" };
}

function defaultForAll(facts: FactConstruct[]): ForallExpr {
  const listFact = facts.find((f) => f.type.base === "List") ?? facts[0];
  const elementType: BaseType =
    listFact?.type.base === "List"
      ? listFact.type.element_type
      : { base: "Bool" };
  return {
    quantifier: "forall",
    variable: "x",
    domain: { fact_ref: listFact?.id ?? "list_fact" },
    variable_type: elementType,
    body: defaultCompare(facts),
  };
}

function defaultExists(facts: FactConstruct[]): ExistsExpr {
  const listFact = facts.find((f) => f.type.base === "List") ?? facts[0];
  const elementType: BaseType =
    listFact?.type.base === "List"
      ? listFact.type.element_type
      : { base: "Bool" };
  return {
    quantifier: "exists",
    variable: "x",
    domain: { fact_ref: listFact?.id ?? "list_fact" },
    variable_type: elementType,
    body: defaultCompare(facts),
  };
}

function defaultExpr(
  type: ExprType,
  facts: FactConstruct[],
  verdicts: string[]
): PredicateExpression {
  switch (type) {
    case "Compare":
      return defaultCompare(facts);
    case "And":
      return defaultAnd(facts);
    case "Or":
      return defaultOr(facts);
    case "Not":
      return defaultNot(facts);
    case "VerdictPresent":
      return defaultVerdictPresent(verdicts);
    case "ForAll":
      return defaultForAll(facts);
    case "Exists":
      return defaultExists(facts);
  }
}

// ---------------------------------------------------------------------------
// Detect expression type
// ---------------------------------------------------------------------------

function exprType(expr: PredicateExpression): ExprType {
  if ("verdict_present" in expr) return "VerdictPresent";
  if ("quantifier" in expr) {
    const e = expr as ForallExpr | ExistsExpr;
    return e.quantifier === "forall" ? "ForAll" : "Exists";
  }
  if ("op" in expr) {
    const op = (expr as { op: string }).op;
    if (op === "and") return "And";
    if (op === "or") return "Or";
    if (op === "not") return "Not";
  }
  return "Compare";
}

// ---------------------------------------------------------------------------
// Helper: get fact type by ID
// ---------------------------------------------------------------------------

function factType(facts: FactConstruct[], factId: string): BaseType | null {
  return facts.find((f) => f.id === factId)?.type ?? null;
}

// ---------------------------------------------------------------------------
// Compare operators available per type
// ---------------------------------------------------------------------------

function comparisonOpsForType(type: BaseType | null): CompareExpr["op"][] {
  if (!type) return ["=", "!=", "<", "<=", ">", ">="];
  switch (type.base) {
    case "Bool":
      return ["=", "!="];
    case "Text":
      return ["=", "!="];
    case "Enum":
      return ["=", "!="];
    case "Date":
    case "DateTime":
    case "Duration":
    case "Int":
    case "Decimal":
    case "Money":
      return ["=", "!=", "<", "<=", ">", ">="];
    default:
      return ["=", "!="];
  }
}

// ---------------------------------------------------------------------------
// Literal input for right side of comparison
// ---------------------------------------------------------------------------

interface LiteralInputProps {
  type: BaseType | null;
  value: boolean | number | string | object;
  onChange: (v: boolean | number | string | object) => void;
}

function LiteralInput({ type, value, onChange }: LiteralInputProps) {
  if (!type) {
    return (
      <input
        type="text"
        value={String(value)}
        onChange={(e) => onChange(e.target.value)}
        className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        placeholder="value"
      />
    );
  }

  switch (type.base) {
    case "Bool":
      return (
        <select
          value={String(value)}
          onChange={(e) => onChange(e.target.value === "true")}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        >
          <option value="true">true</option>
          <option value="false">false</option>
        </select>
      );

    case "Int":
      return (
        <input
          type="number"
          value={typeof value === "number" ? value : 0}
          onChange={(e) => onChange(Number(e.target.value))}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );

    case "Decimal":
    case "Money":
      return (
        <input
          type="text"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 rounded border border-gray-300 px-2 py-1 font-mono text-sm"
          placeholder="0.00"
        />
      );

    case "Enum":
      return (
        <select
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        >
          {type.values.map((v) => (
            <option key={v} value={v}>
              {v}
            </option>
          ))}
        </select>
      );

    case "Date":
      return (
        <input
          type="date"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );

    case "DateTime":
      return (
        <input
          type="datetime-local"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        />
      );

    default:
      return (
        <input
          type="text"
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
          placeholder="value"
        />
      );
  }
}

// ---------------------------------------------------------------------------
// Operand editor (fact_ref or literal)
// ---------------------------------------------------------------------------

type OperandKind = "fact_ref" | "literal";

interface OperandEditorProps {
  value: ExpressionOperand;
  facts: FactConstruct[];
  hintType?: BaseType | null;
  onChange: (operand: ExpressionOperand) => void;
  label: string;
}

function OperandEditor({
  value,
  facts,
  hintType,
  onChange,
  label,
}: OperandEditorProps) {
  const isFactRef = "fact_ref" in value;
  const kind: OperandKind = isFactRef ? "fact_ref" : "literal";

  function handleKindChange(k: OperandKind) {
    if (k === "fact_ref") {
      onChange({ fact_ref: facts[0]?.id ?? "fact" });
    } else {
      const type = hintType ?? { base: "Text" as const };
      onChange(defaultLiteral(type));
    }
  }

  return (
    <div className="flex items-center gap-1">
      <span className="w-8 text-xs text-gray-500">{label}</span>
      <select
        value={kind}
        onChange={(e) => handleKindChange(e.target.value as OperandKind)}
        className="rounded border border-gray-300 px-1 py-0.5 text-xs"
      >
        <option value="fact_ref">fact</option>
        <option value="literal">value</option>
      </select>

      {kind === "fact_ref" && (
        <select
          value={(value as FactRefOperand).fact_ref}
          onChange={(e) => onChange({ fact_ref: e.target.value })}
          className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
        >
          {facts.length === 0 && <option value="">— no facts —</option>}
          {facts.map((f) => (
            <option key={f.id} value={f.id}>
              {f.id}
            </option>
          ))}
        </select>
      )}

      {kind === "literal" && (
        <LiteralInput
          type={hintType ?? null}
          value={(value as LiteralOperand).literal}
          onChange={(v) =>
            onChange({
              literal: v,
              type: hintType ?? { base: "Text" },
            } as LiteralOperand)
          }
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// ExprTypeSelector — dropdown to change expression type with "Replace" button
// ---------------------------------------------------------------------------

interface ExprTypeSelectorProps {
  current: ExprType;
  facts: FactConstruct[];
  verdicts: string[];
  onReplace: (expr: PredicateExpression) => void;
}

function ExprTypeSelector({
  current,
  facts,
  verdicts,
  onReplace,
}: ExprTypeSelectorProps) {
  const [selected, setSelected] = useState<ExprType>(current);

  const types: { value: ExprType; label: string }[] = [
    { value: "Compare", label: "Compare" },
    { value: "And", label: "And" },
    { value: "Or", label: "Or" },
    { value: "Not", label: "Not" },
    { value: "VerdictPresent", label: "Verdict Present" },
    { value: "ForAll", label: "For All" },
    { value: "Exists", label: "Exists" },
  ];

  return (
    <div className="flex items-center gap-1">
      <select
        value={selected}
        onChange={(e) => setSelected(e.target.value as ExprType)}
        className="rounded border border-gray-300 px-1 py-0.5 text-xs"
      >
        {types.map((t) => (
          <option key={t.value} value={t.value}>
            {t.label}
          </option>
        ))}
      </select>
      {selected !== current && (
        <button
          onClick={() => onReplace(defaultExpr(selected, facts, verdicts))}
          className="rounded bg-orange-100 px-1.5 py-0.5 text-xs text-orange-700 hover:bg-orange-200"
        >
          Replace
        </button>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// ExpressionNode — recursive renderer for a single expression
// ---------------------------------------------------------------------------

interface ExpressionNodeProps {
  expr: PredicateExpression;
  facts: FactConstruct[];
  verdicts: string[];
  depth: number;
  onChange: (expr: PredicateExpression) => void;
  onDelete?: () => void;
}

const DEPTH_COLORS = [
  "border-blue-300 bg-blue-50",
  "border-green-300 bg-green-50",
  "border-purple-300 bg-purple-50",
  "border-orange-300 bg-orange-50",
  "border-pink-300 bg-pink-50",
];

function depthColor(depth: number): string {
  return DEPTH_COLORS[depth % DEPTH_COLORS.length];
}

function ExpressionNode({
  expr,
  facts,
  verdicts,
  depth,
  onChange,
  onDelete,
}: ExpressionNodeProps) {
  const type = exprType(expr);
  const colorClass = depthColor(depth);

  const header = (
    <div className="mb-1.5 flex items-center gap-2">
      <ExprTypeSelector
        current={type}
        facts={facts}
        verdicts={verdicts}
        onReplace={onChange}
      />
      <span className="flex-1" />
      {onDelete && (
        <button
          onClick={onDelete}
          className="rounded px-1 py-0.5 text-xs text-red-400 hover:bg-red-100 hover:text-red-600"
          title="Delete expression"
        >
          ×
        </button>
      )}
    </div>
  );

  // ---- Compare ----
  if (type === "Compare") {
    const cmp = expr as CompareExpr;
    const leftFactId = "fact_ref" in cmp.left ? cmp.left.fact_ref : null;
    const leftType = leftFactId ? factType(facts, leftFactId) : null;
    const ops = comparisonOpsForType(leftType);

    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="space-y-1">
          <OperandEditor
            value={cmp.left}
            facts={facts}
            onChange={(left) => onChange({ ...cmp, left })}
            label="left"
          />
          <div className="flex items-center gap-1">
            <span className="w-8 text-xs text-gray-500">op</span>
            <select
              value={cmp.op}
              onChange={(e) =>
                onChange({ ...cmp, op: e.target.value as CompareExpr["op"] })
              }
              className="rounded border border-gray-300 px-2 py-1 text-sm"
            >
              {ops.map((op) => (
                <option key={op} value={op}>
                  {op}
                </option>
              ))}
            </select>
          </div>
          <OperandEditor
            value={cmp.right}
            facts={facts}
            hintType={leftType}
            onChange={(right) => onChange({ ...cmp, right })}
            label="right"
          />
        </div>
      </div>
    );
  }

  // ---- And ----
  if (type === "And") {
    const and = expr as AndExpr;
    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="space-y-1.5">
          <ExpressionNode
            expr={and.left as PredicateExpression}
            facts={facts}
            verdicts={verdicts}
            depth={depth + 1}
            onChange={(left) => onChange({ ...and, left })}
          />
          <div className="flex items-center">
            <span className="rounded bg-blue-200 px-2 py-0.5 text-xs font-bold text-blue-700">
              AND
            </span>
          </div>
          <ExpressionNode
            expr={and.right as PredicateExpression}
            facts={facts}
            verdicts={verdicts}
            depth={depth + 1}
            onChange={(right) => onChange({ ...and, right })}
          />
        </div>
      </div>
    );
  }

  // ---- Or ----
  if (type === "Or") {
    const or = expr as OrExpr;
    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="space-y-1.5">
          <ExpressionNode
            expr={or.left as PredicateExpression}
            facts={facts}
            verdicts={verdicts}
            depth={depth + 1}
            onChange={(left) => onChange({ ...or, left })}
          />
          <div className="flex items-center">
            <span className="rounded bg-green-200 px-2 py-0.5 text-xs font-bold text-green-700">
              OR
            </span>
          </div>
          <ExpressionNode
            expr={or.right as PredicateExpression}
            facts={facts}
            verdicts={verdicts}
            depth={depth + 1}
            onChange={(right) => onChange({ ...or, right })}
          />
        </div>
      </div>
    );
  }

  // ---- Not ----
  if (type === "Not") {
    const not = expr as NotExpr;
    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="flex items-start gap-2">
          <span className="rounded bg-red-200 px-2 py-0.5 text-xs font-bold text-red-700">
            NOT
          </span>
          <div className="flex-1">
            <ExpressionNode
              expr={not.operand as PredicateExpression}
              facts={facts}
              verdicts={verdicts}
              depth={depth + 1}
              onChange={(operand) => onChange({ ...not, operand })}
            />
          </div>
        </div>
      </div>
    );
  }

  // ---- VerdictPresent ----
  if (type === "VerdictPresent") {
    const vp = expr as VerdictPresentExpr;
    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="flex items-center gap-2">
          <span className="rounded bg-purple-100 px-2 py-0.5 text-xs font-semibold text-purple-700">
            verdict_present
          </span>
          <select
            value={vp.verdict_present}
            onChange={(e) => onChange({ verdict_present: e.target.value })}
            className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
          >
            {verdicts.length === 0 && (
              <option value="">— no verdicts available —</option>
            )}
            {verdicts.map((v) => (
              <option key={v} value={v}>
                {v}
              </option>
            ))}
          </select>
          {verdicts.length === 0 && (
            <input
              type="text"
              value={vp.verdict_present}
              onChange={(e) => onChange({ verdict_present: e.target.value })}
              className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
              placeholder="verdict_id"
            />
          )}
        </div>
      </div>
    );
  }

  // ---- ForAll ----
  if (type === "ForAll") {
    const fa = expr as ForallExpr;
    const listFacts = facts.filter((f) => f.type.base === "List");
    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="space-y-1.5">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-gray-600">
              For all
            </span>
            <input
              type="text"
              value={fa.variable}
              onChange={(e) => onChange({ ...fa, variable: e.target.value })}
              className="w-20 rounded border border-gray-300 px-2 py-1 text-sm font-mono"
              placeholder="x"
            />
            <span className="text-xs text-gray-600">in</span>
            <select
              value={fa.domain.fact_ref}
              onChange={(e) =>
                onChange({ ...fa, domain: { fact_ref: e.target.value } })
              }
              className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
            >
              {listFacts.length === 0 && (
                <option value="">— no List facts —</option>
              )}
              {(listFacts.length > 0 ? listFacts : facts).map((f) => (
                <option key={f.id} value={f.id}>
                  {f.id}
                </option>
              ))}
            </select>
          </div>
          <div className="pl-4">
            <span className="text-xs font-semibold text-gray-600">body:</span>
            <div className="mt-1">
              <ExpressionNode
                expr={fa.body}
                facts={facts}
                verdicts={verdicts}
                depth={depth + 1}
                onChange={(body) => onChange({ ...fa, body })}
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  // ---- Exists ----
  if (type === "Exists") {
    const ex = expr as ExistsExpr;
    const listFacts = facts.filter((f) => f.type.base === "List");
    return (
      <div className={`rounded border p-2 ${colorClass}`}>
        {header}
        <div className="space-y-1.5">
          <div className="flex items-center gap-2">
            <span className="text-xs font-semibold text-gray-600">
              Exists
            </span>
            <input
              type="text"
              value={ex.variable}
              onChange={(e) => onChange({ ...ex, variable: e.target.value })}
              className="w-20 rounded border border-gray-300 px-2 py-1 text-sm font-mono"
              placeholder="x"
            />
            <span className="text-xs text-gray-600">in</span>
            <select
              value={ex.domain.fact_ref}
              onChange={(e) =>
                onChange({ ...ex, domain: { fact_ref: e.target.value } })
              }
              className="flex-1 rounded border border-gray-300 px-2 py-1 text-sm"
            >
              {listFacts.length === 0 && (
                <option value="">— no List facts —</option>
              )}
              {(listFacts.length > 0 ? listFacts : facts).map((f) => (
                <option key={f.id} value={f.id}>
                  {f.id}
                </option>
              ))}
            </select>
          </div>
          <div className="pl-4">
            <span className="text-xs font-semibold text-gray-600">body:</span>
            <div className="mt-1">
              <ExpressionNode
                expr={ex.body}
                facts={facts}
                verdicts={verdicts}
                depth={depth + 1}
                onChange={(body) => onChange({ ...ex, body })}
              />
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Fallback
  return (
    <div className={`rounded border p-2 ${colorClass}`}>
      {header}
      <p className="text-xs text-gray-400">Unknown expression type</p>
    </div>
  );
}

// ---------------------------------------------------------------------------
// PredicateBuilder (main export)
// ---------------------------------------------------------------------------

export function PredicateBuilder({
  value,
  onChange,
  availableFacts,
  availableVerdicts,
  mode: _mode,
}: PredicateBuilderProps) {
  if (value === null) {
    return (
      <div className="rounded border-2 border-dashed border-gray-200 p-4 text-center">
        <p className="mb-2 text-xs text-gray-500">No condition — always true</p>
        <ExprTypeDropdown
          facts={availableFacts}
          verdicts={availableVerdicts}
          onAdd={onChange}
        />
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <ExpressionNode
        expr={value}
        facts={availableFacts}
        verdicts={availableVerdicts}
        depth={0}
        onChange={onChange}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// ExprTypeDropdown — "Add expression" button+dropdown
// ---------------------------------------------------------------------------

interface ExprTypeDropdownProps {
  facts: FactConstruct[];
  verdicts: string[];
  onAdd: (expr: PredicateExpression) => void;
}

export function ExprTypeDropdown({
  facts,
  verdicts,
  onAdd,
}: ExprTypeDropdownProps) {
  const [open, setOpen] = useState(false);

  const options: { type: ExprType; label: string }[] = [
    { type: "Compare", label: "Comparison (fact op value)" },
    { type: "And", label: "And (both must be true)" },
    { type: "Or", label: "Or (either can be true)" },
    { type: "Not", label: "Not (negation)" },
    { type: "VerdictPresent", label: "Verdict present" },
    { type: "ForAll", label: "For all (universal quantifier)" },
    { type: "Exists", label: "Exists (existential quantifier)" },
  ];

  return (
    <div className="relative inline-block">
      <button
        onClick={() => setOpen((o) => !o)}
        className="rounded border border-dashed border-gray-300 px-3 py-1 text-xs text-gray-500 hover:bg-gray-50"
      >
        + Add expression
      </button>
      {open && (
        <div className="absolute left-0 top-full z-10 mt-1 w-56 rounded border border-gray-200 bg-white shadow-lg">
          {options.map(({ type, label }) => (
            <button
              key={type}
              onClick={() => {
                onAdd(defaultExpr(type, facts, verdicts));
                setOpen(false);
              }}
              className="block w-full px-3 py-1.5 text-left text-xs hover:bg-gray-50"
            >
              {label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
