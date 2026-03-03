// @ts-check
// Agent Capabilities Panel client-side script.
// Receives messages from the extension and renders the panel content.

(function () {
  // @ts-ignore
  const vscode = acquireVsCodeApi();

  /** Handle messages from the extension host. */
  window.addEventListener("message", (event) => {
    const message = event.data;
    switch (message.type) {
      case "update":
        renderCapabilities(message.data);
        break;
      case "loading":
        showLoading();
        break;
      case "error":
        showError(message.message);
        break;
    }
  });

  function showLoading() {
    const root = document.getElementById("root");
    if (root) {
      root.innerHTML = `
        <div class="loading">
          <div class="spinner"></div>
          <div>Analyzing contract...</div>
        </div>
      `;
    }
  }

  function showError(message) {
    const root = document.getElementById("root");
    if (root) {
      root.innerHTML = `
        <div class="error-message">
          <strong>Elaboration Error</strong>
          <p>${escapeHtml(message)}</p>
        </div>
      `;
    }
  }

  /**
   * Render the full agent capabilities panel.
   * @param {object} caps - AgentCapabilities data from the LSP
   */
  function renderCapabilities(caps) {
    const root = document.getElementById("root");
    if (!root) return;

    if (caps.error) {
      showError(caps.error);
      return;
    }

    const html = [];

    // Toolbar
    html.push(`<div class="toolbar">`);
    html.push(
      `<span class="contract-id">${escapeHtml(caps.contract_id || "Contract")}</span>`
    );
    html.push(
      `<button class="refresh-btn" id="refreshBtn">Refresh</button>`
    );
    html.push(`</div>`);

    // Operations grouped by persona
    html.push(renderPersonaOperations(caps.personas, caps.operations));

    // Entities with SVG diagrams
    html.push(renderEntities(caps.entities));

    // Flows
    html.push(renderFlows(caps.flows));

    // Analysis findings
    html.push(renderFindings(caps.analysis_findings));

    root.innerHTML = html.join("\n");

    // Bind event handlers
    bindRefreshButton();
    bindSectionToggles();
  }

  /**
   * Render operations grouped by persona.
   */
  function renderPersonaOperations(personas, operations) {
    if (!personas || personas.length === 0) {
      return "";
    }

    const opMap = new Map();
    for (const op of operations || []) {
      opMap.set(op.id, op);
    }

    const parts = [];
    parts.push(
      `<h2 class="section-toggle" data-section="operations"><span class="toggle">&#9660;</span> Operations by Persona</h2>`
    );
    parts.push(`<div class="section-content" id="operations">`);

    for (const persona of personas) {
      parts.push(`<div class="persona-group">`);
      parts.push(
        `<div class="persona-header">${escapeHtml(persona.id)}</div>`
      );

      for (const opId of persona.operations || []) {
        const op = opMap.get(opId);
        if (!op) continue;
        parts.push(renderOperationCard(op));
      }

      if (!persona.operations || persona.operations.length === 0) {
        parts.push(
          `<div class="operation-card"><em>No operations</em></div>`
        );
      }

      parts.push(`</div>`);
    }

    parts.push(`</div>`);
    return parts.join("\n");
  }

  /**
   * Render a single operation card.
   */
  function renderOperationCard(op) {
    const parts = [];
    parts.push(`<div class="operation-card">`);
    parts.push(`<div class="op-name">${escapeHtml(op.id)}</div>`);

    // Parameters table
    if (op.parameters && op.parameters.length > 0) {
      parts.push(`<table>`);
      parts.push(`<tr><th>Parameter</th><th>Type</th></tr>`);
      for (const param of op.parameters) {
        parts.push(
          `<tr><td>${escapeHtml(param.name)}</td><td>${escapeHtml(param.fact_type)}</td></tr>`
        );
      }
      parts.push(`</table>`);
    }

    // Preconditions
    if (op.preconditions && op.preconditions.length > 0) {
      for (const pre of op.preconditions) {
        parts.push(`<div class="precondition">${escapeHtml(pre)}</div>`);
      }
    }

    // Effects
    if (op.effects && op.effects.length > 0) {
      parts.push(`<ul class="effects-list">`);
      for (const eff of op.effects) {
        parts.push(
          `<li>${escapeHtml(eff.entity)}: ${escapeHtml(eff.transition)}</li>`
        );
      }
      parts.push(`</ul>`);
    }

    // Outcomes
    if (op.outcomes && op.outcomes.length > 0) {
      for (const outcome of op.outcomes) {
        parts.push(
          `<span class="outcome-badge">${escapeHtml(outcome)}</span>`
        );
      }
    }

    parts.push(`</div>`);
    return parts.join("\n");
  }

  /**
   * Render entity cards with inline SVG diagrams.
   * SVG is generated server-side and included in the data.
   */
  function renderEntities(entities) {
    if (!entities || entities.length === 0) {
      return "";
    }

    const parts = [];
    parts.push(
      `<h2 class="section-toggle" data-section="entities"><span class="toggle">&#9660;</span> Entity State Machines</h2>`
    );
    parts.push(`<div class="section-content" id="entities">`);

    for (const entity of entities) {
      parts.push(`<div class="entity-card">`);
      parts.push(
        `<div class="entity-name">${escapeHtml(entity.id)}</div>`
      );
      parts.push(
        `<div style="font-size:0.9em;color:var(--vscode-descriptionForeground)">States: ${entity.states.map(escapeHtml).join(", ")} (initial: ${escapeHtml(entity.initial_state)})</div>`
      );
      // SVG is rendered in the panel.js since we have the entity data
      if (entity.svg) {
        parts.push(entity.svg);
      } else {
        parts.push(renderStateMachineSvg(entity));
      }
      parts.push(`</div>`);
    }

    parts.push(`</div>`);
    return parts.join("\n");
  }

  /**
   * Client-side SVG state machine renderer (fallback).
   */
  function renderStateMachineSvg(entity) {
    const states = entity.states || [];
    const initial = entity.initial_state;
    const transitions = entity.transitions || [];

    if (states.length === 0) return "";

    const stateW = 120;
    const stateH = 40;
    const rx = 8;
    const pad = 60;
    const spacing = stateW + pad;

    // Simple horizontal layout for readability
    const positions = new Map();
    if (states.length <= 4) {
      for (let i = 0; i < states.length; i++) {
        positions.set(states[i], { x: pad + i * spacing + stateW / 2, y: pad + stateH });
      }
    } else {
      const radius = Math.max(120, states.length * 30);
      const cx = radius + pad + stateW / 2;
      const cy = radius + pad + stateH / 2;
      for (let i = 0; i < states.length; i++) {
        const angle = (2 * Math.PI * i) / states.length - Math.PI / 2;
        positions.set(states[i], {
          x: cx + radius * Math.cos(angle),
          y: cy + radius * Math.sin(angle),
        });
      }
    }

    // Compute viewBox
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const pos of positions.values()) {
      minX = Math.min(minX, pos.x - stateW / 2);
      minY = Math.min(minY, pos.y - stateH / 2);
      maxX = Math.max(maxX, pos.x + stateW / 2);
      maxY = Math.max(maxY, pos.y + stateH / 2);
    }

    const vx = minX - pad;
    const vy = minY - pad * 1.5;
    const vw = maxX - minX + pad * 2;
    const vh = maxY - minY + pad * 3;

    const parts = [];
    parts.push(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="${vx} ${vy} ${vw} ${vh}" style="width:100%;max-height:350px">`);

    // Arrow marker
    parts.push(`<defs><marker id="arr-${escapeHtml(entity.id)}" markerWidth="8" markerHeight="8" refX="8" refY="4" orient="auto" markerUnits="strokeWidth"><path d="M 0 0 L 8 4 L 0 8 Z" fill="var(--vscode-editorWidget-border, #666)" /></marker></defs>`);

    // Transitions
    for (const [from, to] of transitions) {
      const fp = positions.get(from);
      const tp = positions.get(to);
      if (!fp || !tp) continue;

      if (from === to) {
        const topY = fp.y - stateH / 2;
        parts.push(`<path d="M ${fp.x - 20} ${topY} C ${fp.x - 30} ${topY - 30}, ${fp.x + 30} ${topY - 30}, ${fp.x + 20} ${topY}" fill="none" stroke="var(--vscode-editorWidget-border, #666)" stroke-width="1.5" marker-end="url(#arr-${escapeHtml(entity.id)})" />`);
      } else {
        const angle = Math.atan2(tp.y - fp.y, tp.x - fp.x);
        const sx = fp.x + (stateW / 2) * Math.cos(angle);
        const sy = fp.y + (stateH / 2) * Math.sin(angle);
        const ex = tp.x - (stateW / 2 + 8) * Math.cos(angle);
        const ey = tp.y - (stateH / 2 + 8) * Math.sin(angle);
        const mx = (sx + ex) / 2 - (ey - sy) * 0.1;
        const my = (sy + ey) / 2 + (ex - sx) * 0.1;
        parts.push(`<path d="M ${sx} ${sy} Q ${mx} ${my} ${ex} ${ey}" fill="none" stroke="var(--vscode-editorWidget-border, #666)" stroke-width="1.5" marker-end="url(#arr-${escapeHtml(entity.id)})" />`);
      }
    }

    // States
    for (const state of states) {
      const pos = positions.get(state);
      if (!pos) continue;
      const isInit = state === initial;
      const fill = isInit ? "var(--vscode-badge-background, #007acc)" : "var(--vscode-editor-background, #1e1e1e)";
      const textFill = isInit ? "var(--vscode-badge-foreground, #fff)" : "var(--vscode-foreground, #ccc)";
      const sw = isInit ? 2.5 : 1.5;
      parts.push(`<rect x="${pos.x - stateW / 2}" y="${pos.y - stateH / 2}" width="${stateW}" height="${stateH}" rx="${rx}" fill="${fill}" stroke="var(--vscode-editorWidget-border, #666)" stroke-width="${sw}" />`);
      parts.push(`<text x="${pos.x}" y="${pos.y + 5}" text-anchor="middle" fill="${textFill}" font-size="12">${escapeHtml(state)}</text>`);
    }

    parts.push(`</svg>`);
    return parts.join("\n");
  }

  /**
   * Render flow summaries.
   */
  function renderFlows(flows) {
    if (!flows || flows.length === 0) {
      return "";
    }

    const parts = [];
    parts.push(
      `<h2 class="section-toggle" data-section="flows"><span class="toggle">&#9660;</span> Flows</h2>`
    );
    parts.push(`<div class="section-content" id="flows">`);

    for (const flow of flows) {
      parts.push(`<div class="flow-card">`);
      parts.push(
        `<div class="flow-name">${escapeHtml(flow.id)}</div>`
      );
      parts.push(
        `<div class="flow-meta">Entry: ${escapeHtml(flow.entry_point)} | ${flow.step_count} step(s)</div>`
      );

      if (flow.step_summary && flow.step_summary.length > 0) {
        parts.push(`<ol class="step-list">`);
        for (const step of flow.step_summary) {
          parts.push(`<li>${escapeHtml(step)}</li>`);
        }
        parts.push(`</ol>`);
      }

      parts.push(`</div>`);
    }

    parts.push(`</div>`);
    return parts.join("\n");
  }

  /**
   * Render analysis findings.
   */
  function renderFindings(findings) {
    if (!findings || findings.length === 0) {
      return "";
    }

    const parts = [];
    parts.push(
      `<h2 class="section-toggle" data-section="findings"><span class="toggle">&#9660;</span> Analysis Findings (${findings.length})</h2>`
    );
    parts.push(`<div class="section-content" id="findings">`);

    for (const finding of findings) {
      const severityClass =
        finding.severity === "warning" ? "warning" : "info";
      parts.push(`<div class="finding">`);
      parts.push(
        `<span class="severity-badge ${severityClass}">${escapeHtml(finding.severity)}</span>`
      );
      parts.push(`<span class="finding-text">`);
      parts.push(
        `<span class="analysis-tag">[${escapeHtml(finding.analysis)}]</span> `
      );
      parts.push(`${escapeHtml(finding.message)}`);
      parts.push(`</span>`);
      parts.push(`</div>`);
    }

    parts.push(`</div>`);
    return parts.join("\n");
  }

  /** Bind refresh button. */
  function bindRefreshButton() {
    const btn = document.getElementById("refreshBtn");
    if (btn) {
      btn.addEventListener("click", () => {
        vscode.postMessage({ type: "refresh" });
      });
    }
  }

  /** Bind section expand/collapse toggles. */
  function bindSectionToggles() {
    const toggles = document.querySelectorAll(".section-toggle");
    toggles.forEach((toggle) => {
      toggle.addEventListener("click", () => {
        const section = toggle.getAttribute("data-section");
        const content = document.getElementById(section || "");
        if (content) {
          content.classList.toggle("collapsed");
          toggle.classList.toggle("collapsed");
        }
      });
    });
  }

  /** Escape HTML special characters. */
  function escapeHtml(s) {
    if (!s) return "";
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }
})();
