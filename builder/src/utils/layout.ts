/**
 * Graph layout utilities for state machine and flow DAG rendering.
 *
 * Two layout algorithms:
 * 1. layoutStateMachine: force-directed layout for entity state diagrams
 * 2. layoutFlowDag: topological sort + left-to-right DAG layout for flows
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface LayoutNode {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  label?: string;
  type?: string;
}

export interface LayoutEdge {
  from: string;
  to: string;
  label?: string;
  controlPoints?: { x: number; y: number }[];
}

export interface LayoutResult {
  nodes: LayoutNode[];
  edges: LayoutEdge[];
  width: number;
  height: number;
}

// State machine node with extra fields
export interface StateMachineNode extends LayoutNode {
  isInitial: boolean;
}

// Flow node with extra fields
export interface FlowNode extends LayoutNode {
  kind: string;
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const NODE_WIDTH = 120;
const NODE_HEIGHT = 40;
const H_SPACING = 60; // horizontal spacing between nodes
const V_SPACING = 60; // vertical spacing between nodes
const MARGIN = 40;

// ---------------------------------------------------------------------------
// State machine layout (force-directed)
// ---------------------------------------------------------------------------

/**
 * Layout entity state machine nodes and edges.
 *
 * Uses a simple force-directed spring model:
 * - Repulsion between all nodes
 * - Attraction along edges
 * - Initial state anchored at top-left
 */
export function layoutStateMachine(
  states: string[],
  transitions: { from: string; to: string }[],
  initial: string
): LayoutResult & { nodes: StateMachineNode[] } {
  if (states.length === 0) {
    return { nodes: [], edges: [], width: 0, height: 0 };
  }

  // Initialize positions in a circle
  const n = states.length;
  const radius = Math.max(150, n * 40);
  const cx = MARGIN + radius;
  const cy = MARGIN + radius;

  const positions: Map<string, { x: number; y: number }> = new Map();

  for (let i = 0; i < n; i++) {
    const angle = (2 * Math.PI * i) / n - Math.PI / 2;
    positions.set(states[i], {
      x: cx + radius * Math.cos(angle),
      y: cy + radius * Math.sin(angle),
    });
  }

  // Force-directed iterations
  const ITERATIONS = 50;
  const REPULSION = 5000;
  const SPRING = 0.1;
  const SPRING_LENGTH = 150;

  for (let iter = 0; iter < ITERATIONS; iter++) {
    const forces: Map<string, { fx: number; fy: number }> = new Map();
    for (const s of states) forces.set(s, { fx: 0, fy: 0 });

    // Repulsion between all nodes
    for (let i = 0; i < states.length; i++) {
      for (let j = i + 1; j < states.length; j++) {
        const a = positions.get(states[i])!;
        const b = positions.get(states[j])!;
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.max(1, Math.sqrt(dx * dx + dy * dy));
        const force = REPULSION / (dist * dist);
        const fx = force * dx / dist;
        const fy = force * dy / dist;
        const fa = forces.get(states[i])!;
        const fb = forces.get(states[j])!;
        fa.fx -= fx;
        fa.fy -= fy;
        fb.fx += fx;
        fb.fy += fy;
      }
    }

    // Spring attraction along edges
    for (const t of transitions) {
      if (!positions.has(t.from) || !positions.has(t.to)) continue;
      const a = positions.get(t.from)!;
      const b = positions.get(t.to)!;
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.max(1, Math.sqrt(dx * dx + dy * dy));
      const force = SPRING * (dist - SPRING_LENGTH);
      const fx = force * dx / dist;
      const fy = force * dy / dist;
      const fa = forces.get(t.from)!;
      const fb = forces.get(t.to)!;
      fa.fx += fx;
      fa.fy += fy;
      fb.fx -= fx;
      fb.fy -= fy;
    }

    // Apply forces (except anchor initial state after first few iterations)
    for (const s of states) {
      if (iter > 5 && s === initial) continue;
      const pos = positions.get(s)!;
      const f = forces.get(s)!;
      pos.x += f.fx * 0.1;
      pos.y += f.fy * 0.1;
    }
  }

  // Anchor initial state at top-left
  const initialPos = positions.get(initial)!;
  const offsetX = MARGIN - initialPos.x + NODE_WIDTH / 2;
  const offsetY = MARGIN - initialPos.y + NODE_HEIGHT / 2;

  // Build nodes
  let maxX = 0;
  let maxY = 0;

  const nodes: StateMachineNode[] = states.map((state) => {
    const pos = positions.get(state)!;
    const x = pos.x + offsetX;
    const y = pos.y + offsetY;
    maxX = Math.max(maxX, x + NODE_WIDTH);
    maxY = Math.max(maxY, y + NODE_HEIGHT);
    return {
      id: state,
      x,
      y,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
      label: state,
      isInitial: state === initial,
    };
  });

  // Build edges
  const edges: LayoutEdge[] = transitions.map((t) => {
    const fromNode = nodes.find((n) => n.id === t.from)!;
    const toNode = nodes.find((n) => n.id === t.to)!;

    // Self-loop or curved edge
    if (t.from === t.to) {
      const cx2 = fromNode.x + NODE_WIDTH / 2;
      const cy2 = fromNode.y - 30;
      return {
        from: t.from,
        to: t.to,
        controlPoints: [
          { x: cx2 - 20, y: cy2 - 20 },
          { x: cx2 + 20, y: cy2 - 20 },
        ],
      };
    }

    // Straight edge (control points are midpoints)
    const mx = (fromNode.x + toNode.x) / 2 + NODE_WIDTH / 2;
    const my = (fromNode.y + toNode.y) / 2 + NODE_HEIGHT / 2;
    return {
      from: t.from,
      to: t.to,
      controlPoints: [{ x: mx, y: my }],
    };
  });

  return {
    nodes,
    edges,
    width: maxX + MARGIN,
    height: maxY + MARGIN,
  };
}

// ---------------------------------------------------------------------------
// Flow DAG layout (topological sort + layered)
// ---------------------------------------------------------------------------

interface FlowStepInfo {
  id: string;
  kind: string;
  nexts: string[]; // outgoing step IDs
}

/**
 * Layout flow DAG nodes and edges using topological sort + layered layout.
 * Entry step is at the left; terminal outcomes are at the right.
 */
export function layoutFlowDag(
  steps: FlowStepInfo[],
  entry: string
): LayoutResult & { nodes: FlowNode[] } {
  if (steps.length === 0) {
    return { nodes: [], edges: [], width: 0, height: 0 };
  }

  // Build adjacency map
  const stepMap = new Map<string, FlowStepInfo>();
  for (const s of steps) stepMap.set(s.id, s);

  // Topological sort via BFS (Kahn's algorithm)
  const inDegree = new Map<string, number>();
  for (const s of steps) inDegree.set(s.id, 0);

  for (const s of steps) {
    for (const next of s.nexts) {
      if (inDegree.has(next)) {
        inDegree.set(next, (inDegree.get(next) ?? 0) + 1);
      }
    }
  }

  // Assign layers (columns) via BFS from entry
  const layer = new Map<string, number>();
  const queue: string[] = [entry];
  layer.set(entry, 0);
  const visited = new Set<string>([entry]);

  while (queue.length > 0) {
    const cur = queue.shift()!;
    const info = stepMap.get(cur);
    if (!info) continue;
    const curLayer = layer.get(cur) ?? 0;

    for (const next of info.nexts) {
      if (!visited.has(next)) {
        visited.add(next);
        queue.push(next);
        layer.set(next, curLayer + 1);
      } else {
        // Push to later layer if needed
        const existingLayer = layer.get(next) ?? 0;
        if (curLayer + 1 > existingLayer) {
          layer.set(next, curLayer + 1);
        }
      }
    }
  }

  // Any unvisited steps (unreachable) get their own column
  let maxLayer = Math.max(...Array.from(layer.values()));
  for (const s of steps) {
    if (!layer.has(s.id)) {
      layer.set(s.id, ++maxLayer);
    }
  }

  // Group steps by layer
  const layerGroups = new Map<number, string[]>();
  for (const [id, l] of layer.entries()) {
    const group = layerGroups.get(l) ?? [];
    group.push(id);
    layerGroups.set(l, group);
  }

  // Assign positions
  const positions = new Map<string, { x: number; y: number }>();
  let maxX = 0;
  let maxY = 0;

  for (const [l, ids] of layerGroups.entries()) {
    const x = MARGIN + l * (NODE_WIDTH + H_SPACING);
    for (let i = 0; i < ids.length; i++) {
      const y = MARGIN + i * (NODE_HEIGHT + V_SPACING);
      positions.set(ids[i], { x, y });
      maxX = Math.max(maxX, x + NODE_WIDTH);
      maxY = Math.max(maxY, y + NODE_HEIGHT);
    }
  }

  // Build nodes
  const nodes: FlowNode[] = steps.map((s) => {
    const pos = positions.get(s.id) ?? { x: 0, y: 0 };
    return {
      id: s.id,
      x: pos.x,
      y: pos.y,
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
      label: s.id,
      kind: s.kind,
      type: s.kind,
    };
  });

  // Build edges
  const edges: LayoutEdge[] = [];
  for (const s of steps) {
    const fromNode = nodes.find((n) => n.id === s.id);
    if (!fromNode) continue;
    for (const next of s.nexts) {
      const toNode = nodes.find((n) => n.id === next);
      if (!toNode) continue;
      const mx = (fromNode.x + toNode.x) / 2 + NODE_WIDTH / 2;
      const my = (fromNode.y + toNode.y) / 2 + NODE_HEIGHT / 2;
      edges.push({
        from: s.id,
        to: next,
        controlPoints: [{ x: mx, y: my }],
      });
    }
  }

  return {
    nodes,
    edges,
    width: maxX + MARGIN,
    height: maxY + MARGIN,
  };
}

// ---------------------------------------------------------------------------
// Helper: extract step nexts from interchange FlowStep
// ---------------------------------------------------------------------------

import type { FlowStep, StepTarget } from "@/types/interchange";

function stepTargetIds(target: StepTarget): string[] {
  if (typeof target === "string") return [target];
  // Terminal target — no next step
  return [];
}

/**
 * Build FlowStepInfo array from interchange FlowStep array.
 * Extracts all outgoing step IDs from each step type.
 */
export function extractFlowStepInfos(steps: FlowStep[]): FlowStepInfo[] {
  return steps.map((step) => {
    const nexts: string[] = [];

    if (step.kind === "OperationStep") {
      for (const target of Object.values(step.outcomes)) {
        nexts.push(...stepTargetIds(target));
      }
    } else if (step.kind === "BranchStep") {
      nexts.push(...stepTargetIds(step.if_true));
      nexts.push(...stepTargetIds(step.if_false));
    } else if (step.kind === "HandoffStep") {
      nexts.push(step.next);
    } else if (step.kind === "SubFlowStep") {
      nexts.push(...stepTargetIds(step.on_success));
    } else if (step.kind === "ParallelStep") {
      nexts.push(...stepTargetIds(step.join.on_all_success));
    }

    // Filter out step IDs that don't exist in the steps array
    const stepIds = new Set(steps.map((s) => s.id));
    return {
      id: step.id,
      kind: step.kind,
      nexts: nexts.filter((id) => stepIds.has(id)),
    };
  });
}
