/**
 * Collapsible error panel showing real-time validation errors from elaboration store.
 */
import React, { useState } from "react";
import { useElaborationStore } from "@/store/elaboration";
import type { ValidationError } from "@/store/elaboration";
import { Badge } from "@/components/ui/badge";
import { ChevronUp, ChevronDown } from "lucide-react";

interface ErrorPanelProps {
  onNavigateToError?: (error: ValidationError) => void;
}

export function ErrorPanel({ onNavigateToError }: ErrorPanelProps) {
  const [collapsed, setCollapsed] = useState(false);
  const errors = useElaborationStore((s) => s.errors);
  const isValidating = useElaborationStore((s) => s.isValidating);

  const errorCount = errors.filter((e) => e.severity === "error").length;
  const warningCount = errors.filter((e) => e.severity === "warning").length;

  if (errors.length === 0 && !isValidating) {
    return (
      <div className="border-t border-border bg-card px-4 py-2 text-sm text-muted-foreground">
        Contract is valid
      </div>
    );
  }

  return (
    <div className="border-t border-border bg-card">
      {/* Header */}
      <button
        onClick={() => setCollapsed((c) => !c)}
        className="flex w-full items-center justify-between px-4 py-2 text-sm font-medium hover:bg-muted"
      >
        <span className="flex items-center gap-3">
          {isValidating && (
            <span className="text-primary">Validating...</span>
          )}
          {errorCount > 0 && (
            <Badge variant="destructive" className="gap-1">
              {errorCount} error{errorCount !== 1 ? "s" : ""}
            </Badge>
          )}
          {warningCount > 0 && (
            <Badge variant="secondary" className="gap-1">
              {warningCount} warning{warningCount !== 1 ? "s" : ""}
            </Badge>
          )}
        </span>
        {collapsed ? (
          <ChevronUp className="h-4 w-4 text-muted-foreground" />
        ) : (
          <ChevronDown className="h-4 w-4 text-muted-foreground" />
        )}
      </button>

      {/* Error list */}
      {!collapsed && errors.length > 0 && (
        <div className="max-h-40 overflow-y-auto border-t border-border">
          {errors.map((error, idx) => (
            <button
              key={idx}
              onClick={() => onNavigateToError?.(error)}
              className="flex w-full items-start gap-2 px-4 py-2 text-left text-sm hover:bg-muted"
            >
              <span
                className={`mt-0.5 flex-shrink-0 ${
                  error.severity === "error"
                    ? "text-destructive"
                    : "text-muted-foreground"
                }`}
              >
                {error.severity === "error" ? "\u25CF" : "\u25CB"}
              </span>
              <span className="flex flex-col">
                {error.construct_id && (
                  <span className="font-medium text-foreground">
                    {error.construct_kind ?? "?"}: {error.construct_id}
                    {error.field ? ` \u2192 ${error.field}` : ""}
                  </span>
                )}
                <span className="text-muted-foreground">{error.message}</span>
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
