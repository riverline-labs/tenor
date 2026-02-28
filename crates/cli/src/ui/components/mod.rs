//! React component generation functions for the tenor UI project.
//!
//! Each function generates a complete TypeScript/React component file as a String.
//! Components use inline styles with theme variables and import from ../types, ../api, ../theme.

mod action_space;
mod dashboard;
mod entity;
mod fact_input;
mod flow;
mod provenance;

// Re-export all component generators so callers use `components::emit_*` unchanged.
pub(super) use action_space::{emit_action_space, emit_blocked_actions};
pub(super) use dashboard::emit_dashboard;
pub(super) use entity::{emit_entity_detail, emit_entity_list, emit_instance_detail};
pub(super) use fact_input::emit_fact_input;
pub(super) use flow::{emit_flow_execution, emit_flow_history};
pub(super) use provenance::{emit_provenance_drill, emit_verdict_display};
