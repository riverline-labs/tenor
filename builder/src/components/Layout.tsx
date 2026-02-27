/**
 * Application shell layout with sidebar navigation, toolbar, and error panel.
 */
import React from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import {
  useContractStore,
  undoContract,
  redoContract,
  canUndo,
  canRedo,
} from "@/store/contract";
import { useElaborationStore } from "@/store/elaboration";
import { ErrorPanel } from "./shared/ErrorPanel";
import { generateDsl } from "@/utils/dsl-generator";
import type { ValidationError } from "@/store/elaboration";

interface NavItem {
  label: string;
  href: string;
  icon: string;
  countSelector?: (state: ReturnType<typeof useContractStore.getState>) => number;
}

const NAV_ITEMS: NavItem[] = [
  { label: "Overview", href: "/", icon: "⊞" },
  { label: "Personas", href: "/personas", icon: "👤", countSelector: (s) => s.personas().length },
  { label: "Sources", href: "/sources", icon: "🔌", countSelector: (s) => s.sources().length },
  { label: "Facts", href: "/facts", icon: "📊", countSelector: (s) => s.facts().length },
  { label: "Entities", href: "/entities", icon: "🔷", countSelector: (s) => s.entities().length },
  { label: "Rules", href: "/rules", icon: "⚖️", countSelector: (s) => s.rules().length },
  { label: "Operations", href: "/operations", icon: "⚡", countSelector: (s) => s.operations().length },
  { label: "Flows", href: "/flows", icon: "🔄", countSelector: (s) => s.flows().length },
  { label: "Systems", href: "/systems", icon: "🏗️", countSelector: (s) => s.systems().length },
  { label: "Simulation", href: "/simulation", icon: "▶" },
];

function SidebarLink({ item }: { item: NavItem }) {
  const count = item.countSelector
    ? useContractStore(item.countSelector)
    : null;

  return (
    <NavLink
      to={item.href}
      end={item.href === "/"}
      className={({ isActive }) =>
        `flex items-center justify-between rounded-md px-3 py-2 text-sm transition-colors ${
          isActive
            ? "bg-blue-100 text-blue-700 font-medium"
            : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
        }`
      }
    >
      <span className="flex items-center gap-2">
        <span className="w-5 text-center">{item.icon}</span>
        {item.label}
      </span>
      {count !== null && count > 0 && (
        <span className="rounded-full bg-gray-200 px-1.5 py-0.5 text-xs font-medium text-gray-600">
          {count}
        </span>
      )}
    </NavLink>
  );
}

function Toolbar() {
  const bundle = useContractStore((s) => s.bundle);
  const navigate = useNavigate();
  const undoable = canUndo();
  const redoable = canRedo();

  function handleExportDsl() {
    const dsl = generateDsl(bundle);
    const blob = new Blob([dsl], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${bundle.id || "contract"}.tenor`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleExportJson() {
    const json = JSON.stringify(bundle, null, 2);
    const blob = new Blob([json], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${bundle.id || "contract"}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function handleImportJson() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = (ev) => {
        try {
          const bundle = JSON.parse(ev.target?.result as string);
          useContractStore.getState().loadBundle(bundle);
          navigate("/");
        } catch {
          alert("Invalid interchange JSON file.");
        }
      };
      reader.readAsText(file);
    };
    input.click();
  }

  return (
    <div className="flex h-12 items-center justify-between border-b border-gray-200 bg-white px-4">
      {/* Left: Contract name */}
      <div className="flex items-center gap-3">
        <span className="text-sm font-semibold text-gray-800">
          Tenor Builder
        </span>
        <span className="text-gray-300">|</span>
        <span className="text-sm text-gray-600">
          {bundle.id || "untitled"}
        </span>
      </div>

      {/* Center: Undo/Redo */}
      <div className="flex items-center gap-1">
        <button
          onClick={undoContract}
          disabled={!undoable}
          className="rounded px-2 py-1 text-sm text-gray-500 hover:bg-gray-100 disabled:opacity-30"
          title="Undo"
        >
          ↩ Undo
        </button>
        <button
          onClick={redoContract}
          disabled={!redoable}
          className="rounded px-2 py-1 text-sm text-gray-500 hover:bg-gray-100 disabled:opacity-30"
          title="Redo"
        >
          Redo ↪
        </button>
      </div>

      {/* Right: Import/Export */}
      <div className="flex items-center gap-2">
        <button
          onClick={handleImportJson}
          className="rounded border border-gray-200 bg-gray-50 px-3 py-1 text-sm text-gray-600 hover:bg-gray-100"
        >
          Import JSON
        </button>
        <div className="relative">
          <button
            onClick={handleExportDsl}
            className="rounded border border-blue-200 bg-blue-50 px-3 py-1 text-sm text-blue-600 hover:bg-blue-100"
          >
            Export .tenor
          </button>
        </div>
        <button
          onClick={handleExportJson}
          className="rounded border border-gray-200 bg-gray-50 px-3 py-1 text-sm text-gray-600 hover:bg-gray-100"
        >
          Export JSON
        </button>
      </div>
    </div>
  );
}

export function Layout() {
  function handleNavigateToError(error: ValidationError) {
    // Navigate to the relevant editor based on construct kind
    const kindToRoute: Record<string, string> = {
      Fact: "/facts",
      Entity: "/entities",
      Rule: "/rules",
      Operation: "/operations",
      Flow: "/flows",
      Persona: "/personas",
      Source: "/sources",
      System: "/systems",
    };
    const route = error.construct_kind
      ? kindToRoute[error.construct_kind]
      : null;
    if (route) {
      // Navigate — router handles this
      window.location.href = route;
    }
  }

  return (
    <div className="flex h-screen flex-col">
      {/* Top toolbar */}
      <Toolbar />

      {/* Body: sidebar + main content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="flex w-52 flex-shrink-0 flex-col gap-1 overflow-y-auto border-r border-gray-200 bg-gray-50 px-2 py-3">
          {NAV_ITEMS.map((item) => (
            <SidebarLink key={item.href} item={item} />
          ))}
        </nav>

        {/* Main content + error panel */}
        <div className="flex flex-1 flex-col overflow-hidden">
          {/* Scrollable main area */}
          <main className="flex-1 overflow-y-auto bg-gray-50">
            <Outlet />
          </main>

          {/* Bottom error panel */}
          <ErrorPanel onNavigateToError={handleNavigateToError} />
        </div>
      </div>
    </div>
  );
}
