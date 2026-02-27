/**
 * Collapsible error panel showing real-time validation errors from elaboration store.
 */
import React, { useState } from "react";
import { useElaborationStore } from "@/store/elaboration";
import type { ValidationError } from "@/store/elaboration";

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
      <div className="border-t border-gray-200 bg-green-50 px-4 py-2 text-sm text-green-700">
        Contract is valid
      </div>
    );
  }

  return (
    <div className="border-t border-gray-200 bg-white">
      {/* Header */}
      <button
        onClick={() => setCollapsed((c) => !c)}
        className="flex w-full items-center justify-between px-4 py-2 text-sm font-medium hover:bg-gray-50"
      >
        <span className="flex items-center gap-3">
          {isValidating && (
            <span className="text-blue-600">Validating...</span>
          )}
          {errorCount > 0 && (
            <span className="flex items-center gap-1 text-red-600">
              <span className="inline-block h-2 w-2 rounded-full bg-red-500" />
              {errorCount} error{errorCount !== 1 ? "s" : ""}
            </span>
          )}
          {warningCount > 0 && (
            <span className="flex items-center gap-1 text-yellow-600">
              <span className="inline-block h-2 w-2 rounded-full bg-yellow-500" />
              {warningCount} warning{warningCount !== 1 ? "s" : ""}
            </span>
          )}
        </span>
        <span className="text-gray-400">{collapsed ? "▲" : "▼"}</span>
      </button>

      {/* Error list */}
      {!collapsed && errors.length > 0 && (
        <div className="max-h-40 overflow-y-auto border-t border-gray-100">
          {errors.map((error, idx) => (
            <button
              key={idx}
              onClick={() => onNavigateToError?.(error)}
              className="flex w-full items-start gap-2 px-4 py-2 text-left text-sm hover:bg-gray-50"
            >
              <span
                className={`mt-0.5 flex-shrink-0 ${
                  error.severity === "error"
                    ? "text-red-500"
                    : "text-yellow-500"
                }`}
              >
                {error.severity === "error" ? "●" : "○"}
              </span>
              <span className="flex flex-col">
                {error.construct_id && (
                  <span className="font-medium text-gray-700">
                    {error.construct_kind ?? "?"}: {error.construct_id}
                    {error.field ? ` → ${error.field}` : ""}
                  </span>
                )}
                <span className="text-gray-600">{error.message}</span>
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
