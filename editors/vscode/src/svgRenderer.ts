/**
 * SVG state machine diagram renderer for entity state machines.
 *
 * Generates responsive SVG diagrams using VS Code CSS variables
 * for theme-aware rendering. Layouts adapt based on state count:
 * - 2-4 states: horizontal layout
 * - 5+ states: circular layout
 */

export interface EntityView {
  id: string;
  states: string[];
  initial_state: string;
  transitions: [string, string][];
}

const STATE_WIDTH = 120;
const STATE_HEIGHT = 40;
const STATE_RX = 8;
const PADDING = 60;
const ARROW_SIZE = 8;

/**
 * Render an entity's state machine as an SVG string.
 */
export function renderStateMachine(entity: EntityView): string {
  const { states, initial_state, transitions } = entity;
  if (states.length === 0) {
    return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 50"><text x="100" y="25" text-anchor="middle" fill="var(--vscode-foreground)" font-size="12">No states defined</text></svg>';
  }

  const positions = computeLayout(states);
  const viewBox = computeViewBox(positions);

  const parts: string[] = [];
  parts.push(
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox.x} ${viewBox.y} ${viewBox.w} ${viewBox.h}" style="width:100%;max-height:400px">`
  );

  // Arrow marker definition
  parts.push("<defs>");
  parts.push(
    `<marker id="arrow-${entity.id}" markerWidth="${ARROW_SIZE}" markerHeight="${ARROW_SIZE}" refX="${ARROW_SIZE}" refY="${ARROW_SIZE / 2}" orient="auto" markerUnits="strokeWidth">`
  );
  parts.push(
    `<path d="M 0 0 L ${ARROW_SIZE} ${ARROW_SIZE / 2} L 0 ${ARROW_SIZE} Z" fill="var(--vscode-editorWidget-border, #666)" />`
  );
  parts.push("</marker>");
  parts.push("</defs>");

  // Draw transitions first (behind states)
  for (const [from, to] of transitions) {
    const fromPos = positions.get(from);
    const toPos = positions.get(to);
    if (!fromPos || !toPos) continue;

    if (from === to) {
      // Self-transition: loop above the state
      parts.push(renderSelfTransition(entity.id, fromPos));
    } else {
      parts.push(renderTransition(entity.id, fromPos, toPos));
    }
  }

  // Draw states
  for (const state of states) {
    const pos = positions.get(state);
    if (!pos) continue;
    const isInitial = state === initial_state;
    parts.push(renderState(state, pos, isInitial));
  }

  parts.push("</svg>");
  return parts.join("\n");
}

interface Point {
  x: number;
  y: number;
}

function computeLayout(states: string[]): Map<string, Point> {
  const positions = new Map<string, Point>();

  if (states.length <= 4) {
    // Horizontal layout
    const spacing = STATE_WIDTH + PADDING;
    const startX = PADDING;
    const y = PADDING + STATE_HEIGHT;
    for (let i = 0; i < states.length; i++) {
      positions.set(states[i], { x: startX + i * spacing, y });
    }
  } else {
    // Circular layout
    const radius = Math.max(120, states.length * 30);
    const cx = radius + PADDING + STATE_WIDTH / 2;
    const cy = radius + PADDING + STATE_HEIGHT / 2;
    for (let i = 0; i < states.length; i++) {
      const angle = (2 * Math.PI * i) / states.length - Math.PI / 2;
      positions.set(states[i], {
        x: cx + radius * Math.cos(angle),
        y: cy + radius * Math.sin(angle),
      });
    }
  }

  return positions;
}

function computeViewBox(positions: Map<string, Point>): {
  x: number;
  y: number;
  w: number;
  h: number;
} {
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const pos of positions.values()) {
    minX = Math.min(minX, pos.x - STATE_WIDTH / 2);
    minY = Math.min(minY, pos.y - STATE_HEIGHT / 2);
    maxX = Math.max(maxX, pos.x + STATE_WIDTH / 2);
    maxY = Math.max(maxY, pos.y + STATE_HEIGHT / 2);
  }

  const pad = PADDING;
  return {
    x: minX - pad,
    y: minY - pad * 1.5,
    w: maxX - minX + pad * 2,
    h: maxY - minY + pad * 3,
  };
}

function renderState(name: string, pos: Point, isInitial: boolean): string {
  const x = pos.x - STATE_WIDTH / 2;
  const y = pos.y - STATE_HEIGHT / 2;

  const fill = isInitial
    ? "var(--vscode-badge-background, #007acc)"
    : "var(--vscode-editor-background, #1e1e1e)";
  const textFill = isInitial
    ? "var(--vscode-badge-foreground, #fff)"
    : "var(--vscode-foreground, #ccc)";
  const stroke = "var(--vscode-editorWidget-border, #666)";
  const strokeWidth = isInitial ? 2.5 : 1.5;

  let svg = `<rect x="${x}" y="${y}" width="${STATE_WIDTH}" height="${STATE_HEIGHT}" rx="${STATE_RX}" fill="${fill}" stroke="${stroke}" stroke-width="${strokeWidth}" />`;
  svg += `<text x="${pos.x}" y="${pos.y + 5}" text-anchor="middle" fill="${textFill}" font-size="12" font-family="var(--vscode-font-family, sans-serif)">${escapeXml(name)}</text>`;
  return svg;
}

function renderTransition(
  entityId: string,
  from: Point,
  to: Point
): string {
  // Calculate edge points on the state rectangles
  const angle = Math.atan2(to.y - from.y, to.x - from.x);
  const startX = from.x + (STATE_WIDTH / 2) * Math.cos(angle);
  const startY = from.y + (STATE_HEIGHT / 2) * Math.sin(angle);
  const endX = to.x - (STATE_WIDTH / 2 + ARROW_SIZE) * Math.cos(angle);
  const endY = to.y - (STATE_HEIGHT / 2 + ARROW_SIZE) * Math.sin(angle);

  // Offset for a slight curve to avoid overlapping with reverse transitions
  const midX = (startX + endX) / 2;
  const midY = (startY + endY) / 2;
  const perpX = -(endY - startY) * 0.1;
  const perpY = (endX - startX) * 0.1;
  const ctrlX = midX + perpX;
  const ctrlY = midY + perpY;

  return `<path d="M ${startX} ${startY} Q ${ctrlX} ${ctrlY} ${endX} ${endY}" fill="none" stroke="var(--vscode-editorWidget-border, #666)" stroke-width="1.5" marker-end="url(#arrow-${entityId})" />`;
}

function renderSelfTransition(entityId: string, pos: Point): string {
  const topY = pos.y - STATE_HEIGHT / 2;
  const loopH = 30;
  const loopW = 20;
  const startX = pos.x - loopW;
  const endX = pos.x + loopW;

  return `<path d="M ${startX} ${topY} C ${startX - 10} ${topY - loopH}, ${endX + 10} ${topY - loopH}, ${endX} ${topY}" fill="none" stroke="var(--vscode-editorWidget-border, #666)" stroke-width="1.5" marker-end="url(#arrow-${entityId})" />`;
}

function escapeXml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
