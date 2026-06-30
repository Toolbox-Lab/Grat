"use client";

import type { DiagnosticReport } from "@/lib/types";

interface ErrorCardProps {
  report: DiagnosticReport | null;
}

export default function ErrorCard({ report }: ErrorCardProps) {
  if (!report) {
    return <div>{/* Decoded error with root cause */}</div>;
  }

  return (
    <div>
      {report.failing_contract_id && (
        <FailingContractBadge contractId={report.failing_contract_id} />
      )}
    </div>
  );
}

function FailingContractBadge({ contractId }: { contractId: string }) {
  const copy = () => {
    navigator.clipboard.writeText(contractId).catch(() => {});
  };

  return (
    <div className="flex items-center gap-2 rounded-md border border-red-400/50 bg-red-950/40 px-3 py-2 text-sm">
      <span className="font-medium text-red-300">Failing Contract:</span>
      <code className="truncate font-mono text-red-200">{contractId}</code>
      <button
        type="button"
        onClick={copy}
        className="shrink-0 rounded px-1.5 py-0.5 text-xs text-red-300 hover:bg-red-800/50 hover:text-red-100"
        title="Copy contract ID"
      >
        Copy
      </button>
    </div>
  );
}
