/**
 * App entry point with routing, WASM initialization, and contract pre-loading.
 *
 * Contract pre-loading sources (checked in order):
 * 1. VITE_TENOR_CONTRACT_PATH env var — set by `tenor builder --contract <path>`
 * 2. ?contract=<url> query parameter — for URL-based pre-loading
 *
 * Pre-loading fetches the contract file, detects format (.tenor or .json),
 * imports it into the contract store, then navigates to the contract overview.
 */
import React, { useEffect, useState } from "react";
import { BrowserRouter, Routes, Route, useNavigate } from "react-router";
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
import { SimulationPage } from "./components/simulation/SimulationPage";
import { useElaborationStore } from "./store/elaboration";
import { useContractStore } from "./store/contract";
import { importInterchangeJson, importTenorFile } from "./utils/import";
import { Button } from "@/components/ui/button";

// ---------------------------------------------------------------------------
// Contract pre-loader (runs inside BrowserRouter context for useNavigate)
// ---------------------------------------------------------------------------

function ContractPreLoader({ children }: { children: React.ReactNode }) {
  const navigate = useNavigate();
  const loadBundle = useContractStore((s) => s.loadBundle);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    // Determine the contract URL/path to pre-load
    const contractUrl = resolveContractUrl();
    if (!contractUrl) return;

    setLoading(true);
    void fetchAndLoadContract(contractUrl)
      .then(() => {
        setLoading(false);
        navigate("/");
      })
      .catch((e) => {
        const msg = e instanceof Error ? e.message : String(e);
        setLoadError(`Failed to pre-load contract: ${msg}`);
        setLoading(false);
      });
  // We only want this to run once on mount, not on every navigate change
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function fetchAndLoadContract(url: string): Promise<void> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} fetching '${url}'`);
    }
    const text = await response.text();

    // Auto-detect format by URL extension or content
    const isJson =
      url.endsWith(".json") ||
      url.includes("/api/") ||
      text.trimStart().startsWith("{");

    let bundle;
    if (isJson) {
      bundle = importInterchangeJson(text);
    } else {
      bundle = await importTenorFile(text);
    }
    loadBundle(bundle);
  }

  if (loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex flex-col items-center gap-3">
          <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
          <p className="text-sm text-muted-foreground">Loading contract...</p>
        </div>
      </div>
    );
  }

  if (loadError) {
    return (
      <div className="flex h-screen items-center justify-center bg-background">
        <div className="flex max-w-md flex-col gap-3 rounded-lg border border-destructive/50 bg-card p-6">
          <h2 className="text-sm font-semibold text-destructive">
            Contract pre-load failed
          </h2>
          <pre className="whitespace-pre-wrap font-mono text-xs text-muted-foreground">{loadError}</pre>
          <Button
            variant="destructive"
            size="sm"
            onClick={() => setLoadError(null)}
            className="self-start"
          >
            Continue without pre-loading
          </Button>
        </div>
      </div>
    );
  }

  return <>{children}</>;
}

// ---------------------------------------------------------------------------
// URL resolution for pre-loading
// ---------------------------------------------------------------------------

function resolveContractUrl(): string | null {
  // 1. Check ?contract=<url> query parameter
  const params = new URLSearchParams(window.location.search);
  const queryContract = params.get("contract");
  if (queryContract) {
    return queryContract;
  }

  // 2. Check VITE_TENOR_CONTRACT_PATH (set by tenor builder --contract)
  // Vite replaces import.meta.env.VITE_* at build time; in dev it reads from .env
  const envPath = import.meta.env.VITE_TENOR_CONTRACT_PATH as string | undefined;
  if (envPath && envPath.trim() !== "") {
    // The env path is a filesystem path served by Vite.
    // In dev mode, Vite serves files relative to the project root.
    // We construct a URL to fetch it from the dev server.
    return `/${encodeURIComponent(envPath.replace(/^\//, ""))}`;
  }

  return null;
}

// ---------------------------------------------------------------------------
// App component
// ---------------------------------------------------------------------------

export default function App() {
  const initWasm = useElaborationStore((s) => s.initWasm);

  // Initialize WASM evaluator on mount
  useEffect(() => {
    void initWasm();
  }, [initWasm]);

  return (
    <BrowserRouter>
      <ContractPreLoader>
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
            <Route path="simulation" element={<SimulationPage />} />
          </Route>
        </Routes>
      </ContractPreLoader>
    </BrowserRouter>
  );
}
