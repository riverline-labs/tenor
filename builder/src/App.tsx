/**
 * App entry point with routing and WASM initialization.
 */
import React, { useEffect } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Layout } from "./components/Layout";
import { ContractOverview } from "./components/ContractOverview";
import { useElaborationStore } from "./store/elaboration";

// Lazy placeholder pages — full editors implemented in later plans
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
          <Route
            path="personas"
            element={<PlaceholderPage title="Personas" />}
          />
          <Route
            path="sources"
            element={<PlaceholderPage title="Sources" />}
          />
          <Route path="facts" element={<PlaceholderPage title="Facts" />} />
          <Route
            path="entities"
            element={<PlaceholderPage title="Entities" />}
          />
          <Route path="rules" element={<PlaceholderPage title="Rules" />} />
          <Route
            path="operations"
            element={<PlaceholderPage title="Operations" />}
          />
          <Route path="flows" element={<PlaceholderPage title="Flows" />} />
          <Route
            path="systems"
            element={<PlaceholderPage title="Systems" />}
          />
          <Route
            path="simulation"
            element={<PlaceholderPage title="Simulation" />}
          />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
