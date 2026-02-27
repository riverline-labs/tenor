/**
 * Application shell layout with sidebar navigation, toolbar, and error panel.
 */
import React, { useState, useEffect } from "react";
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
import { ExportDialog } from "./shared/ExportDialog";
import { ImportDialog } from "./shared/ImportDialog";
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

interface ToolbarProps {
  onOpenExport: () => void;
  onOpenImport: () => void;
}

function Toolbar({ onOpenExport, onOpenImport }: ToolbarProps) {
  const bundle = useContractStore((s) => s.bundle);
  const undoable = canUndo();
  const redoable = canRedo();

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
          onClick={onOpenImport}
          className="rounded border border-gray-200 bg-gray-50 px-3 py-1 text-sm text-gray-600 hover:bg-gray-100"
          title="Import contract (Ctrl+I)"
        >
          Import
        </button>
        <button
          onClick={onOpenExport}
          className="rounded border border-blue-200 bg-blue-50 px-3 py-1 text-sm text-blue-600 hover:bg-blue-100"
          title="Export contract (Ctrl+E)"
        >
          Export
        </button>
      </div>
    </div>
  );
}

export function Layout() {
  const navigate = useNavigate();
  const [exportOpen, setExportOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);

  // Keyboard shortcuts: Ctrl+E for export, Ctrl+I for import
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (!e.ctrlKey && !e.metaKey) return;
      if (e.key === "e" || e.key === "E") {
        e.preventDefault();
        setExportOpen(true);
      }
      if (e.key === "i" || e.key === "I") {
        e.preventDefault();
        setImportOpen(true);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

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
      window.location.href = route;
    }
  }

  function handleImported() {
    // Navigate to overview after successful import
    navigate("/");
  }

  return (
    <div className="flex h-screen flex-col">
      {/* Top toolbar */}
      <Toolbar
        onOpenExport={() => setExportOpen(true)}
        onOpenImport={() => setImportOpen(true)}
      />

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

      {/* Modals */}
      {exportOpen && (
        <ExportDialog onClose={() => setExportOpen(false)} />
      )}
      {importOpen && (
        <ImportDialog
          onClose={() => setImportOpen(false)}
          onImported={handleImported}
        />
      )}
    </div>
  );
}
