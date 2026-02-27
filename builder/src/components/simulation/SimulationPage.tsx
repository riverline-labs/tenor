/**
 * SimulationPage: Main simulation mode compositing all simulation panels.
 *
 * Three tabs:
 *  1. Evaluate — FactInputPanel (left) + VerdictPanel (right)
 *  2. Actions  — FactInputPanel compact (left) + ActionSpacePanel (right)
 *  3. Flows    — FlowRunner (full width with DAG)
 *
 * Initializes simulation from the current contract on mount.
 */
import React, { useEffect, useState } from "react";
import { useContractStore } from "@/store/contract";
import { useElaborationStore } from "@/store/elaboration";
import { useSimulationStore } from "@/store/simulation";
import { FactInputPanel } from "./FactInputPanel";
import { VerdictPanel } from "./VerdictPanel";
import { ActionSpacePanel } from "./ActionSpacePanel";
import { FlowRunner } from "./FlowRunner";

// ---------------------------------------------------------------------------
// Tab definition
// ---------------------------------------------------------------------------

type TabId = "evaluate" | "actions" | "flows";

interface Tab {
  id: TabId;
  label: string;
}

const TABS: Tab[] = [
  { id: "evaluate", label: "Evaluate" },
  { id: "actions", label: "Actions" },
  { id: "flows", label: "Flows" },
];

// ---------------------------------------------------------------------------
// SimulationPage
// ---------------------------------------------------------------------------

export function SimulationPage() {
  const [activeTab, setActiveTab] = useState<TabId>("evaluate");
  const contractHandle = useElaborationStore((s) => s.contractHandle);
  const wasmReady = useElaborationStore((s) => s.wasmReady);
  const isValidating = useElaborationStore((s) => s.isValidating);
  const errors = useElaborationStore((s) => s.errors);
  const hasErrors = errors.some((e) => e.severity === "error");
  const bundle = useContractStore((s) => s.bundle);
  const validate = useElaborationStore((s) => s.validate);
  const initFromContract = useSimulationStore((s) => s.initFromContract);

  // Initialize simulation when contract is loaded
  useEffect(() => {
    if (contractHandle !== null) {
      initFromContract();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [contractHandle]);

  // Validate on mount if WASM is ready but no handle yet
  useEffect(() => {
    if (wasmReady && contractHandle === null && !isValidating) {
      void validate(bundle);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wasmReady]);

  const handleInit = () => {
    void validate(bundle).then(() => {
      initFromContract();
    });
  };

  return (
    <div className="flex h-full flex-col">
      {/* Top bar */}
      <div className="flex items-center justify-between border-b border-gray-200 bg-white px-4 py-2">
        <div className="flex items-center gap-3">
          <h1 className="text-base font-semibold text-gray-800">Simulation</h1>
          {/* Contract status */}
          {isValidating && (
            <span className="rounded bg-yellow-100 px-2 py-0.5 text-xs text-yellow-700">
              Validating...
            </span>
          )}
          {!isValidating && contractHandle !== null && !hasErrors && (
            <span className="rounded bg-green-100 px-2 py-0.5 text-xs text-green-700">
              Contract loaded
            </span>
          )}
          {!isValidating && hasErrors && (
            <span className="rounded bg-red-100 px-2 py-0.5 text-xs text-red-700">
              Contract has errors — fix before simulating
            </span>
          )}
          {!isValidating && contractHandle === null && !hasErrors && (
            <span className="rounded bg-gray-100 px-2 py-0.5 text-xs text-gray-500">
              Not loaded
            </span>
          )}
        </div>
        <button
          onClick={handleInit}
          disabled={isValidating}
          className="rounded border border-blue-200 bg-blue-50 px-3 py-1.5 text-sm text-blue-600 hover:bg-blue-100 disabled:opacity-40"
        >
          Init Simulation
        </button>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-gray-200 bg-white px-4">
        {TABS.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`border-b-2 px-4 py-2.5 text-sm font-medium transition-colors ${
              activeTab === tab.id
                ? "border-blue-600 text-blue-600"
                : "border-transparent text-gray-500 hover:text-gray-700"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* No contract message */}
      {contractHandle === null && !isValidating && (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <div className="text-4xl">▶</div>
          <div className="text-lg font-semibold text-gray-700">
            Contract not loaded
          </div>
          <div className="max-w-sm text-sm text-gray-500">
            Click "Init Simulation" to validate the contract and load it into
            the WASM evaluator, then fill in fact values to evaluate.
          </div>
          <button
            onClick={handleInit}
            disabled={isValidating}
            className="mt-2 rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-40"
          >
            Init Simulation
          </button>
        </div>
      )}

      {/* Tab content */}
      {contractHandle !== null && (
        <div className="flex-1 overflow-hidden">
          {/* Evaluate tab */}
          {activeTab === "evaluate" && (
            <div className="flex h-full">
              <div className="w-80 shrink-0 border-r border-gray-200">
                <FactInputPanel />
              </div>
              <div className="flex-1">
                <VerdictPanel />
              </div>
            </div>
          )}

          {/* Actions tab */}
          {activeTab === "actions" && (
            <div className="flex h-full">
              <div className="w-72 shrink-0 border-r border-gray-200">
                <FactInputPanel compact />
              </div>
              <div className="flex-1">
                <ActionSpacePanel />
              </div>
            </div>
          )}

          {/* Flows tab */}
          {activeTab === "flows" && (
            <div className="h-full">
              <FlowRunner />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
