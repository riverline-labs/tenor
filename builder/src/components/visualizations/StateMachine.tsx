/**
 * Interactive SVG state machine diagram component.
 *
 * Renders entity state machines as interactive SVG with:
 * - Force-directed layout via layoutStateMachine()
 * - Pan, zoom, drag-to-rearrange nodes
 * - Visual distinction for initial state (double border + entry arrow)
 * - Curved arrows with arrowheads for transitions
 * - Self-loop transitions
 * - Selection highlight on click
 * - CSS transition animations
 */
import React, { useEffect, useRef, useState } from "react";
import {
  layoutStateMachine,
  type StateMachineNode,
  type LayoutEdge,
} from "@/utils/layout";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface StateMachineProps {
  states: string[];
  transitions: [string, string][];
  initialState: string;
  onStateClick?: (state: string) => void;
  onTransitionClick?: (from: string, to: string) => void;
  onAddTransition?: (from: string, to: string) => void;
  editable?: boolean;
  selectedState?: string | null;
  selectedTransition?: [string, string] | null;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SVG_WIDTH = 600;
const SVG_HEIGHT = 400;
const NODE_RX = 8; // border radius for state rects

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function StateMachine({
  states,
  transitions,
  initialState,
  onStateClick,
  onTransitionClick,
  onAddTransition,
  editable = false,
  selectedState = null,
  selectedTransition = null,
}: StateMachineProps) {
  // Convert transitions to layout format
  const layoutTransitions = transitions.map(([from, to]) => ({ from, to }));

  // Layout result
  const layout = layoutStateMachine(states, layoutTransitions, initialState);

  // Viewport state for pan/zoom
  const [viewport, setViewport] = useState({ x: 0, y: 0, scale: 1 });
  const svgRef = useRef<SVGSVGElement>(null);

  // Per-node drag positions (overrides layout positions when dragged)
  const [nodePosOverrides, setNodePosOverrides] = useState<
    Record<string, { x: number; y: number }>
  >({});

  // Reset overrides when states change
  useEffect(() => {
    setNodePosOverrides({});
  }, [states.join(","), transitions.map(([f, t]) => `${f}->${t}`).join(",")]);

  // Pan state
  const panStart = useRef<{ x: number; y: number; vpX: number; vpY: number } | null>(null);

  // Drag state
  const dragState = useRef<{
    nodeId: string;
    startX: number;
    startY: number;
    origX: number;
    origY: number;
  } | null>(null);

  // Add-transition mode: first click selects source, second selects target
  const addTransitionSource = useRef<string | null>(null);
  const [addMode, setAddMode] = useState(false);
  const [addSource, setAddSource] = useState<string | null>(null);

  // Build effective node positions (layout + overrides)
  const effectiveNodes: StateMachineNode[] = layout.nodes.map((n) => {
    const smNode = n as StateMachineNode;
    return {
      ...smNode,
      x: nodePosOverrides[smNode.id]?.x ?? smNode.x,
      y: nodePosOverrides[smNode.id]?.y ?? smNode.y,
    };
  });

  // Rebuild edges based on effective node positions
  const effectiveEdges: LayoutEdge[] = layoutTransitions.map((t) => {
    const fromNode = effectiveNodes.find((n) => n.id === t.from)!;
    const toNode = effectiveNodes.find((n) => n.id === t.to)!;
    if (!fromNode || !toNode) return { from: t.from, to: t.to };

    if (t.from === t.to) {
      // Self-loop
      const cx = fromNode.x + fromNode.width / 2;
      const cy = fromNode.y - 30;
      return {
        from: t.from,
        to: t.to,
        controlPoints: [
          { x: cx - 25, y: cy - 20 },
          { x: cx + 25, y: cy - 20 },
        ],
      };
    }

    const mx = (fromNode.x + toNode.x) / 2 + fromNode.width / 2;
    const my = (fromNode.y + toNode.y) / 2 + fromNode.height / 2;
    return {
      from: t.from,
      to: t.to,
      controlPoints: [{ x: mx, y: my }],
    };
  });

  // SVG coordinate helpers
  function svgPoint(e: React.MouseEvent): { x: number; y: number } {
    const rect = svgRef.current!.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left - viewport.x) / viewport.scale,
      y: (e.clientY - rect.top - viewport.y) / viewport.scale,
    };
  }

  // Mouse wheel for zoom
  function handleWheel(e: React.WheelEvent) {
    if (!editable) return;
    e.preventDefault();
    const delta = e.deltaY < 0 ? 1.1 : 0.9;
    setViewport((v) => ({
      ...v,
      scale: Math.max(0.2, Math.min(3, v.scale * delta)),
    }));
  }

  // Background mouse down for pan
  function handleBgMouseDown(e: React.MouseEvent) {
    if (!editable || e.button !== 0) return;
    panStart.current = {
      x: e.clientX,
      y: e.clientY,
      vpX: viewport.x,
      vpY: viewport.y,
    };
  }

  // Node mouse down for drag
  function handleNodeMouseDown(e: React.MouseEvent, nodeId: string) {
    if (!editable) return;
    e.stopPropagation();

    if (addMode) {
      // In add-transition mode, handle state click
      handleStateClickInAddMode(nodeId);
      return;
    }

    const node = effectiveNodes.find((n) => n.id === nodeId)!;
    dragState.current = {
      nodeId,
      startX: e.clientX,
      startY: e.clientY,
      origX: node.x,
      origY: node.y,
    };
  }

  function handleStateClickInAddMode(nodeId: string) {
    if (addSource === null) {
      setAddSource(nodeId);
      addTransitionSource.current = nodeId;
    } else {
      // Complete the transition
      if (onAddTransition && addTransitionSource.current !== null) {
        onAddTransition(addTransitionSource.current, nodeId);
      }
      setAddSource(null);
      setAddMode(false);
      addTransitionSource.current = null;
    }
  }

  // Global mouse move
  function handleMouseMove(e: React.MouseEvent) {
    if (panStart.current) {
      const dx = e.clientX - panStart.current.x;
      const dy = e.clientY - panStart.current.y;
      setViewport((v) => ({
        ...v,
        x: panStart.current!.vpX + dx,
        y: panStart.current!.vpY + dy,
      }));
    }
    if (dragState.current) {
      const ds = dragState.current;
      const dx = (e.clientX - ds.startX) / viewport.scale;
      const dy = (e.clientY - ds.startY) / viewport.scale;
      setNodePosOverrides((prev) => ({
        ...prev,
        [ds.nodeId]: { x: ds.origX + dx, y: ds.origY + dy },
      }));
    }
  }

  function handleMouseUp() {
    panStart.current = null;
    dragState.current = null;
  }

  function handleNodeClick(e: React.MouseEvent, nodeId: string) {
    e.stopPropagation();
    if (addMode) return;
    onStateClick?.(nodeId);
  }

  function handleEdgeClick(e: React.MouseEvent, from: string, to: string) {
    e.stopPropagation();
    onTransitionClick?.(from, to);
  }

  // Path helpers
  function edgePath(edge: LayoutEdge): string {
    const fromNode = effectiveNodes.find((n) => n.id === edge.from);
    const toNode = effectiveNodes.find((n) => n.id === edge.to);
    if (!fromNode || !toNode) return "";

    const fw = fromNode.width;
    const fh = fromNode.height;
    const tw = toNode.width;
    const th = toNode.height;

    // Entry/exit points from centers
    const fx = fromNode.x + fw / 2;
    const fy = fromNode.y + fh / 2;
    const tx = toNode.x + tw / 2;
    const ty = toNode.y + th / 2;

    if (edge.from === edge.to) {
      // Self-loop: cubic bezier above the node
      const cx = fromNode.x + fw / 2;
      const cy = fromNode.y - 30;
      const sx = cx - fw / 4;
      const sy = fromNode.y;
      const ex = cx + fw / 4;
      const ey = fromNode.y;
      return `M ${sx} ${sy} C ${sx} ${cy - 20} ${ex} ${cy - 20} ${ex} ${ey}`;
    }

    // Straight or slightly curved: use midpoint as quadratic control
    const cps = edge.controlPoints;
    if (cps && cps.length === 1) {
      // Quadratic bezier: snap from/to to node edges
      return `M ${fx} ${fy} Q ${cps[0].x} ${cps[0].y} ${tx} ${ty}`;
    }

    return `M ${fx} ${fy} L ${tx} ${ty}`;
  }

  // Arrowhead marker
  function arrowheadPos(
    edge: LayoutEdge
  ): { x: number; y: number; angle: number } | null {
    const fromNode = effectiveNodes.find((n) => n.id === edge.from);
    const toNode = effectiveNodes.find((n) => n.id === edge.to);
    if (!fromNode || !toNode) return null;

    const tx = toNode.x + toNode.width / 2;
    const ty = toNode.y + toNode.height / 2;

    if (edge.from === edge.to) {
      // Self-loop: arrowhead at bottom-right of loop
      const ex = tx + toNode.width / 4;
      const ey = toNode.y;
      return { x: ex, y: ey, angle: 90 };
    }

    const cps = edge.controlPoints;
    if (cps && cps.length === 1) {
      const dx = tx - cps[0].x;
      const dy = ty - cps[0].y;
      const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
      return { x: tx, y: ty, angle };
    }

    const fx = fromNode.x + fromNode.width / 2;
    const fy = fromNode.y + fromNode.height / 2;
    const dx = tx - fx;
    const dy = ty - fy;
    const angle = (Math.atan2(dy, dx) * 180) / Math.PI;
    return { x: tx, y: ty, angle };
  }

  if (states.length === 0) {
    return (
      <div className="flex h-48 items-center justify-center rounded border-2 border-dashed border-gray-200 text-sm text-gray-400">
        No states — add one to begin
      </div>
    );
  }

  const isSelectedEdge = (from: string, to: string) =>
    selectedTransition?.[0] === from && selectedTransition?.[1] === to;

  return (
    <div className="relative rounded border border-gray-200 bg-white">
      {/* Add-transition mode banner */}
      {addMode && (
        <div className="absolute left-0 right-0 top-0 z-10 rounded-t bg-blue-500 px-3 py-1 text-center text-xs text-white">
          {addSource === null
            ? "Click source state..."
            : `From: ${addSource} — click target state`}
          <button
            onClick={() => {
              setAddMode(false);
              setAddSource(null);
            }}
            className="ml-3 underline"
          >
            Cancel
          </button>
        </div>
      )}

      <svg
        ref={svgRef}
        width="100%"
        viewBox={`0 0 ${SVG_WIDTH} ${SVG_HEIGHT}`}
        style={{ minHeight: 200, cursor: editable ? (panStart.current ? "grabbing" : "grab") : "default" }}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onMouseDown={handleBgMouseDown}
        onWheel={handleWheel}
      >
        <defs>
          <marker
            id="arrowhead"
            markerWidth="10"
            markerHeight="7"
            refX="9"
            refY="3.5"
            orient="auto"
          >
            <polygon points="0 0, 10 3.5, 0 7" fill="#6b7280" />
          </marker>
          <marker
            id="arrowhead-selected"
            markerWidth="10"
            markerHeight="7"
            refX="9"
            refY="3.5"
            orient="auto"
          >
            <polygon points="0 0, 10 3.5, 0 7" fill="#ef4444" />
          </marker>
          <marker
            id="arrowhead-blue"
            markerWidth="10"
            markerHeight="7"
            refX="9"
            refY="3.5"
            orient="auto"
          >
            <polygon points="0 0, 10 3.5, 0 7" fill="#3b82f6" />
          </marker>
        </defs>

        <g
          transform={`translate(${viewport.x}, ${viewport.y}) scale(${viewport.scale})`}
        >
          {/* Render edges */}
          {effectiveEdges.map((edge, idx) => {
            const isSelected = isSelectedEdge(edge.from, edge.to);
            const color = isSelected ? "#ef4444" : "#6b7280";
            const markerId = isSelected
              ? "arrowhead-selected"
              : "arrowhead";
            const path = edgePath(edge);

            return (
              <g key={`edge-${idx}`}>
                {/* Hit area for clicking */}
                <path
                  d={path}
                  stroke="transparent"
                  strokeWidth={12}
                  fill="none"
                  style={{ cursor: editable ? "pointer" : "default" }}
                  onClick={(e) => handleEdgeClick(e, edge.from, edge.to)}
                />
                <path
                  d={path}
                  stroke={color}
                  strokeWidth={isSelected ? 2 : 1.5}
                  fill="none"
                  markerEnd={`url(#${markerId})`}
                  style={{ pointerEvents: "none", transition: "stroke 0.15s" }}
                />
                {/* Edge label on hover — show as title */}
                <title>{`${edge.from} → ${edge.to}`}</title>
              </g>
            );
          })}

          {/* Render nodes */}
          {effectiveNodes.map((node) => {
            const isInitial = node.isInitial;
            const isSelected = selectedState === node.id;
            const isAddSource = addSource === node.id;

            let strokeColor = "#d1d5db";
            let fillColor = "#ffffff";
            let strokeWidth = 1.5;

            if (isInitial) {
              strokeColor = "#3b82f6";
              fillColor = "#eff6ff";
            }
            if (isSelected) {
              strokeColor = "#ef4444";
              strokeWidth = 2.5;
            }
            if (isAddSource) {
              strokeColor = "#10b981";
              strokeWidth = 2.5;
            }

            return (
              <g
                key={node.id}
                style={{ cursor: editable ? "pointer" : "default", transition: "transform 0.1s" }}
                onMouseDown={(e) => handleNodeMouseDown(e, node.id)}
                onClick={(e) => handleNodeClick(e, node.id)}
              >
                {/* Initial state: outer double border */}
                {isInitial && (
                  <rect
                    x={node.x - 3}
                    y={node.y - 3}
                    width={node.width + 6}
                    height={node.height + 6}
                    rx={NODE_RX + 2}
                    ry={NODE_RX + 2}
                    fill="none"
                    stroke="#3b82f6"
                    strokeWidth={1}
                    opacity={0.5}
                  />
                )}

                {/* Main state rect */}
                <rect
                  x={node.x}
                  y={node.y}
                  width={node.width}
                  height={node.height}
                  rx={NODE_RX}
                  ry={NODE_RX}
                  fill={fillColor}
                  stroke={strokeColor}
                  strokeWidth={strokeWidth}
                  style={{ transition: "stroke 0.15s, fill 0.15s" }}
                />

                {/* State name */}
                <text
                  x={node.x + node.width / 2}
                  y={node.y + node.height / 2 + 4}
                  textAnchor="middle"
                  fontSize={11}
                  fontFamily="ui-monospace, monospace"
                  fill={isInitial ? "#1d4ed8" : "#374151"}
                  style={{ pointerEvents: "none", userSelect: "none" }}
                >
                  {node.label}
                </text>

                {/* Initial state entry indicator: filled circle + arrow */}
                {isInitial && (
                  <>
                    <circle
                      cx={node.x - 18}
                      cy={node.y + node.height / 2}
                      r={5}
                      fill="#3b82f6"
                    />
                    <line
                      x1={node.x - 13}
                      y1={node.y + node.height / 2}
                      x2={node.x}
                      y2={node.y + node.height / 2}
                      stroke="#3b82f6"
                      strokeWidth={1.5}
                      markerEnd="url(#arrowhead-blue)"
                    />
                  </>
                )}
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
}
