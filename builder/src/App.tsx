/**
 * App entry point with routing and WASM initialization.
 */
import React, { useEffect } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/Layout";
import { ContractOverview } from "./components/ContractOverview";
import { EntityEditor } from "./components/editors/EntityEditor";
import { FactEditor } from "./components/editors/FactEditor";
import { PersonaEditor } from "./components/editors/PersonaEditor";
import { SourceEditor } from "./components/editors/SourceEditor";
import { RuleEditor } from "./components/editors/RuleEditor";
import { OperationEditor } from "./components/editors/OperationEditor";
import { FlowEditor } from "./components/editors/FlowEditor";
import { SystemEditor } from "./components/editors/SystemEditor";
import { useElaborationStore } from "./store/elaboration";

// Placeholder for editors not yet implemented
function PlaceholderPage({ title }: { title: string }) {
  return (
    <div className="p-6">
      <h2 className="text-xl font-semibold text-gray-700">{title}</h2>
      <p className="mt-2 text-sm text-gray-500">
        Editor for {title.toLowerCase()} will be implemented in subsequent plans.
      </p>
    </div>
  );
}

export default function App() {
  const initWasm = useElaborationStore((s) => s.initWasm);

  // Initialize WASM evaluator on mount
  useEffect(() => {
    void initWasm();
  }, [initWasm]);

  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Layout />}>
          <Route index element={<ContractOverview />} />
          <Route path="personas" element={<PersonaEditor />} />
          <Route path="sources" element={<SourceEditor />} />
          <Route path="facts" element={<FactEditor />} />
          <Route path="entities" element={<EntityEditor />} />
          <Route path="rules" element={<RuleEditor />} />
          <Route path="operations" element={<OperationEditor />} />
          <Route path="flows" element={<FlowEditor />} />
          <Route path="systems" element={<SystemEditor />} />
          <Route
            path="simulation"
            element={<PlaceholderPage title="Simulation" />}
          />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
