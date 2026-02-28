/**
 * TypeScript type definitions mirroring the Tenor interchange JSON schema.
 * Reference: schema/interchange-schema.json
 */

// ---------------------------------------------------------------------------
// Bundle
// ---------------------------------------------------------------------------

export interface InterchangeBundle {
  constructs: InterchangeConstruct[];
  id: string;
  kind: "Bundle";
  tenor: string;
  tenor_version: string;
}

// ---------------------------------------------------------------------------
// Constructs (discriminated union on "kind")
// ---------------------------------------------------------------------------

export type InterchangeConstruct =
  | FactConstruct
  | EntityConstruct
  | RuleConstruct
  | OperationConstruct
  | FlowConstruct
  | PersonaConstruct
  | SourceConstruct
  | SystemConstruct;

// ---------------------------------------------------------------------------
// Provenance
// ---------------------------------------------------------------------------

export interface Provenance {
  file: string;
  line: number;
}

// ---------------------------------------------------------------------------
// Base Types
// ---------------------------------------------------------------------------

export type BaseType =
  | BoolType
  | IntType
  | DecimalType
  | MoneyType
  | TextType
  | DateType
  | DateTimeType
  | DurationType
  | EnumType
  | ListType
  | RecordType
  | TaggedUnionType;

export interface BoolType {
  base: "Bool";
}

export interface IntType {
  base: "Int";
  min?: number;
  max?: number;
}

export interface DecimalType {
  base: "Decimal";
  precision: number;
  scale: number;
}

export interface MoneyType {
  base: "Money";
  currency: string;
}

export interface TextType {
  base: "Text";
  max_length?: number;
}

export interface DateType {
  base: "Date";
}

export interface DateTimeType {
  base: "DateTime";
}

export interface DurationType {
  base: "Duration";
  unit?: string;
  min?: number;
  max?: number;
}

export interface EnumType {
  base: "Enum";
  values: string[];
}

export interface ListType {
  base: "List";
  element_type: BaseType;
  max?: number;
}

export interface RecordType {
  base: "Record";
  fields: Record<string, BaseType>;
}

export interface TaggedUnionType {
  base: "TaggedUnion";
  variants: Record<string, BaseType>;
}

// ---------------------------------------------------------------------------
// Fact values (defaults)
// ---------------------------------------------------------------------------

export interface DecimalValue {
  kind: "decimal_value";
  precision: number;
  scale: number;
  value: string;
}

export interface MoneyValue {
  amount: DecimalValue;
  currency: string;
  kind: "money_value";
}

export interface BoolLiteral {
  kind: "bool_literal";
  value: boolean;
}

export type FactDefault =
  | DecimalValue
  | MoneyValue
  | BoolLiteral
  | boolean
  | number
  | string;

// ---------------------------------------------------------------------------
// Fact Source
// ---------------------------------------------------------------------------

export interface FreetextSource {
  field: string;
  system: string;
}

export interface StructuredSource {
  path: string;
  source_id: string;
}

export type FactSource = FreetextSource | StructuredSource;

// ---------------------------------------------------------------------------
// Fact
// ---------------------------------------------------------------------------

export interface FactConstruct {
  default?: FactDefault;
  id: string;
  kind: "Fact";
  provenance: Provenance;
  source?: FactSource;
  tenor: string;
  type: BaseType;
}

// ---------------------------------------------------------------------------
// Entity
// ---------------------------------------------------------------------------

export interface Transition {
  from: string;
  to: string;
}

export interface EntityConstruct {
  id: string;
  initial: string;
  kind: "Entity";
  parent?: string;
  provenance: Provenance;
  states: string[];
  tenor: string;
  transitions: Transition[];
}

// ---------------------------------------------------------------------------
// Predicate Expressions
// ---------------------------------------------------------------------------

export type PredicateExpression =
  | CompareExpr
  | AndExpr
  | OrExpr
  | NotExpr
  | ForallExpr
  | ExistsExpr
  | VerdictPresentExpr;

export type ExpressionOperand =
  | FactRefOperand
  | LiteralOperand
  | FieldRefOperand
  | VerdictPresentExpr
  | MulExpr
  | CompareExpr
  | AndExpr
  | OrExpr
  | NotExpr
  | ForallExpr
  | ExistsExpr;

export interface FactRefOperand {
  fact_ref: string;
}

export interface LiteralOperand {
  literal: boolean | number | string | object;
  type: BaseType;
}

export interface FieldRefOperand {
  field_ref: {
    field: string;
    var: string;
  };
}

export interface VerdictPresentExpr {
  verdict_present: string;
}

export interface MulExpr {
  left: FactRefOperand;
  literal: number;
  op: "*";
  result_type: BaseType;
}

export interface CompareExpr {
  comparison_type?: BaseType;
  left: ExpressionOperand;
  op: "=" | "!=" | "<" | "<=" | ">" | ">=";
  right: ExpressionOperand;
}

export interface AndExpr {
  left: ExpressionOperand;
  op: "and";
  right: ExpressionOperand;
}

export interface OrExpr {
  left: ExpressionOperand;
  op: "or";
  right: ExpressionOperand;
}

export interface NotExpr {
  op: "not";
  operand: ExpressionOperand;
}

export interface ForallExpr {
  body: PredicateExpression;
  domain: FactRefOperand;
  quantifier: "forall";
  variable: string;
  variable_type: BaseType;
}

export interface ExistsExpr {
  body: PredicateExpression;
  domain: FactRefOperand;
  quantifier: "exists";
  variable: string;
  variable_type: BaseType;
}

// ---------------------------------------------------------------------------
// Rule
// ---------------------------------------------------------------------------

export interface ProduceClause {
  payload: {
    type: BaseType;
    value: boolean | number | string | MulExpr;
  };
  verdict_type: string;
}

export interface RuleBody {
  produce: ProduceClause;
  when: PredicateExpression;
}

export interface RuleConstruct {
  body: RuleBody;
  id: string;
  kind: "Rule";
  provenance: Provenance;
  stratum: number;
  tenor: string;
}

// ---------------------------------------------------------------------------
// Operation
// ---------------------------------------------------------------------------

export interface Effect {
  entity_id: string;
  from: string;
  outcome?: string;
  to: string;
}

export interface OperationConstruct {
  allowed_personas: string[];
  effects: Effect[];
  error_contract: string[];
  id: string;
  kind: "Operation";
  outcomes?: string[];
  precondition: PredicateExpression;
  provenance: Provenance;
  tenor: string;
}

// ---------------------------------------------------------------------------
// Flow Steps
// ---------------------------------------------------------------------------

export interface TerminalTarget {
  kind: "Terminal";
  outcome: string;
}

export type StepTarget = string | TerminalTarget;

export interface CompensationStep {
  on_failure: TerminalTarget;
  op: string;
  persona: string;
}

export interface TerminateHandler {
  kind: "Terminate";
  outcome: string;
}

export interface CompensateHandler {
  kind: "Compensate";
  steps: CompensationStep[];
  then: TerminalTarget;
}

export interface EscalateHandler {
  kind: "Escalate";
  next: string;
  to_persona: string;
}

export type FailureHandler = TerminateHandler | CompensateHandler | EscalateHandler;

export interface OperationStep {
  id: string;
  kind: "OperationStep";
  on_failure: FailureHandler;
  op: string;
  outcomes: Record<string, StepTarget>;
  persona: string;
}

export interface BranchStep {
  condition: PredicateExpression;
  id: string;
  if_false: StepTarget;
  if_true: StepTarget;
  kind: "BranchStep";
  persona: string;
}

export interface HandoffStep {
  from_persona: string;
  id: string;
  kind: "HandoffStep";
  next: string;
  to_persona: string;
}

export interface SubFlowStep {
  flow: string;
  id: string;
  kind: "SubFlowStep";
  on_failure: FailureHandler;
  on_success: StepTarget;
  persona: string;
}

export interface ParallelBranch {
  entry: string;
  id: string;
  steps: FlowStep[];
}

export interface JoinPolicy {
  on_all_complete?: StepTarget;
  on_all_success: StepTarget;
  on_any_failure: FailureHandler;
}

export interface ParallelStep {
  branches: ParallelBranch[];
  id: string;
  join: JoinPolicy;
  kind: "ParallelStep";
}

export type FlowStep =
  | OperationStep
  | BranchStep
  | HandoffStep
  | SubFlowStep
  | ParallelStep;

// ---------------------------------------------------------------------------
// Flow
// ---------------------------------------------------------------------------

export interface FlowConstruct {
  entry: string;
  id: string;
  kind: "Flow";
  provenance: Provenance;
  snapshot?: "at_initiation";
  steps: FlowStep[];
  tenor: string;
}

// ---------------------------------------------------------------------------
// Persona
// ---------------------------------------------------------------------------

export interface PersonaConstruct {
  id: string;
  kind: "Persona";
  provenance: Provenance;
  tenor: string;
}

// ---------------------------------------------------------------------------
// Source
// ---------------------------------------------------------------------------

export interface SourceConstruct {
  description?: string;
  fields: Record<string, string>;
  id: string;
  kind: "Source";
  protocol: string;
  provenance: Provenance;
  tenor: string;
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

export interface SystemMember {
  id: string;
  path: string;
}

export interface SharedPersona {
  contracts: string[];
  persona: string;
}

export interface SystemTrigger {
  on: "success" | "failure" | "escalation";
  persona: string;
  source_contract: string;
  source_flow: string;
  target_contract: string;
  target_flow: string;
}

export interface SharedEntity {
  contracts: string[];
  entity: string;
}

export interface SystemConstruct {
  id: string;
  kind: "System";
  members: SystemMember[];
  provenance: Provenance;
  shared_entities: SharedEntity[];
  shared_personas: SharedPersona[];
  tenor: string;
  triggers: SystemTrigger[];
}

// ---------------------------------------------------------------------------
// Helpers — type guards
// ---------------------------------------------------------------------------

export function isFact(c: InterchangeConstruct): c is FactConstruct {
  return c.kind === "Fact";
}

export function isEntity(c: InterchangeConstruct): c is EntityConstruct {
  return c.kind === "Entity";
}

export function isRule(c: InterchangeConstruct): c is RuleConstruct {
  return c.kind === "Rule";
}

export function isOperation(c: InterchangeConstruct): c is OperationConstruct {
  return c.kind === "Operation";
}

export function isFlow(c: InterchangeConstruct): c is FlowConstruct {
  return c.kind === "Flow";
}

export function isPersona(c: InterchangeConstruct): c is PersonaConstruct {
  return c.kind === "Persona";
}

export function isSource(c: InterchangeConstruct): c is SourceConstruct {
  return c.kind === "Source";
}

export function isSystem(c: InterchangeConstruct): c is SystemConstruct {
  return c.kind === "System";
}
