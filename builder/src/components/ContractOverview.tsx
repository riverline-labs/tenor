/**
 * Contract overview dashboard showing construct counts and validation status.
 */
import React from "react";
import { Link } from "react-router";
import {
  useContractStore,
  useFacts,
  useEntities,
  useRules,
  useOperations,
  useFlows,
  usePersonas,
  useSources,
  useSystems,
} from "@/store/contract";
import {
  useElaborationStore,
  selectErrorCount,
  selectWarningCount,
  selectWasmReady,
} from "@/store/elaboration";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Users,
  Plug,
  BarChart3,
  Diamond,
  Scale,
  Zap,
  RefreshCw,
  Building2,
  ArrowRight,
  Plus,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

interface ConstructSummaryCardProps {
  kind: string;
  count: number;
  href: string;
  icon: LucideIcon;
}

function ConstructSummaryCard({
  kind,
  count,
  href,
  icon: Icon,
}: ConstructSummaryCardProps) {
  return (
    <Link to={href}>
      <Card className="transition-shadow hover:shadow-md">
        <CardContent className="flex items-center justify-between p-4">
          <div className="flex items-center gap-3">
            <Icon className="h-6 w-6 text-primary" />
            <div>
              <div className="text-sm font-medium text-muted-foreground">{kind}</div>
              <div className="text-2xl font-bold text-foreground">{count}</div>
            </div>
          </div>
          <ArrowRight className="h-4 w-4 text-muted-foreground" />
        </CardContent>
      </Card>
    </Link>
  );
}

export function ContractOverview() {
  const bundle = useContractStore((s) => s.bundle);
  const facts = useFacts();
  const entities = useEntities();
  const rules = useRules();
  const operations = useOperations();
  const flows = useFlows();
  const personas = usePersonas();
  const sources = useSources();
  const systems = useSystems();

  const errorCount = useElaborationStore(selectErrorCount);
  const warningCount = useElaborationStore(selectWarningCount);
  const wasmReady = useElaborationStore(selectWasmReady);

  const sections: ConstructSummaryCardProps[] = [
    { kind: "Personas", count: personas.length, href: "/personas", icon: Users },
    { kind: "Sources", count: sources.length, href: "/sources", icon: Plug },
    { kind: "Facts", count: facts.length, href: "/facts", icon: BarChart3 },
    { kind: "Entities", count: entities.length, href: "/entities", icon: Diamond },
    { kind: "Rules", count: rules.length, href: "/rules", icon: Scale },
    { kind: "Operations", count: operations.length, href: "/operations", icon: Zap },
    { kind: "Flows", count: flows.length, href: "/flows", icon: RefreshCw },
    { kind: "Systems", count: systems.length, href: "/systems", icon: Building2 },
  ];

  return (
    <div className="space-y-6 p-6">
      {/* Contract header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-foreground">
            {bundle.id || "untitled"}
          </h1>
          <p className="text-sm text-muted-foreground">
            {bundle.constructs.length} constructs total
          </p>
        </div>
        <div className="flex items-center gap-3">
          {/* WASM status */}
          <Badge variant={wasmReady ? "default" : "secondary"}>
            <span
              className={`mr-1.5 inline-block h-2 w-2 rounded-full ${
                wasmReady ? "bg-primary-foreground" : "bg-muted-foreground"
              }`}
            />
            WASM {wasmReady ? "ready" : "loading..."}
          </Badge>

          {/* Validation status */}
          {errorCount > 0 ? (
            <Badge variant="destructive">
              <span className="mr-1.5 inline-block h-2 w-2 rounded-full bg-destructive-foreground" />
              {errorCount} error{errorCount !== 1 ? "s" : ""}
            </Badge>
          ) : warningCount > 0 ? (
            <Badge variant="secondary">
              <span className="mr-1.5 inline-block h-2 w-2 rounded-full bg-muted-foreground" />
              {warningCount} warning{warningCount !== 1 ? "s" : ""}
            </Badge>
          ) : (
            <Badge variant="default">
              <span className="mr-1.5 inline-block h-2 w-2 rounded-full bg-primary-foreground" />
              Valid
            </Badge>
          )}
        </div>
      </div>

      {/* Construct summary grid */}
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-4">
        {sections.map((section) => (
          <ConstructSummaryCard key={section.kind} {...section} />
        ))}
      </div>

      {/* Quick actions */}
      <Card>
        <CardContent className="p-4">
          <h2 className="mb-3 text-sm font-semibold text-foreground">
            Quick Add
          </h2>
          <div className="flex flex-wrap gap-2">
            {sections.map((s) => (
              <Button key={s.kind} variant="outline" size="sm" asChild>
                <Link to={`${s.href}?new=1`}>
                  <Plus className="mr-1 h-3 w-3" />
                  {s.kind === "Entities" ? "Entity" : s.kind.slice(0, -1)}
                </Link>
              </Button>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
