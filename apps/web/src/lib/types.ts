export interface DiagnosticReport {
  error_category: string;
  error_code: number;
  error_name: string;
  summary: string;
  detailed_explanation: string;
  severity: "info" | "warning" | "error" | "fatal";
  root_causes: RootCause[];
  suggested_fixes: SuggestedFix[];
  contract_error?: ContractErrorInfo;
  transaction_context?: TransactionContext;
}

export interface RootCause {
  description: string;
  likelihood: string;
}

export interface SuggestedFix {
  description: string;
  difficulty: string;
  requires_upgrade: boolean;
  example?: string;
}

export interface ContractErrorInfo {
  contract_id: string;
  error_code: number;
  error_name?: string;
  doc_comment?: string;
}

export interface ResourceSummary {
  cpu_instructions_used: number;
  cpu_instructions_limit: number;
  memory_bytes_used: number;
  memory_bytes_limit: number;
  read_bytes: number;
  read_bytes_limit: number;
  write_bytes: number;
  write_bytes_limit: number;
}

export interface TransactionContext {
  tx_hash: string;
  ledger_sequence: number;
  function_name?: string;
  arguments: string[];
  resources: ResourceSummary;
}

export type Network = "mainnet" | "testnet" | "futurenet" | "custom";
