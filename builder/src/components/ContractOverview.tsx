/**
 * Contract overview dashboard showing construct counts and validation status.
 */
import React from "react";
import { Link } from "react-router-dom";
import {
  useContractStore,
  selectFacts,
  selectEntities,
  selectRules,
  selectOperations,
  selectFlows,
  selectPersonas,
  selectSources,
  selectSystems,
} from "@/store/contract";
import {
  useElaborationStore,
  selectErrorCount,
  selectWarningCount,
  selectWasmReady,
} from "@/store/elaboration";

interface ConstructSummaryCardProps {
  kind: string;
  count: number;
  href: string;
  icon: string;
  color: string;
}

function ConstructSummaryCard({
  kind,
  count,
  href,
  icon,
  color,
}: ConstructSummaryCardProps) {
  return (
    <Link
      to={href}
      className={`flex items-center justify-between rounded-lg border border-gray-200 bg-white p-4 shadow-sm transition-shadow hover:shadow-md`}
    >
      <div className="flex items-center gap-3">
        <span className={`text-2xl ${color}`}>{icon}</span>
        <div>
          <div className="text-sm font-medium text-gray-500">{kind}</div>
          <div className="text-2xl font-bold text-gray-900">{count}</div>
        </div>
      </div>
      <span className="text-gray-400 hover:text-gray-600">→</span>
    </Link>
  );
}

export function ContractOverview() {
  const bundle = useContractStore((s) => s.bundle);
  const facts = useContractStore(selectFacts);
  const entities = useContractStore(selectEntities);
  const rules = useContractStore(selectRules);
  const operations = useContractStore(selectOperations);
  const flows = useContractStore(selectFlows);
  const personas = useContractStore(selectPersonas);
  const sources = useContractStore(selectSources);
  const systems = useContractStore(selectSystems);

  const errorCount = useElaborationStore(selectErrorCount);
  const warningCount = useElaborationStore(selectWarningCount);
  const wasmReady = useElaborationStore(selectWasmReady);

  const sections: ConstructSummaryCardProps[] = [
    {
      kind: "Personas",
      count: personas.length,
      href: "/personas",
      icon: "👤",
      color: "text-purple-600",
    },
    {
      kind: "Sources",
      count: sources.length,
      href: "/sources",
      icon: "🔌",
      color: "text-indigo-600",
    },
    {
      kind: "Facts",
      count: facts.length,
      href: "/facts",
      icon: "📊",
      color: "text-blue-600",
    },
    {
      kind: "Entities",
      count: entities.length,
      href: "/entities",
      icon: "🔷",
      color: "text-cyan-600",
    },
    {
      kind: "Rules",
      count: rules.length,
      href: "/rules",
      icon: "⚖️",
      color: "text-amber-600",
    },
    {
      kind: "Operations",
      count: operations.length,
      href: "/operations",
      icon: "⚡",
      color: "text-orange-600",
    },
    {
      kind: "Flows",
      count: flows.length,
      href: "/flows",
      icon: "🔄",
      color: "text-emerald-600",
    },
    {
      kind: "Systems",
      count: systems.length,
      href: "/systems",
      icon: "🏗️",
      color: "text-teal-600",
    },
  ];

  return (
    <div className="space-y-6 p-6">
      {/* Contract header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">
            {bundle.id || "untitled"}
          </h1>
          <p className="text-sm text-gray-500">
            {bundle.constructs.length} constructs total
          </p>
        </div>
        <div className="flex items-center gap-3">
          {/* WASM status */}
          <span
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium ${
              wasmReady
                ? "bg-green-100 text-green-700"
                : "bg-gray-100 text-gray-500"
            }`}
          >
            <span
              className={`h-2 w-2 rounded-full ${
                wasmReady ? "bg-green-500" : "bg-gray-400"
              }`}
            />
            WASM {wasmReady ? "ready" : "loading..."}
          </span>

          {/* Validation status */}
          {errorCount > 0 ? (
            <span className="flex items-center gap-1.5 rounded-full bg-red-100 px-3 py-1 text-xs font-medium text-red-700">
              <span className="h-2 w-2 rounded-full bg-red-500" />
              {errorCount} error{errorCount !== 1 ? "s" : ""}
            </span>
          ) : warningCount > 0 ? (
            <span className="flex items-center gap-1.5 rounded-full bg-yellow-100 px-3 py-1 text-xs font-medium text-yellow-700">
              <span className="h-2 w-2 rounded-full bg-yellow-500" />
              {warningCount} warning{warningCount !== 1 ? "s" : ""}
            </span>
          ) : (
            <span className="flex items-center gap-1.5 rounded-full bg-green-100 px-3 py-1 text-xs font-medium text-green-700">
              <span className="h-2 w-2 rounded-full bg-green-500" />
              Valid
            </span>
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
      <div className="rounded-lg border border-gray-200 bg-white p-4">
        <h2 className="mb-3 text-sm font-semibold text-gray-700">
          Quick Add
        </h2>
        <div className="flex flex-wrap gap-2">
          {sections.map((s) => (
            <Link
              key={s.kind}
              to={`${s.href}?new=1`}
              className="rounded border border-gray-200 bg-gray-50 px-3 py-1.5 text-sm text-gray-600 hover:bg-gray-100"
            >
              + {s.kind.slice(0, -1)} {/* Remove trailing 's' */}
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
