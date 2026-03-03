package tenor

// FactSet maps fact IDs to their values. Values may be bool, float64, string,
// map[string]interface{}, or []interface{} depending on the fact type.
type FactSet map[string]interface{}

// EntityStateMap maps entity IDs to their current state (single-instance, old format).
// Use EntityStateMapNested for multi-instance contracts.
type EntityStateMap map[string]string

// EntityStateMapNested maps entity IDs to a map of instance_id -> state.
// This is the new multi-instance format.
type EntityStateMapNested map[string]map[string]string

// InstanceBindings maps entity IDs to instance IDs for flow execution.
type InstanceBindings map[string]string

// VerdictProvenance traces how a verdict was produced.
type VerdictProvenance struct {
	Rule         string   `json:"rule"`
	Stratum      int      `json:"stratum"`
	FactsUsed    []string `json:"facts_used"`
	VerdictsUsed []string `json:"verdicts_used"`
}

// Verdict represents a single evaluated verdict.
type Verdict struct {
	Type       string            `json:"type"`
	Payload    interface{}       `json:"payload"`
	Provenance VerdictProvenance `json:"provenance"`
}

// VerdictSet contains all verdicts produced by evaluation.
type VerdictSet struct {
	Verdicts []Verdict `json:"verdicts"`
}

// VerdictSummary is a compact verdict representation used in action spaces.
type VerdictSummary struct {
	VerdictType   string      `json:"verdict_type"`
	Payload       interface{} `json:"payload"`
	ProducingRule string      `json:"producing_rule"`
	Stratum       int         `json:"stratum"`
}

// EntitySummary describes an entity's current state in the action space.
type EntitySummary struct {
	EntityID            string   `json:"entity_id"`
	CurrentState        string   `json:"current_state"`
	PossibleTransitions []string `json:"possible_transitions"`
}

// Action represents an available action in the action space.
// InstanceBindings maps entity_id to the set of valid instance_ids for this action.
type Action struct {
	FlowID           string                       `json:"flow_id"`
	PersonaID        string                       `json:"persona_id"`
	EntryOperationID string                       `json:"entry_operation_id"`
	EnablingVerdicts []VerdictSummary             `json:"enabling_verdicts"`
	AffectedEntities []EntitySummary              `json:"affected_entities"`
	Description      string                       `json:"description"`
	InstanceBindings map[string][]string          `json:"instance_bindings,omitempty"`
}

// BlockedReason describes why an action is blocked.
// The Type field contains one of: PersonaNotAuthorized, PreconditionNotMet,
// EntityNotInSourceState, MissingFacts.
type BlockedReason struct {
	Type            string   `json:"type"`
	MissingVerdicts []string `json:"missing_verdicts,omitempty"`
	EntityID        string   `json:"entity_id,omitempty"`
	CurrentState    string   `json:"current_state,omitempty"`
	RequiredState   string   `json:"required_state,omitempty"`
	FactIDs         []string `json:"fact_ids,omitempty"`
}

// BlockedAction represents an action that exists but cannot currently be executed.
type BlockedAction struct {
	FlowID           string              `json:"flow_id"`
	Reason           BlockedReason       `json:"reason"`
	InstanceBindings map[string][]string `json:"instance_bindings"`
}

// ActionSpace is the complete set of available and blocked actions for a persona.
type ActionSpace struct {
	PersonaID       string           `json:"persona_id"`
	Actions         []Action         `json:"actions"`
	CurrentVerdicts []VerdictSummary `json:"current_verdicts"`
	BlockedActions  []BlockedAction  `json:"blocked_actions"`
}

// StepResult describes the result of a single flow step.
type StepResult struct {
	StepID           string            `json:"step_id"`
	StepType         string            `json:"step_type"`
	Result           string            `json:"result"`
	InstanceBindings map[string]string `json:"instance_bindings,omitempty"`
}

// EntityStateChange describes a state transition caused by flow execution.
type EntityStateChange struct {
	EntityID   string `json:"entity_id"`
	InstanceID string `json:"instance_id"`
	FromState  string `json:"from_state"`
	ToState    string `json:"to_state"`
}

// FlowResult contains the results of a flow simulation.
type FlowResult struct {
	Simulation       bool                `json:"simulation"`
	FlowID           string              `json:"flow_id"`
	Persona          string              `json:"persona"`
	Outcome          string              `json:"outcome"`
	Path             []StepResult        `json:"path"`
	WouldTransition  []EntityStateChange `json:"would_transition"`
	Verdicts         []Verdict           `json:"verdicts"`
	InstanceBindings InstanceBindings    `json:"instance_bindings"`
}
