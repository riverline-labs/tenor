/**
 * Application shell layout with sidebar navigation, toolbar, and error panel.
 */
import React, { useState, useEffect } from "react";
import { NavLink, Outlet, useNavigate } from "react-router";
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
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  SidebarProvider,
  Sidebar,
  SidebarContent,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarMenuBadge,
} from "@/components/ui/sidebar";
import {
  LayoutDashboard,
  Users,
  Plug,
  BarChart3,
  Diamond,
  Scale,
  Zap,
  RefreshCw,
  Building2,
  Play,
  Undo2,
  Redo2,
  Upload,
  Download,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

interface NavItem {
  label: string;
  href: string;
  icon: LucideIcon;
  countSelector?: (state: ReturnType<typeof useContractStore.getState>) => number;
}

const NAV_ITEMS: NavItem[] = [
  { label: "Overview", href: "/", icon: LayoutDashboard },
  { label: "Personas", href: "/personas", icon: Users, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Persona").length },
  { label: "Sources", href: "/sources", icon: Plug, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Source").length },
  { label: "Facts", href: "/facts", icon: BarChart3, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Fact").length },
  { label: "Entities", href: "/entities", icon: Diamond, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Entity").length },
  { label: "Rules", href: "/rules", icon: Scale, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Rule").length },
  { label: "Operations", href: "/operations", icon: Zap, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Operation").length },
  { label: "Flows", href: "/flows", icon: RefreshCw, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "Flow").length },
  { label: "Systems", href: "/systems", icon: Building2, countSelector: (s) => s.bundle.constructs.filter((c) => c.kind === "System").length },
  { label: "Simulation", href: "/simulation", icon: Play },
];

function SidebarLink({ item }: { item: NavItem }) {
  const count = item.countSelector
    ? useContractStore(item.countSelector)
    : null;
  const Icon = item.icon;

  return (
    <SidebarMenuItem>
      <SidebarMenuButton asChild>
        <NavLink
          to={item.href}
          end={item.href === "/"}
          className={({ isActive }) =>
            isActive
              ? "bg-sidebar-accent text-sidebar-accent-foreground font-medium"
              : ""
          }
        >
          <Icon className="h-4 w-4" />
          <span>{item.label}</span>
        </NavLink>
      </SidebarMenuButton>
      {count !== null && count > 0 && (
        <SidebarMenuBadge>
          <Badge variant="secondary" className="h-5 px-1.5 text-[10px]">
            {count}
          </Badge>
        </SidebarMenuBadge>
      )}
    </SidebarMenuItem>
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
    <div className="flex h-12 items-center justify-between border-b border-border bg-card px-4">
      {/* Left: Contract name */}
      <div className="flex items-center gap-3">
        <span className="text-sm font-semibold text-foreground">
          Tenor Builder
        </span>
        <Separator orientation="vertical" className="h-5" />
        <span className="text-sm text-muted-foreground">
          {bundle.id || "untitled"}
        </span>
      </div>

      {/* Center: Undo/Redo */}
      <div className="flex items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          onClick={undoContract}
          disabled={!undoable}
          title="Undo"
        >
          <Undo2 className="mr-1 h-4 w-4" />
          Undo
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={redoContract}
          disabled={!redoable}
          title="Redo"
        >
          Redo
          <Redo2 className="ml-1 h-4 w-4" />
        </Button>
      </div>

      {/* Right: Import/Export */}
      <div className="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={onOpenImport}
          title="Import contract (Ctrl+I)"
        >
          <Upload className="mr-1 h-4 w-4" />
          Import
        </Button>
        <Button
          size="sm"
          onClick={onOpenExport}
          title="Export contract (Ctrl+E)"
        >
          <Download className="mr-1 h-4 w-4" />
          Export
        </Button>
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
    navigate("/");
  }

  return (
    <SidebarProvider>
      <div className="flex h-screen w-full flex-col">
        {/* Top toolbar */}
        <Toolbar
          onOpenExport={() => setExportOpen(true)}
          onOpenImport={() => setImportOpen(true)}
        />

        {/* Body: sidebar + main content */}
        <div className="flex flex-1 overflow-hidden">
          {/* Sidebar */}
          <Sidebar collapsible="none" className="border-r border-border">
            <SidebarContent className="px-2 py-3">
              <SidebarMenu>
                {NAV_ITEMS.map((item) => (
                  <SidebarLink key={item.href} item={item} />
                ))}
              </SidebarMenu>
            </SidebarContent>
          </Sidebar>

          {/* Main content + error panel */}
          <div className="flex flex-1 flex-col overflow-hidden">
            {/* Scrollable main area */}
            <main className="flex-1 overflow-y-auto bg-background">
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
    </SidebarProvider>
  );
}
