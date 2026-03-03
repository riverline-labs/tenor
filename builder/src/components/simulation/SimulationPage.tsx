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
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

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
      <div className="flex items-center justify-between border-b border-border bg-card px-4 py-2">
        <div className="flex items-center gap-3">
          <h1 className="text-base font-semibold text-foreground">Simulation</h1>
          {/* Contract status */}
          {isValidating && (
            <Badge variant="secondary">
              Validating...
            </Badge>
          )}
          {!isValidating && contractHandle !== null && !hasErrors && (
            <Badge variant="secondary" className="bg-primary/10 text-primary">
              Contract loaded
            </Badge>
          )}
          {!isValidating && hasErrors && (
            <Badge variant="destructive">
              Contract has errors — fix before simulating
            </Badge>
          )}
          {!isValidating && contractHandle === null && !hasErrors && (
            <Badge variant="secondary">
              Not loaded
            </Badge>
          )}
        </div>
        <Button
          onClick={handleInit}
          disabled={isValidating}
          variant="outline"
          size="sm"
        >
          Init Simulation
        </Button>
      </div>

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={(v) => setActiveTab(v as TabId)} className="flex h-full flex-col gap-0">
        <div className="border-b border-border bg-card px-4">
          <TabsList variant="line">
            {TABS.map((tab) => (
              <TabsTrigger key={tab.id} value={tab.id}>
                {tab.label}
              </TabsTrigger>
            ))}
          </TabsList>
        </div>

        {/* No contract message */}
        {contractHandle === null && !isValidating && (
          <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
            <div className="text-4xl">▶</div>
            <div className="text-lg font-semibold text-foreground">
              Contract not loaded
            </div>
            <div className="max-w-sm text-sm text-muted-foreground">
              Click "Init Simulation" to validate the contract and load it into
              the WASM evaluator, then fill in fact values to evaluate.
            </div>
            <Button
              onClick={handleInit}
              disabled={isValidating}
              size="sm"
              className="mt-2"
            >
              Init Simulation
            </Button>
          </div>
        )}

        {/* Tab content */}
        {contractHandle !== null && (
          <div className="flex-1 overflow-hidden">
            {/* Evaluate tab */}
            <TabsContent value="evaluate" className="h-full">
              <div className="flex h-full">
                <div className="w-80 shrink-0 border-r border-border">
                  <FactInputPanel />
                </div>
                <div className="flex-1">
                  <VerdictPanel />
                </div>
              </div>
            </TabsContent>

            {/* Actions tab */}
            <TabsContent value="actions" className="h-full">
              <div className="flex h-full">
                <div className="w-72 shrink-0 border-r border-border">
                  <FactInputPanel compact />
                </div>
                <div className="flex-1">
                  <ActionSpacePanel />
                </div>
              </div>
            </TabsContent>

            {/* Flows tab */}
            <TabsContent value="flows" className="h-full">
              <div className="h-full">
                <FlowRunner />
              </div>
            </TabsContent>
          </div>
        )}
      </Tabs>
    </div>
  );
}
