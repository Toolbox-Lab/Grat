"use client";

import { useState } from "react";

interface ContractErrorDetail {
  code: number;
  name: string;
  summary: string;
  description: string;
  causes: string[];
  fixes: string[];
}

const standardContractErrors: Record<number, ContractErrorDetail> = {
  0: {
    code: 0,
    name: "ContractError",
    summary: "The contract's own logic rejected this call.",
    description: "Unlike host errors, contract errors are defined by the contract author. When a contract panics or returns an error value, the host wraps it as a ContractError with code 0 or the custom code.",
    causes: [
      "Business logic assertion failure in the contract (e.g., insufficient balance, invalid state)",
      "Input validation failure — the contract rejected the provided arguments",
      "Access control check failed — the caller is not authorized for this operation"
    ],
    fixes: [
      "Use prism's contract error resolver to map the numeric code to the contract's error enum name",
      "Review the contract source code for the error enum definition to understand the specific failure"
    ]
  },
  1: {
    code: 1,
    name: "InternalError",
    summary: "An internal protocol implementation error occurred (e.g. invalid ledger state).",
    description: "The contract encountered an internal error during execution, indicating a protocol implementation issue or invalid ledger state.",
    causes: [
      "Internal validation or state consistency checks failed in the contract"
    ],
    fixes: [
      "Check the diagnostic logs to see if there are internal contract assertion failures"
    ]
  },
  2: {
    code: 2,
    name: "OperationNotSupportedError",
    summary: "The operation is not supported (e.g. calling clawback on an asset without clawback enabled).",
    description: "The contract attempted an operation that is unsupported by the contract configuration or type (e.g., performing a clawback on an asset when clawback is disabled).",
    causes: [
      "Invoking clawback on a non-clawbackable asset"
    ],
    fixes: [
      "Check the asset flags and ensure the operation is allowed for the target asset"
    ]
  },
  3: {
    code: 3,
    name: "AlreadyInitializedError",
    summary: "The contract instance has already been initialized and cannot be re-initialized.",
    description: "The contract instance was initialized twice. Soroban contracts typically only allow initialization once.",
    causes: [
      "Calling the initialize function on an already initialized contract instance"
    ],
    fixes: [
      "Ensure that initialization is only called once per contract deployment"
    ]
  },
  6: {
    code: 6,
    name: "AccountMissingError",
    summary: "An account involved in the transaction does not exist on the network.",
    description: "The operation required a specific account to exist on the network, but the account could not be found.",
    causes: [
      "Providing an invalid account address or an account that has not been created/funded"
    ],
    fixes: [
      "Verify that the addresses provided exist and are active on the target network"
    ]
  }
};

export default function ContractErrorView() {
  const [selectedCode, setSelectedCode] = useState<number>(0);
  const [customCode, setCustomCode] = useState<string>("");
  const [isCustomMode, setIsCustomMode] = useState<boolean>(false);

  const activeError = isCustomMode
    ? standardContractErrors[parseInt(customCode)]
    : standardContractErrors[selectedCode];

  const handleCustomCodeChange = (val: string) => {
    setCustomCode(val);
    setIsCustomMode(val.trim() !== "");
  };

  return (
    <div className="bg-slate-900 border border-slate-800 rounded-xl p-6 shadow-xl max-w-3xl mx-auto my-6 text-slate-100">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-bold tracking-tight text-violet-400">
          📋 Contract Error Lookup
        </h2>
        <span className="text-xs px-2.5 py-1 rounded-full bg-violet-950/50 border border-violet-800/50 text-violet-300 font-mono">
          HostError::Contract
        </span>
      </div>

      <p className="text-sm text-slate-400 mb-6">
        Soroban smart contracts return numeric codes when they revert. Select a standard code or enter a custom code to view its details.
      </p>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
        <div>
          <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
            Select Standard Code
          </label>
          <select
            value={selectedCode}
            onChange={(e) => {
              setSelectedCode(Number(e.target.value));
              setIsCustomMode(false);
              setCustomCode("");
            }}
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200 focus:outline-none focus:border-violet-500 transition-colors"
          >
            {Object.values(standardContractErrors).map((err) => (
              <option key={err.code} value={err.code}>
                Code {err.code}: {err.name}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
            Or Type Any Code
          </label>
          <input
            type="number"
            value={customCode}
            onChange={(e) => handleCustomCodeChange(e.target.value)}
            placeholder="e.g. 42"
            className="w-full bg-slate-950 border border-slate-800 rounded-lg px-3 py-2 text-slate-200 placeholder-slate-600 focus:outline-none focus:border-violet-500 transition-colors"
          />
        </div>
      </div>

      {activeError ? (
        <div className="border-t border-slate-800 pt-6">
          <div className="flex items-start justify-between mb-4">
            <div>
              <span className="text-xs font-bold text-violet-400 uppercase tracking-wider">
                Category: Contract
              </span>
              <h3 className="text-lg font-bold text-slate-100 flex items-center gap-2 mt-1">
                {activeError.name}
                <span className="text-sm font-mono text-slate-500 bg-slate-950 px-2 py-0.5 rounded border border-slate-800">
                  Code {activeError.code}
                </span>
              </h3>
            </div>
            <span className="text-xs px-2.5 py-1 rounded bg-red-950/40 border border-red-900/50 text-red-400 font-medium">
              Error
            </span>
          </div>

          <div className="bg-slate-950 rounded-lg p-4 border border-slate-800 mb-4">
            <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-1">
              Summary
            </h4>
            <p className="text-slate-300 text-sm">{activeError.summary}</p>
          </div>

          <div className="mb-4">
            <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-1">
              Detailed Explanation
            </h4>
            <p className="text-slate-400 text-sm leading-relaxed">
              {activeError.description}
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
            <div>
              <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-2">
                Common Causes
              </h4>
              <ul className="space-y-2">
                {activeError.causes.map((cause, i) => (
                  <li key={i} className="text-sm text-slate-300 flex items-start gap-2">
                    <span className="text-red-400 mt-1">•</span>
                    <span>{cause}</span>
                  </li>
                ))}
              </ul>
            </div>

            <div>
              <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-2">
                Suggested Fixes
              </h4>
              <ul className="space-y-2">
                {activeError.fixes.map((fix, i) => (
                  <li key={i} className="text-sm text-slate-300 flex items-start gap-2">
                    <span className="text-emerald-400 mt-1">✓</span>
                    <span>{fix}</span>
                  </li>
                ))}
              </ul>
            </div>
          </div>
        </div>
      ) : isCustomMode && customCode.trim() !== "" ? (
        <div className="border-t border-slate-800 pt-6">
          <div className="flex items-start justify-between mb-4">
            <div>
              <span className="text-xs font-bold text-amber-400 uppercase tracking-wider">
                Category: Contract (Custom)
              </span>
              <h3 className="text-lg font-bold text-slate-100 flex items-center gap-2 mt-1">
                CustomContractError
                <span className="text-sm font-mono text-slate-500 bg-slate-950 px-2 py-0.5 rounded border border-slate-800">
                  Code {customCode}
                </span>
              </h3>
            </div>
            <span className="text-xs px-2.5 py-1 rounded bg-red-950/40 border border-red-900/50 text-red-400 font-medium">
              Error
            </span>
          </div>

          <div className="bg-slate-950 rounded-lg p-4 border border-slate-800 mb-4">
            <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-1">
              Summary
            </h4>
            <p className="text-slate-300 text-sm">
              The contract reverted with custom error code {customCode}.
            </p>
          </div>

          <div className="mb-4">
            <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-1">
              Detailed Explanation
            </h4>
            <p className="text-slate-400 text-sm leading-relaxed">
              This is a custom contract-defined error code. Custom error codes are defined by the contract author inside their `#[contracterror]` enum. To map this code to its name, you need the contract's WASM specification or source code.
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
            <div>
              <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-2">
                Common Causes
              </h4>
              <ul className="space-y-2">
                <li className="text-sm text-slate-300 flex items-start gap-2">
                  <span className="text-red-400 mt-1">•</span>
                  <span>A business logic requirement or assertion in the contract was not met.</span>
                </li>
              </ul>
            </div>

            <div>
              <h4 className="text-xs font-bold text-slate-500 uppercase tracking-wider mb-2">
                Suggested Fixes
              </h4>
              <ul className="space-y-2">
                <li className="text-sm text-slate-300 flex items-start gap-2">
                  <span className="text-emerald-400 mt-1">✓</span>
                  <span>Locate the source code of the contract and find its `#[contracterror]` enum.</span>
                </li>
                <li className="text-sm text-slate-300 flex items-start gap-2">
                  <span className="text-emerald-400 mt-1">✓</span>
                  <span>Match the numeric code {customCode} with the integer values assigned to the enum variants.</span>
                </li>
              </ul>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
