/**
 * AuthorityMatrix: persona x operation authorization grid.
 *
 * Displays a table where:
 * - Rows = personas
 * - Columns = operations
 * - Cells = checkmark if persona is in operation's allowed_personas
 *
 * Features:
 * - Green cell = authorized, gray = unauthorized
 * - Click cell to toggle authorization (dispatches to contract store)
 * - Summary row: total authorized operations per persona
 * - Summary column: total authorized personas per operation
 * - Warning highlight: personas with no operations (empty row),
 *   operations with no personas (empty column)
 */
import React from "react";
import { useContractStore } from "@/store/contract";
import type { PersonaConstruct, OperationConstruct } from "@/types/interchange";

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface AuthorityMatrixProps {
  personas: PersonaConstruct[];
  operations: OperationConstruct[];
}

// ---------------------------------------------------------------------------
// AuthorityMatrix (main export)
// ---------------------------------------------------------------------------

export function AuthorityMatrix({ personas, operations }: AuthorityMatrixProps) {
  const updateConstruct = useContractStore((s) => s.updateConstruct);

  function isAuthorized(persona: PersonaConstruct, op: OperationConstruct): boolean {
    return op.allowed_personas.includes(persona.id);
  }

  function toggleAuthorization(persona: PersonaConstruct, op: OperationConstruct) {
    const current = op.allowed_personas;
    const next = isAuthorized(persona, op)
      ? current.filter((p) => p !== persona.id)
      : [...current, persona.id];
    updateConstruct(op.id, "Operation", { allowed_personas: next });
  }

  if (personas.length === 0 || operations.length === 0) {
    return (
      <div className="rounded border-2 border-dashed border-gray-200 py-6 text-center text-xs text-gray-400">
        {personas.length === 0 && operations.length === 0
          ? "No personas or operations defined"
          : personas.length === 0
          ? "No personas defined — add personas to populate the matrix"
          : "No operations defined — add operations to populate the matrix"}
      </div>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs">
        <thead>
          <tr>
            {/* Corner cell */}
            <th className="w-32 border-b border-r border-gray-200 bg-gray-50 px-2 py-1.5 text-left text-xs font-semibold text-gray-600">
              Persona \ Op
            </th>
            {operations.map((op) => {
              const count = personas.filter((p) =>
                op.allowed_personas.includes(p.id)
              ).length;
              const isEmpty = count === 0;
              return (
                <th
                  key={op.id}
                  className={`border-b border-l border-gray-200 px-2 py-1.5 text-center font-mono font-medium ${
                    isEmpty
                      ? "bg-amber-50 text-amber-700"
                      : "bg-gray-50 text-gray-700"
                  }`}
                  title={isEmpty ? "No personas authorized — operation is unreachable" : op.id}
                >
                  <div className="max-w-24 truncate">{op.id}</div>
                  {isEmpty && (
                    <div className="text-xs font-normal text-amber-600">
                      no personas
                    </div>
                  )}
                </th>
              );
            })}
            {/* Summary column header */}
            <th className="border-b border-l border-gray-200 bg-gray-100 px-2 py-1.5 text-center font-semibold text-gray-600">
              Total ops
            </th>
          </tr>
        </thead>

        <tbody>
          {personas.map((persona) => {
            const authorizedCount = operations.filter((op) =>
              op.allowed_personas.includes(persona.id)
            ).length;
            const isEmpty = authorizedCount === 0;

            return (
              <tr
                key={persona.id}
                className={isEmpty ? "bg-amber-50" : "hover:bg-gray-50"}
              >
                {/* Persona label */}
                <td
                  className={`border-b border-r border-gray-200 px-2 py-1.5 font-mono font-medium ${
                    isEmpty ? "text-amber-700" : "text-gray-700"
                  }`}
                  title={isEmpty ? "Persona has no authorized operations" : persona.id}
                >
                  <div className="flex items-center gap-1">
                    <span className="truncate max-w-28">{persona.id}</span>
                    {isEmpty && (
                      <span className="rounded bg-amber-100 px-1 py-0.5 text-xs text-amber-600">
                        unused
                      </span>
                    )}
                  </div>
                </td>

                {/* Authorization cells */}
                {operations.map((op) => {
                  const authorized = isAuthorized(persona, op);
                  return (
                    <td
                      key={op.id}
                      className={`border-b border-l border-gray-200 px-2 py-1.5 text-center ${
                        authorized
                          ? "bg-green-50"
                          : "bg-gray-50 hover:bg-gray-100"
                      }`}
                    >
                      <button
                        onClick={() => toggleAuthorization(persona, op)}
                        className={`flex h-6 w-6 items-center justify-center rounded transition-colors mx-auto ${
                          authorized
                            ? "bg-green-500 text-white hover:bg-green-600"
                            : "border border-gray-300 text-gray-300 hover:border-green-400 hover:text-green-500"
                        }`}
                        title={
                          authorized
                            ? `Remove ${persona.id} from ${op.id}`
                            : `Authorize ${persona.id} for ${op.id}`
                        }
                        aria-label={
                          authorized ? "Authorized — click to remove" : "Not authorized — click to add"
                        }
                      >
                        {authorized ? "✓" : ""}
                      </button>
                    </td>
                  );
                })}

                {/* Summary: total ops count */}
                <td
                  className={`border-b border-l border-gray-200 px-2 py-1.5 text-center font-semibold ${
                    isEmpty
                      ? "bg-amber-50 text-amber-700"
                      : "bg-gray-100 text-gray-700"
                  }`}
                >
                  {authorizedCount}
                </td>
              </tr>
            );
          })}

          {/* Summary row: total personas per operation */}
          <tr className="bg-gray-100">
            <td className="border-t border-r border-gray-200 px-2 py-1.5 text-xs font-semibold text-gray-600">
              Total personas
            </td>
            {operations.map((op) => {
              const count = personas.filter((p) =>
                op.allowed_personas.includes(p.id)
              ).length;
              return (
                <td
                  key={op.id}
                  className={`border-t border-l border-gray-200 px-2 py-1.5 text-center font-semibold ${
                    count === 0 ? "text-amber-700" : "text-gray-700"
                  }`}
                >
                  {count}
                </td>
              );
            })}
            {/* Grand total cell */}
            <td className="border-t border-l border-gray-200 px-2 py-1.5 text-center font-semibold text-gray-500">
              —
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
