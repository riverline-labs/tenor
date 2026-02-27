/**
 * FlowDag: SVG-based interactive flow DAG renderer.
 *
 * Renders all five flow step types as nodes with directed edges:
 * - OperationStep: rectangular card with operation name and persona, blue border
 * - BranchStep: diamond shape with condition summary, orange border
 * - ParallelStep: horizontal split node showing parallel branches
 * - SubFlowStep: double-bordered rectangle with sub-flow ID
 * - HandoffStep: arrow-shaped node showing from_persona -> to_persona
 * - Terminal nodes: rounded end caps labeled with outcome
 *
 * Supports pan/zoom, click to select, and drag to rearrange.
 */
import React, { useState, useRef, useCallback, useMemo } from "react";
import type { FlowStep, StepTarget, TerminalTarget } from "@/types/interchange";
import { layoutFlowDag, extractFlowStepInfos } from "@/utils/layout";
import type { FlowNode, LayoutEdge } from "@/utils/layout";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface FlowDagProps {
  steps: FlowStep[];
  entry: string;
  onStepClick?: (stepId: string) => void;
  onEdgeClick?: (from: string, to: string, label?: string) => void;
  editable?: boolean;
  highlightedStep?: string;
  direction?: "LR" | "TB";
}

interface TerminalNode {
  id: string;
  outcome: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

interface LabeledEdge extends LayoutEdge {
  label?: string;
  isFailure?: boolean;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const STEP_W = 160;
const STEP_H = 56;
const TERMINAL_W = 100;
const TERMINAL_H = 36;
const H_GAP = 80;
const V_GAP = 60;
const MARGIN = 50;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isTerminal(target: StepTarget): target is TerminalTarget {
  return typeof target === "object" && target !== null && "kind" in target && target.kind === "Terminal";
}

function stepTargetId(target: StepTarget): string | null {
  if (typeof target === "string") return target;
  return null;
}

function summaryCondition(cond: object): string {
  if ("op" in cond) {
    const op = (cond as { op: string }).op;
    if (op === "and") return "... AND ...";
    if (op === "or") return "... OR ...";
    if (op === "not") return "NOT ...";
    if (["=", "!=", "<", "<=", ">", ">="].includes(op)) {
      const c = cond as { left: { fact_ref?: string }; op: string };
      const factId = c.left?.fact_ref ?? "?";
      return `${factId} ${op} ...`;
    }
  }
  if ("verdict_present" in cond) {
    return `verdict: ${(cond as { verdict_present: string }).verdict_present}`;
  }
  if ("quantifier" in cond) {
    return `${(cond as { quantifier: string }).quantifier} ...`;
  }
  return "condition";
}

// ---------------------------------------------------------------------------
// Arrow marker definition (SVG defs)
// ---------------------------------------------------------------------------

function ArrowDefs() {
  return (
    <defs>
      <marker
        id="arrow-blue"
        markerWidth="8"
        markerHeight="6"
        refX="7"
        refY="3"
        orient="auto"
      >
        <polygon points="0 0, 8 3, 0 6" fill="#3b82f6" />
      </marker>
      <marker
        id="arrow-gray"
        markerWidth="8"
        markerHeight="6"
        refX="7"
        refY="3"
        orient="auto"
      >
        <polygon points="0 0, 8 3, 0 6" fill="#6b7280" />
      </marker>
      <marker
        id="arrow-red"
        markerWidth="8"
        markerHeight="6"
        refX="7"
        refY="3"
        orient="auto"
      >
        <polygon points="0 0, 8 3, 0 6" fill="#ef4444" />
      </marker>
    </defs>
  );
}

// ---------------------------------------------------------------------------
// Edge renderer
// ---------------------------------------------------------------------------

interface EdgeProps {
  edge: LabeledEdge;
  fromNode: { x: number; y: number; width: number; height: number } | undefined;
  toNode: { x: number; y: number; width: number; height: number } | undefined;
  onClick?: () => void;
}

function EdgePath({ edge, fromNode, toNode, onClick }: EdgeProps) {
  if (!fromNode || !toNode) return null;

  const x1 = fromNode.x + fromNode.width;
  const y1 = fromNode.y + fromNode.height / 2;
  const x2 = toNode.x;
  const y2 = toNode.y + toNode.height / 2;
  const mx = (x1 + x2) / 2;

  const d = `M ${x1} ${y1} C ${mx} ${y1}, ${mx} ${y2}, ${x2} ${y2}`;

  const color = edge.isFailure ? "#ef4444" : "#6b7280";
  const markerId = edge.isFailure ? "arrow-red" : "arrow-gray";
  const strokeDash = edge.isFailure ? "5,3" : undefined;

  return (
    <g className="cursor-pointer" onClick={onClick}>
      <path
        d={d}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeDasharray={strokeDash}
        markerEnd={`url(#${markerId})`}
        opacity={0.7}
      />
      {edge.label && (
        <text
          x={mx}
          y={(y1 + y2) / 2 - 4}
          textAnchor="middle"
          fontSize={10}
          fill={color}
          className="select-none pointer-events-none"
        >
          {edge.label}
        </text>
      )}
      {/* Invisible wider hit area */}
      <path d={d} fill="none" stroke="transparent" strokeWidth={8} />
    </g>
  );
}

// ---------------------------------------------------------------------------
// Step node renderers
// ---------------------------------------------------------------------------

interface StepNodeProps {
  node: FlowNode;
  step: FlowStep;
  isSelected: boolean;
  isHighlighted: boolean;
  isEntry: boolean;
  onClick: (e: React.MouseEvent) => void;
}

function OperationStepNode({ node, step, isSelected, isHighlighted, isEntry, onClick }: StepNodeProps) {
  const s = step as Extract<FlowStep, { kind: "OperationStep" }>;
  const borderColor = isSelected ? "#1d4ed8" : isHighlighted ? "#7c3aed" : "#3b82f6";
  const bgColor = isHighlighted ? "#ede9fe" : isSelected ? "#dbeafe" : "#eff6ff";
  const glow = isHighlighted ? "drop-shadow(0 0 6px #7c3aed)" : undefined;

  return (
    <g transform={`translate(${node.x}, ${node.y})`} onClick={onClick} className="cursor-pointer" style={{ filter: glow }}>
      {isEntry && (
        <>
          <circle cx={-16} cy={STEP_H / 2} r={6} fill="#1d4ed8" />
          <path d={`M -10 ${STEP_H / 2} L -2 ${STEP_H / 2}`} stroke="#1d4ed8" strokeWidth={1.5} markerEnd="url(#arrow-blue)" />
        </>
      )}
      <rect
        width={STEP_W}
        height={STEP_H}
        rx={6}
        ry={6}
        fill={bgColor}
        stroke={borderColor}
        strokeWidth={isSelected ? 2 : 1.5}
      />
      <text x={8} y={18} fontSize={10} fill="#374151" fontWeight={600} className="select-none">
        {s.op}
      </text>
      <text x={8} y={32} fontSize={9} fill="#6b7280" className="select-none">
        persona: {s.persona || "—"}
      </text>
      <text x={8} y={46} fontSize={9} fill="#9ca3af" className="select-none">
        {Object.keys(s.outcomes).join(", ") || "no outcomes"}
      </text>
    </g>
  );
}

function BranchStepNode({ node, step, isSelected, isHighlighted, isEntry, onClick }: StepNodeProps) {
  const s = step as Extract<FlowStep, { kind: "BranchStep" }>;
  const condText = summaryCondition(s.condition);
  const borderColor = isSelected ? "#c2410c" : "#f97316";
  const bgColor = isSelected ? "#ffedd5" : "#fff7ed";
  const hw = STEP_W / 2;
  const hh = STEP_H / 2;
  const glow = isHighlighted ? "drop-shadow(0 0 6px #7c3aed)" : undefined;

  // Diamond: center at (hw, hh)
  const points = `${hw},0 ${STEP_W},${hh} ${hw},${STEP_H} 0,${hh}`;

  return (
    <g transform={`translate(${node.x}, ${node.y})`} onClick={onClick} className="cursor-pointer" style={{ filter: glow }}>
      {isEntry && (
        <>
          <circle cx={-16} cy={hh} r={6} fill="#1d4ed8" />
          <path d={`M -10 ${hh} L -2 ${hh}`} stroke="#1d4ed8" strokeWidth={1.5} markerEnd="url(#arrow-blue)" />
        </>
      )}
      <polygon
        points={points}
        fill={bgColor}
        stroke={borderColor}
        strokeWidth={isSelected ? 2 : 1.5}
      />
      <text x={hw} y={hh - 6} fontSize={9} fill="#374151" textAnchor="middle" className="select-none">
        {condText.length > 18 ? condText.slice(0, 16) + "…" : condText}
      </text>
      <text x={hw} y={hh + 8} fontSize={8} fill="#6b7280" textAnchor="middle" className="select-none">
        T / F
      </text>
    </g>
  );
}

function HandoffStepNode({ node, step, isSelected, isHighlighted, isEntry, onClick }: StepNodeProps) {
  const s = step as Extract<FlowStep, { kind: "HandoffStep" }>;
  const borderColor = isSelected ? "#065f46" : "#10b981";
  const bgColor = isSelected ? "#d1fae5" : "#ecfdf5";
  const glow = isHighlighted ? "drop-shadow(0 0 6px #7c3aed)" : undefined;

  return (
    <g transform={`translate(${node.x}, ${node.y})`} onClick={onClick} className="cursor-pointer" style={{ filter: glow }}>
      {isEntry && (
        <>
          <circle cx={-16} cy={STEP_H / 2} r={6} fill="#1d4ed8" />
          <path d={`M -10 ${STEP_H / 2} L -2 ${STEP_H / 2}`} stroke="#1d4ed8" strokeWidth={1.5} markerEnd="url(#arrow-blue)" />
        </>
      )}
      <rect
        width={STEP_W}
        height={STEP_H}
        rx={STEP_H / 2}
        ry={STEP_H / 2}
        fill={bgColor}
        stroke={borderColor}
        strokeWidth={isSelected ? 2 : 1.5}
      />
      <text x={STEP_W / 2} y={STEP_H / 2 - 4} fontSize={10} fill="#374151" textAnchor="middle" fontWeight={600} className="select-none">
        handoff
      </text>
      <text x={STEP_W / 2} y={STEP_H / 2 + 10} fontSize={9} fill="#6b7280" textAnchor="middle" className="select-none">
        {s.from_persona} → {s.to_persona}
      </text>
    </g>
  );
}

function SubFlowStepNode({ node, step, isSelected, isHighlighted, isEntry, onClick }: StepNodeProps) {
  const s = step as Extract<FlowStep, { kind: "SubFlowStep" }>;
  const borderColor = isSelected ? "#7c3aed" : "#8b5cf6";
  const bgColor = isSelected ? "#ede9fe" : "#f5f3ff";
  const glow = isHighlighted ? "drop-shadow(0 0 6px #7c3aed)" : undefined;

  return (
    <g transform={`translate(${node.x}, ${node.y})`} onClick={onClick} className="cursor-pointer" style={{ filter: glow }}>
      {isEntry && (
        <>
          <circle cx={-16} cy={STEP_H / 2} r={6} fill="#1d4ed8" />
          <path d={`M -10 ${STEP_H / 2} L -2 ${STEP_H / 2}`} stroke="#1d4ed8" strokeWidth={1.5} markerEnd="url(#arrow-blue)" />
        </>
      )}
      {/* Double border */}
      <rect width={STEP_W} height={STEP_H} rx={4} ry={4} fill={bgColor} stroke={borderColor} strokeWidth={4} />
      <rect x={3} y={3} width={STEP_W - 6} height={STEP_H - 6} rx={3} ry={3} fill="none" stroke={borderColor} strokeWidth={1} />
      <text x={STEP_W / 2} y={STEP_H / 2 - 4} fontSize={10} fill="#374151" textAnchor="middle" fontWeight={600} className="select-none">
        sub-flow
      </text>
      <text x={STEP_W / 2} y={STEP_H / 2 + 10} fontSize={9} fill="#6b7280" textAnchor="middle" className="select-none">
        {s.flow}
      </text>
    </g>
  );
}

function ParallelStepNode({ node, step, isSelected, isHighlighted, isEntry, onClick }: StepNodeProps) {
  const s = step as Extract<FlowStep, { kind: "ParallelStep" }>;
  const borderColor = isSelected ? "#1e40af" : "#60a5fa";
  const bgColor = isSelected ? "#dbeafe" : "#eff6ff";
  const glow = isHighlighted ? "drop-shadow(0 0 6px #7c3aed)" : undefined;

  const branchCount = s.branches.length;
  const laneH = Math.max(STEP_H, branchCount * 18);
  const laneW = STEP_W + 20;

  return (
    <g transform={`translate(${node.x}, ${node.y - (laneH - STEP_H) / 2})`} onClick={onClick} className="cursor-pointer" style={{ filter: glow }}>
      {isEntry && (
        <>
          <circle cx={-16} cy={laneH / 2} r={6} fill="#1d4ed8" />
          <path d={`M -10 ${laneH / 2} L -2 ${laneH / 2}`} stroke="#1d4ed8" strokeWidth={1.5} markerEnd="url(#arrow-blue)" />
        </>
      )}
      <rect width={laneW} height={laneH} rx={4} ry={4} fill={bgColor} stroke={borderColor} strokeWidth={isSelected ? 2 : 1.5} />
      {/* Swim lane dividers */}
      {s.branches.map((branch, i) => (
        <g key={branch.id}>
          {i > 0 && (
            <line x1={0} y1={i * (laneH / branchCount)} x2={laneW} y2={i * (laneH / branchCount)} stroke={borderColor} strokeWidth={0.5} strokeDasharray="3,2" />
          )}
          <text x={6} y={(i + 0.5) * (laneH / branchCount) + 4} fontSize={9} fill="#374151" className="select-none">
            {branch.id}
          </text>
        </g>
      ))}
      <text x={laneW / 2} y={-8} fontSize={9} fill="#6b7280" textAnchor="middle" fontWeight={600} className="select-none">
        parallel ({branchCount})
      </text>
    </g>
  );
}

function TerminalStepNode({
  node,
  onClick,
}: {
  node: TerminalNode;
  onClick?: () => void;
}) {
  const isSuccess = node.outcome === "success" || node.outcome === "completed";
  const borderColor = isSuccess ? "#16a34a" : "#dc2626";
  const bgColor = isSuccess ? "#dcfce7" : "#fee2e2";
  const textColor = isSuccess ? "#15803d" : "#b91c1c";

  return (
    <g transform={`translate(${node.x}, ${node.y})`} onClick={onClick} className={onClick ? "cursor-pointer" : undefined}>
      <rect
        width={TERMINAL_W}
        height={TERMINAL_H}
        rx={TERMINAL_H / 2}
        ry={TERMINAL_H / 2}
        fill={bgColor}
        stroke={borderColor}
        strokeWidth={1.5}
      />
      <text
        x={TERMINAL_W / 2}
        y={TERMINAL_H / 2 + 4}
        textAnchor="middle"
        fontSize={10}
        fill={textColor}
        fontWeight={600}
        className="select-none"
      >
        {node.outcome}
      </text>
    </g>
  );
}

// ---------------------------------------------------------------------------
// Main FlowDag component
// ---------------------------------------------------------------------------

export function FlowDag({
  steps,
  entry,
  onStepClick,
  onEdgeClick,
  editable = false,
  highlightedStep,
  direction: _direction = "LR",
}: FlowDagProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [selectedStep, setSelectedStep] = useState<string | null>(null);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [zoom, setZoom] = useState(1);
  const [isPanning, setIsPanning] = useState(false);
  const panStart = useRef<{ x: number; y: number; px: number; py: number } | null>(null);
  const [nodePosOverrides, setNodePosOverrides] = useState<Map<string, { x: number; y: number }>>(new Map());
  const draggingNode = useRef<{ id: string; ox: number; oy: number } | null>(null);

  // Compute layout
  const stepInfos = useMemo(() => extractFlowStepInfos(steps), [steps]);
  const layout = useMemo(
    () => (steps.length > 0 && entry ? layoutFlowDag(stepInfos, entry) : { nodes: [], edges: [], width: 400, height: 200 }),
    [stepInfos, entry, steps.length]
  );

  // Build node map with position overrides applied
  const nodeMap = useMemo(() => {
    const map = new Map<string, { x: number; y: number; width: number; height: number; kind: string }>();
    for (const node of layout.nodes) {
      const override = nodePosOverrides.get(node.id);
      map.set(node.id, {
        x: override?.x ?? node.x,
        y: override?.y ?? node.y,
        width: STEP_W,
        height: STEP_H,
        kind: node.kind,
      });
    }
    return map;
  }, [layout.nodes, nodePosOverrides]);

  // Compute terminal nodes from step targets
  const terminalNodes = useMemo(() => {
    const terminals = new Map<string, { outcome: string; sources: string[] }>();
    const rightmostX = Math.max(...Array.from(nodeMap.values()).map((n) => n.x + n.width), 0);
    const terminalX = rightmostX + H_GAP;

    for (const step of steps) {
      if (step.kind === "OperationStep") {
        for (const [label, target] of Object.entries(step.outcomes)) {
          if (isTerminal(target)) {
            const key = `terminal_${target.outcome}`;
            const existing = terminals.get(key);
            if (existing) {
              existing.sources.push(`${step.id}:${label}`);
            } else {
              terminals.set(key, { outcome: target.outcome, sources: [`${step.id}:${label}`] });
            }
          }
        }
        if (step.on_failure.kind === "Terminate") {
          const key = `terminal_${step.on_failure.outcome}`;
          if (!terminals.has(key)) {
            terminals.set(key, { outcome: step.on_failure.outcome, sources: [`${step.id}:failure`] });
          }
        }
      } else if (step.kind === "BranchStep") {
        for (const target of [step.if_true, step.if_false]) {
          if (isTerminal(target)) {
            const key = `terminal_${target.outcome}`;
            if (!terminals.has(key)) {
              terminals.set(key, { outcome: target.outcome, sources: [step.id] });
            }
          }
        }
      } else if (step.kind === "SubFlowStep") {
        if (isTerminal(step.on_success)) {
          const key = `terminal_${step.on_success.outcome}`;
          if (!terminals.has(key)) {
            terminals.set(key, { outcome: step.on_success.outcome, sources: [step.id] });
          }
        }
        if (step.on_failure.kind === "Terminate") {
          const key = `terminal_${step.on_failure.outcome}`;
          if (!terminals.has(key)) {
            terminals.set(key, { outcome: step.on_failure.outcome, sources: [step.id] });
          }
        }
      }
    }

    const result: TerminalNode[] = [];
    let yi = 0;
    for (const [id, info] of terminals.entries()) {
      result.push({
        id,
        outcome: info.outcome,
        x: terminalX,
        y: MARGIN + yi * (TERMINAL_H + V_GAP),
        width: TERMINAL_W,
        height: TERMINAL_H,
      });
      yi++;
    }
    return result;
  }, [steps, nodeMap]);

  // Compute all edges including terminal edges and failure edges
  const allEdges = useMemo(() => {
    const edges: LabeledEdge[] = [];

    for (const step of steps) {
      const fromNode = nodeMap.get(step.id);
      if (!fromNode) continue;

      if (step.kind === "OperationStep") {
        for (const [label, target] of Object.entries(step.outcomes)) {
          const toId = stepTargetId(target);
          if (toId) {
            edges.push({ from: step.id, to: toId, label });
          } else if (isTerminal(target)) {
            edges.push({ from: step.id, to: `terminal_${target.outcome}`, label });
          }
        }
        // Failure edge
        if (step.on_failure.kind === "Terminate") {
          edges.push({ from: step.id, to: `terminal_${step.on_failure.outcome}`, label: "failure", isFailure: true });
        } else if (step.on_failure.kind === "Escalate") {
          const esc = step.on_failure;
          if (esc.next) edges.push({ from: step.id, to: esc.next, label: "escalate", isFailure: true });
        }
      } else if (step.kind === "BranchStep") {
        const trueTarget = stepTargetId(step.if_true);
        if (trueTarget) edges.push({ from: step.id, to: trueTarget, label: "true" });
        else if (isTerminal(step.if_true)) edges.push({ from: step.id, to: `terminal_${step.if_true.outcome}`, label: "true" });

        const falseTarget = stepTargetId(step.if_false);
        if (falseTarget) edges.push({ from: step.id, to: falseTarget, label: "false" });
        else if (isTerminal(step.if_false)) edges.push({ from: step.id, to: `terminal_${step.if_false.outcome}`, label: "false" });
      } else if (step.kind === "HandoffStep") {
        if (step.next) edges.push({ from: step.id, to: step.next });
      } else if (step.kind === "SubFlowStep") {
        const successTarget = stepTargetId(step.on_success);
        if (successTarget) edges.push({ from: step.id, to: successTarget, label: "success" });
        else if (isTerminal(step.on_success)) edges.push({ from: step.id, to: `terminal_${step.on_success.outcome}`, label: "success" });

        if (step.on_failure.kind === "Terminate") {
          edges.push({ from: step.id, to: `terminal_${step.on_failure.outcome}`, label: "failure", isFailure: true });
        }
      } else if (step.kind === "ParallelStep") {
        const successTarget = stepTargetId(step.join.on_all_success);
        if (successTarget) edges.push({ from: step.id, to: successTarget, label: "all done" });
        else if (isTerminal(step.join.on_all_success)) {
          edges.push({ from: step.id, to: `terminal_${step.join.on_all_success.outcome}`, label: "all done" });
        }
      }
    }
    return edges;
  }, [steps, nodeMap]);

  // Combined lookup for all nodes (step + terminal)
  function lookupNode(id: string): { x: number; y: number; width: number; height: number } | undefined {
    const stepNode = nodeMap.get(id);
    if (stepNode) return stepNode;
    return terminalNodes.find((t) => t.id === id);
  }

  // Pan handlers
  const handleMouseDown = useCallback(
    (e: React.MouseEvent<SVGSVGElement>) => {
      if (e.button !== 0) return;
      if (!editable) return;
      setIsPanning(true);
      panStart.current = { x: e.clientX, y: e.clientY, px: pan.x, py: pan.y };
    },
    [editable, pan]
  );

  const handleMouseMove = useCallback(
    (e: React.MouseEvent<SVGSVGElement>) => {
      if (draggingNode.current) {
        const svgRect = svgRef.current?.getBoundingClientRect();
        if (!svgRect) return;
        const svgX = (e.clientX - svgRect.left) / zoom - pan.x;
        const svgY = (e.clientY - svgRect.top) / zoom - pan.y;
        const newX = svgX - draggingNode.current.ox;
        const newY = svgY - draggingNode.current.oy;
        setNodePosOverrides((prev) => {
          const next = new Map(prev);
          next.set(draggingNode.current!.id, { x: newX, y: newY });
          return next;
        });
        return;
      }
      if (!isPanning || !panStart.current) return;
      const dx = (e.clientX - panStart.current.x) / zoom;
      const dy = (e.clientY - panStart.current.y) / zoom;
      setPan({ x: panStart.current.px + dx, y: panStart.current.py + dy });
    },
    [isPanning, zoom]
  );

  const handleMouseUp = useCallback(() => {
    setIsPanning(false);
    panStart.current = null;
    draggingNode.current = null;
  }, []);

  const handleWheel = useCallback((e: React.WheelEvent<SVGSVGElement>) => {
    e.preventDefault();
    const factor = e.deltaY > 0 ? 0.9 : 1.1;
    setZoom((z) => Math.min(3, Math.max(0.3, z * factor)));
  }, []);

  function handleNodeMouseDown(e: React.MouseEvent, nodeId: string) {
    if (!editable) return;
    e.stopPropagation();
    const node = nodeMap.get(nodeId);
    if (!node) return;
    const svgRect = svgRef.current?.getBoundingClientRect();
    if (!svgRect) return;
    const svgX = (e.clientX - svgRect.left) / zoom - pan.x;
    const svgY = (e.clientY - svgRect.top) / zoom - pan.y;
    draggingNode.current = {
      id: nodeId,
      ox: svgX - node.x,
      oy: svgY - node.y,
    };
  }

  function handleNodeClick(e: React.MouseEvent, stepId: string) {
    e.stopPropagation();
    setSelectedStep(stepId);
    onStepClick?.(stepId);
  }

  const svgW = layout.width + terminalNodes.length > 0 ? layout.width + TERMINAL_W + H_GAP + MARGIN : layout.width + MARGIN;
  const svgH = Math.max(layout.height + MARGIN, terminalNodes.length * (TERMINAL_H + V_GAP) + MARGIN);

  if (steps.length === 0) {
    return (
      <div className="flex h-40 items-center justify-center rounded border-2 border-dashed border-gray-200 text-sm text-gray-400">
        No steps yet — use the toolbar to add steps.
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded border border-gray-200 bg-white">
      <svg
        ref={svgRef}
        width="100%"
        height={Math.max(300, svgH * zoom)}
        viewBox={`${-pan.x} ${-pan.y} ${svgW / zoom} ${svgH / zoom}`}
        className="select-none"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onWheel={handleWheel}
        style={{ cursor: isPanning ? "grabbing" : editable ? "grab" : "default" }}
      >
        <ArrowDefs />

        {/* Edges */}
        {allEdges.map((edge, i) => {
          const fromNode = lookupNode(edge.from);
          const toNode = lookupNode(edge.to);
          return (
            <EdgePath
              key={i}
              edge={edge}
              fromNode={fromNode}
              toNode={toNode}
              onClick={onEdgeClick ? () => onEdgeClick(edge.from, edge.to, edge.label) : undefined}
            />
          );
        })}

        {/* Step nodes */}
        {steps.map((step) => {
          const node = nodeMap.get(step.id);
          if (!node) return null;
          const flowNode: FlowNode = {
            id: step.id,
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
            kind: step.kind,
            label: step.id,
            type: step.kind,
          };
          const isSelected = selectedStep === step.id;
          const isHighlighted = highlightedStep === step.id;
          const isEntry = step.id === entry;

          const commonProps = { node: flowNode, step, isSelected, isHighlighted, isEntry, onClick: (e: React.MouseEvent) => handleNodeClick(e, step.id) };

          return (
            <g
              key={step.id}
              onMouseDown={(e) => handleNodeMouseDown(e, step.id)}
            >
              {step.kind === "OperationStep" && <OperationStepNode {...commonProps} />}
              {step.kind === "BranchStep" && <BranchStepNode {...commonProps} />}
              {step.kind === "HandoffStep" && <HandoffStepNode {...commonProps} />}
              {step.kind === "SubFlowStep" && <SubFlowStepNode {...commonProps} />}
              {step.kind === "ParallelStep" && <ParallelStepNode {...commonProps} />}

              {/* Step ID label below node */}
              <text
                x={node.x + STEP_W / 2}
                y={node.y + STEP_H + 13}
                textAnchor="middle"
                fontSize={9}
                fill="#9ca3af"
                className="select-none pointer-events-none"
              >
                {step.id}
              </text>
            </g>
          );
        })}

        {/* Terminal nodes */}
        {terminalNodes.map((tnode) => (
          <TerminalStepNode key={tnode.id} node={tnode} />
        ))}
      </svg>

      {/* Zoom controls */}
      {editable && (
        <div className="flex items-center gap-1 border-t border-gray-100 bg-gray-50 px-3 py-1">
          <button
            onClick={() => setZoom((z) => Math.min(3, z * 1.2))}
            className="rounded px-2 py-0.5 text-xs text-gray-500 hover:bg-gray-200"
          >
            +
          </button>
          <span className="text-xs text-gray-400">{Math.round(zoom * 100)}%</span>
          <button
            onClick={() => setZoom((z) => Math.max(0.3, z * 0.8))}
            className="rounded px-2 py-0.5 text-xs text-gray-500 hover:bg-gray-200"
          >
            −
          </button>
          <button
            onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }); }}
            className="rounded px-2 py-0.5 text-xs text-gray-500 hover:bg-gray-200"
          >
            Reset
          </button>
        </div>
      )}
    </div>
  );
}
