/**
 * DSL generator: converts interchange JSON to .tenor source.
 *
 * Follows canonical construct ordering from pass6_serialize.rs:
 * 1. TypeDecls (inline, extracted from Record/TaggedUnion fact types)
 * 2. Facts
 * 3. Entities
 * 4. Rules (ascending stratum)
 * 5. Operations
 * 6. Flows
 * 7. Personas
 * 8. Sources
 * 9. Systems
 *
 * Uses lowercase keywords per CLAUDE.md: fact, entity, rule, operation, flow, type, persona, source, system
 */

import type {
  InterchangeBundle,
  InterchangeConstruct,
  FactConstruct,
  EntityConstruct,
  RuleConstruct,
  OperationConstruct,
  FlowConstruct,
  PersonaConstruct,
  SourceConstruct,
  SystemConstruct,
  BaseType,
  PredicateExpression,
  ExpressionOperand,
  FlowStep,
  StepTarget,
  FailureHandler,
  CompensationStep,
  FactDefault,
  FactSource,
} from "@/types/interchange";

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/**
 * Generate valid .tenor DSL from an interchange bundle.
 */
export function generateDsl(bundle: InterchangeBundle): string {
  const sections: string[] = [];

  // Split constructs by kind
  const facts = bundle.constructs.filter(
    (c): c is FactConstruct => c.kind === "Fact"
  );
  const entities = bundle.constructs.filter(
    (c): c is EntityConstruct => c.kind === "Entity"
  );
  const rules = bundle.constructs
    .filter((c): c is RuleConstruct => c.kind === "Rule")
    .sort((a, b) => a.stratum - b.stratum);
  const operations = bundle.constructs.filter(
    (c): c is OperationConstruct => c.kind === "Operation"
  );
  const flows = bundle.constructs.filter(
    (c): c is FlowConstruct => c.kind === "Flow"
  );
  const personas = bundle.constructs.filter(
    (c): c is PersonaConstruct => c.kind === "Persona"
  );
  const sources = bundle.constructs.filter(
    (c): c is SourceConstruct => c.kind === "Source"
  );

  // Collect named types (Record/TaggedUnion used in fact types)
  const namedTypes = collectNamedTypes(facts);
  if (namedTypes.length > 0) {
    sections.push("// ── Named Types ─────────────────────────────────────────────────────────────\n");
    sections.push(namedTypes.map(([name, type]) => generateTypeDecl(name, type)).join("\n"));
  }

  // Personas first (before facts, per canonical ordering)
  if (personas.length > 0) {
    sections.push("// ── Personas ─────────────────────────────────────────────────────────────────\n");
    sections.push(personas.map(generatePersona).join("\n"));
  }

  // Sources
  if (sources.length > 0) {
    sections.push("// ── Sources ──────────────────────────────────────────────────────────────────\n");
    sections.push(sources.map(generateSource).join("\n"));
  }

  // Facts
  if (facts.length > 0) {
    sections.push("// ── Facts ────────────────────────────────────────────────────────────────────\n");
    sections.push(facts.map(generateFact).join("\n"));
  }

  // Entities
  if (entities.length > 0) {
    sections.push("// ── Entities ─────────────────────────────────────────────────────────────────\n");
    sections.push(entities.map(generateEntity).join("\n"));
  }

  // Rules (grouped by stratum)
  if (rules.length > 0) {
    const strata = [...new Set(rules.map((r) => r.stratum))].sort(
      (a, b) => a - b
    );
    for (const stratum of strata) {
      const strataRules = rules.filter((r) => r.stratum === stratum);
      sections.push(
        `// ── Rules — Stratum ${stratum} ────────────────────────────────────────────────────\n`
      );
      sections.push(strataRules.map(generateRule).join("\n"));
    }
  }

  // Operations
  if (operations.length > 0) {
    sections.push("// ── Operations ───────────────────────────────────────────────────────────────\n");
    sections.push(operations.map(generateOperation).join("\n"));
  }

  // Flows
  if (flows.length > 0) {
    sections.push("// ── Flows ────────────────────────────────────────────────────────────────────\n");
    sections.push(flows.map(generateFlow).join("\n"));
  }

  return sections.join("\n") + "\n";
}

// ---------------------------------------------------------------------------
// Named type collection
// ---------------------------------------------------------------------------

/**
 * Collect named types from fact types.
 * Returns (name, type) pairs for Record/TaggedUnion types that should be
 * declared as top-level `type` declarations.
 *
 * Note: The builder uses inline types in most cases. Named types are only
 * needed when the same Record/TaggedUnion is referenced by ID in the bundle.
 * For now, we emit inline types in fact declarations.
 */
function collectNamedTypes(
  _facts: FactConstruct[]
): [string, BaseType][] {
  // In the builder, types are always inline. Named types would come from
  // a "TypeDecl" construct kind, which is not in scope for Phase 9.
  return [];
}

// ---------------------------------------------------------------------------
// Type declaration
// ---------------------------------------------------------------------------

function generateTypeDecl(name: string, type: BaseType): string {
  if (type.base === "Record") {
    const fieldLines = Object.entries(type.fields)
      .map(([f, t]) => `  ${f}: ${formatType(t)}`)
      .join("\n");
    return `type ${name} {\n${fieldLines}\n}\n`;
  }
  return `type ${name} = ${formatType(type)}\n`;
}

// ---------------------------------------------------------------------------
// Type formatting
// ---------------------------------------------------------------------------

function formatType(type: BaseType): string {
  switch (type.base) {
    case "Bool":
      return "Bool";
    case "Int":
      if (type.min !== undefined && type.max !== undefined) {
        return `Int(min: ${type.min}, max: ${type.max})`;
      }
      if (type.min !== undefined) return `Int(min: ${type.min})`;
      if (type.max !== undefined) return `Int(max: ${type.max})`;
      return "Int";
    case "Decimal":
      return `Decimal(precision: ${type.precision}, scale: ${type.scale})`;
    case "Money":
      return `Money(currency: "${type.currency}")`;
    case "Text":
      if (type.max_length !== undefined) {
        return `Text(max_length: ${type.max_length})`;
      }
      return "Text";
    case "Date":
      return "Date";
    case "DateTime":
      return "DateTime";
    case "Duration":
      if (type.unit) {
        return `Duration(unit: "${type.unit}")`;
      }
      return "Duration";
    case "Enum":
      return `Enum(values: [${type.values.map((v) => `"${v}"`).join(", ")}])`;
    case "List": {
      const elemType = formatType(type.element_type);
      if (type.max !== undefined) {
        return `List(element_type: ${elemType}, max: ${type.max})`;
      }
      return `List(element_type: ${elemType})`;
    }
    case "Record": {
      const fields = Object.entries(type.fields)
        .map(([f, t]) => `${f}: ${formatType(t)}`)
        .join(", ");
      return `{${fields}}`;
    }
    case "TaggedUnion": {
      const variants = Object.entries(type.variants)
        .map(([tag, t]) => `${tag}: ${formatType(t)}`)
        .join(", ");
      return `TaggedUnion({${variants}})`;
    }
  }
}

// ---------------------------------------------------------------------------
// Persona
// ---------------------------------------------------------------------------

function generatePersona(persona: PersonaConstruct): string {
  return `persona ${persona.id}\n`;
}

// ---------------------------------------------------------------------------
// Source
// ---------------------------------------------------------------------------

function generateSource(source: SourceConstruct): string {
  const lines: string[] = [`source ${source.id} {`];
  lines.push(`  protocol: ${source.protocol}`);
  for (const [key, value] of Object.entries(source.fields)) {
    lines.push(`  ${key}: "${value}"`);
  }
  if (source.description) {
    lines.push(`  description: "${source.description}"`);
  }
  lines.push("}\n");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Fact
// ---------------------------------------------------------------------------

function generateFact(fact: FactConstruct): string {
  const lines: string[] = [`fact ${fact.id} {`];
  lines.push(`  type:   ${formatType(fact.type)}`);
  if (fact.source) {
    lines.push(`  source: ${formatFactSource(fact.source)}`);
  }
  if (fact.default !== undefined) {
    lines.push(`  default: ${formatFactDefault(fact.default, fact.type)}`);
  }
  lines.push("}\n");
  return lines.join("\n");
}

function formatFactSource(source: FactSource): string {
  if ("source_id" in source) {
    return `@${source.source_id}.${source.path}`;
  }
  return `"${source.system}.${source.field}"`;
}

function formatFactDefault(def: FactDefault, type: BaseType): string {
  if (typeof def === "boolean") return def.toString();
  if (typeof def === "number") return def.toString();
  if (typeof def === "string") return `"${def}"`;

  if (typeof def === "object" && def !== null) {
    if ("kind" in def) {
      if (def.kind === "bool_literal") return def.value.toString();
      if (def.kind === "money_value") {
        return `Money { amount: "${def.amount.value}", currency: "${def.currency}" }`;
      }
      if (def.kind === "decimal_value") {
        return `"${def.value}"`;
      }
    }
  }

  // Fallback: use type to determine format
  if (type.base === "Money" && typeof def === "object" && def !== null) {
    return JSON.stringify(def);
  }

  return JSON.stringify(def);
}

// ---------------------------------------------------------------------------
// Entity
// ---------------------------------------------------------------------------

function generateEntity(entity: EntityConstruct): string {
  const lines: string[] = [`entity ${entity.id} {`];
  lines.push(`  states:  [${entity.states.join(", ")}]`);
  lines.push(`  initial: ${entity.initial}`);
  if (entity.transitions.length > 0) {
    lines.push("  transitions: [");
    for (const t of entity.transitions) {
      lines.push(`    (${t.from}, ${t.to}),`);
    }
    lines.push("  ]");
  }
  lines.push("}\n");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Predicate expression formatting
// ---------------------------------------------------------------------------

function formatPredicate(expr: PredicateExpression, indent = 0): string {
  const pad = "  ".repeat(indent);
  return formatOperand(expr as ExpressionOperand, indent, pad);
}

function formatOperand(
  expr: ExpressionOperand,
  indent = 0,
  pad = ""
): string {
  if ("fact_ref" in expr) return expr.fact_ref;
  if ("verdict_present" in expr) return `verdict_present(${expr.verdict_present})`;
  if ("field_ref" in expr)
    return `${expr.field_ref.var}.${expr.field_ref.field}`;

  // Check MulExpr before generic literal check (MulExpr has op:"*" + literal)
  if ("op" in expr && expr.op === "*") {
    // MulExpr: { left: FactRefOperand, literal: number, op: "*", result_type: BaseType }
    const mul = expr as unknown as { left: { fact_ref: string }; literal: number; op: "*" };
    return `${mul.left.fact_ref} * ${mul.literal}`;
  }

  if ("literal" in expr) {
    const lit = expr.literal;
    if (typeof lit === "boolean") return lit.toString();
    if (typeof lit === "number") return lit.toString();
    if (typeof lit === "string") return `"${lit}"`;
    return JSON.stringify(lit);
  }

  if ("op" in expr) {
    const op = expr.op;

    if (op === "not" && "operand" in expr) {
      const inner = formatOperand(expr.operand, indent, pad);
      // Use unicode negation for cleaner output
      return `¬${inner}`;
    }

    if (op === "and" && "left" in expr && "right" in expr) {
      const left = formatOperand(
        expr.left as ExpressionOperand,
        indent,
        pad
      );
      const right = formatOperand(
        expr.right as ExpressionOperand,
        indent + 1,
        "  ".repeat(indent + 1)
      );
      return `${left}\n${pad}       ∧ ${right}`;
    }

    if (op === "or" && "left" in expr && "right" in expr) {
      const left = formatOperand(
        expr.left as ExpressionOperand,
        indent,
        pad
      );
      const right = formatOperand(
        expr.right as ExpressionOperand,
        indent + 1,
        "  ".repeat(indent + 1)
      );
      return `${left}\n${pad}       ∨ ${right}`;
    }

    if (
      ["=", "!=", "<", "<=", ">", ">="].includes(op) &&
      "left" in expr &&
      "right" in expr
    ) {
      const left = formatOperand(
        expr.left as ExpressionOperand,
        indent,
        pad
      );
      const right = formatOperand(
        expr.right as ExpressionOperand,
        indent,
        pad
      );
      return `${left} ${op} ${right}`;
    }
  }

  // Quantifiers
  if ("quantifier" in expr) {
    const q = expr.quantifier === "forall" ? "∀" : "∃";
    const body = formatOperand(
      expr.body as ExpressionOperand,
      indent,
      pad
    );
    return `${q} ${expr.variable} ∈ ${expr.domain.fact_ref} . ${body}`;
  }

  return JSON.stringify(expr);
}

// ---------------------------------------------------------------------------
// Rule
// ---------------------------------------------------------------------------

function generateRule(rule: RuleConstruct): string {
  const lines: string[] = [`rule ${rule.id} {`];
  lines.push(`  stratum: ${rule.stratum}`);
  lines.push(
    `  when:    ${formatPredicate(rule.body.when, 1)}`
  );
  const produce = rule.body.produce;
  const payloadValue = formatPayloadValue(produce.payload.value, produce.payload.type);
  lines.push(`  produce: verdict ${produce.verdict_type} { payload: ${formatType(produce.payload.type)} = ${payloadValue} }`);
  lines.push("}\n");
  return lines.join("\n");
}

function formatPayloadValue(
  value: boolean | number | string | { left: { fact_ref: string }; literal: number; op: "*"; result_type: BaseType },
  _type: BaseType
): string {
  if (typeof value === "boolean") return value.toString();
  if (typeof value === "number") return value.toString();
  if (typeof value === "string") return `"${value}"`;
  // MulExpr — op is always "*" here (not a comparison op)
  if (typeof value === "object" && value !== null) {
    const obj = value as { left?: { fact_ref: string }; literal?: number; op?: string };
    if (obj.op === "*" && obj.left && obj.literal !== undefined) {
      return `${obj.left.fact_ref} * ${obj.literal}`;
    }
  }
  return JSON.stringify(value);
}

// ---------------------------------------------------------------------------
// Operation
// ---------------------------------------------------------------------------

function generateOperation(op: OperationConstruct): string {
  const lines: string[] = [`operation ${op.id} {`];
  lines.push(
    `  allowed_personas: [${op.allowed_personas.join(", ")}]`
  );
  lines.push(
    `  precondition:     ${formatPredicate(op.precondition, 1)}`
  );
  if (op.effects.length > 0) {
    const effectParts = op.effects.map(
      (e) => `(${e.entity_id}, ${e.from}, ${e.to})`
    );
    lines.push(`  effects:          [${effectParts.join(", ")}]`);
  } else {
    lines.push("  effects:          []");
  }
  if (op.error_contract.length > 0) {
    lines.push(
      `  error_contract:   [${op.error_contract.join(", ")}]`
    );
  }
  lines.push("}\n");
  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Flow
// ---------------------------------------------------------------------------

function generateFlow(flow: FlowConstruct): string {
  const lines: string[] = [`flow ${flow.id} {`];
  if (flow.snapshot) {
    lines.push(`  snapshot: ${flow.snapshot}`);
  }
  lines.push(`  entry:    ${flow.entry}`);
  lines.push("");
  lines.push("  steps: {");
  for (const step of flow.steps) {
    lines.push(generateFlowStep(step, "    "));
  }
  lines.push("  }");
  lines.push("}\n");
  return lines.join("\n");
}

function generateFlowStep(step: FlowStep, indent: string): string {
  switch (step.kind) {
    case "OperationStep":
      return generateOperationStep(step, indent);
    case "BranchStep":
      return generateBranchStep(step, indent);
    case "HandoffStep":
      return generateHandoffStep(step, indent);
    case "SubFlowStep":
      return generateSubFlowStep(step, indent);
    case "ParallelStep":
      return generateParallelStep(step, indent);
  }
}

function generateOperationStep(
  step: { id: string; kind: "OperationStep"; op: string; persona: string; outcomes: Record<string, StepTarget>; on_failure: FailureHandler },
  indent: string
): string {
  const lines: string[] = [`${indent}${step.id}: OperationStep {`];
  lines.push(`${indent}  op:      ${step.op}`);
  lines.push(`${indent}  persona: ${step.persona}`);

  // Outcomes
  const outcomeEntries = Object.entries(step.outcomes);
  if (outcomeEntries.length > 0) {
    lines.push(`${indent}  outcomes: {`);
    for (const [label, target] of outcomeEntries) {
      lines.push(`${indent}    ${label}: ${formatStepTarget(target)}`);
    }
    lines.push(`${indent}  }`);
  }

  // Failure handler
  lines.push(`${indent}  on_failure: ${formatFailureHandler(step.on_failure, indent + "  ")}`);
  lines.push(`${indent}}`);
  lines.push("");
  return lines.join("\n");
}

function generateBranchStep(
  step: { id: string; kind: "BranchStep"; condition: PredicateExpression; persona: string; if_true: StepTarget; if_false: StepTarget },
  indent: string
): string {
  const lines: string[] = [`${indent}${step.id}: BranchStep {`];
  lines.push(`${indent}  condition: ${formatPredicate(step.condition)}`);
  lines.push(`${indent}  persona:   ${step.persona}`);
  lines.push(`${indent}  if_true:   ${formatStepTarget(step.if_true)}`);
  lines.push(`${indent}  if_false:  ${formatStepTarget(step.if_false)}`);
  lines.push(`${indent}}`);
  lines.push("");
  return lines.join("\n");
}

function generateHandoffStep(
  step: { id: string; kind: "HandoffStep"; from_persona: string; to_persona: string; next: string },
  indent: string
): string {
  const lines: string[] = [`${indent}${step.id}: HandoffStep {`];
  lines.push(`${indent}  from_persona: ${step.from_persona}`);
  lines.push(`${indent}  to_persona:   ${step.to_persona}`);
  lines.push(`${indent}  next:         ${step.next}`);
  lines.push(`${indent}}`);
  lines.push("");
  return lines.join("\n");
}

function generateSubFlowStep(
  step: { id: string; kind: "SubFlowStep"; flow: string; persona: string; on_success: StepTarget; on_failure: FailureHandler },
  indent: string
): string {
  const lines: string[] = [`${indent}${step.id}: SubFlowStep {`];
  lines.push(`${indent}  flow:       ${step.flow}`);
  lines.push(`${indent}  persona:    ${step.persona}`);
  lines.push(`${indent}  on_success: ${formatStepTarget(step.on_success)}`);
  lines.push(`${indent}  on_failure: ${formatFailureHandler(step.on_failure, indent + "  ")}`);
  lines.push(`${indent}}`);
  lines.push("");
  return lines.join("\n");
}

function generateParallelStep(
  step: { id: string; kind: "ParallelStep"; branches: { id: string; entry: string; steps: FlowStep[] }[]; join: { on_all_success: StepTarget; on_any_failure: FailureHandler; on_all_complete?: StepTarget } },
  indent: string
): string {
  const lines: string[] = [`${indent}${step.id}: ParallelStep {`];
  lines.push(`${indent}  branches: [`);
  for (const branch of step.branches) {
    lines.push(`${indent}    ${branch.id}: {`);
    lines.push(`${indent}      entry: ${branch.entry}`);
    lines.push(`${indent}      steps: {`);
    for (const s of branch.steps) {
      lines.push(generateFlowStep(s, indent + "        "));
    }
    lines.push(`${indent}      }`);
    lines.push(`${indent}    }`);
  }
  lines.push(`${indent}  ]`);
  lines.push(`${indent}  join: {`);
  lines.push(
    `${indent}    on_all_success: ${formatStepTarget(step.join.on_all_success)}`
  );
  lines.push(
    `${indent}    on_any_failure: ${formatFailureHandler(step.join.on_any_failure, indent + "    ")}`
  );
  if (step.join.on_all_complete) {
    lines.push(
      `${indent}    on_all_complete: ${formatStepTarget(step.join.on_all_complete)}`
    );
  }
  lines.push(`${indent}  }`);
  lines.push(`${indent}}`);
  lines.push("");
  return lines.join("\n");
}

function formatStepTarget(target: StepTarget): string {
  if (typeof target === "string") return target;
  return `Terminal(${target.outcome})`;
}

function formatFailureHandler(handler: FailureHandler, _indent: string): string {
  if (handler.kind === "Terminate") {
    return `Terminate(outcome: ${handler.outcome})`;
  }
  if (handler.kind === "Escalate") {
    return `Escalate(to_persona: ${handler.to_persona}, next: ${handler.next})`;
  }
  // Compensate
  const stepsStr = handler.steps
    .map((s: CompensationStep) =>
      `{
          op:         ${s.op}
          persona:    ${s.persona}
          on_failure: ${formatFailureHandler(s.on_failure as unknown as FailureHandler, "          ")}
        }`
    )
    .join(", ");
  return `Compensate(
        steps: [${stepsStr}]
        then: Terminal(${handler.then.outcome})
      )`;
}
