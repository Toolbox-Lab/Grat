import { useState } from "react";
import type { DiagnosticReport } from "@/lib/types";

export function useDecode() {
  const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function decode(txHash: string) {
    setLoading(true);
    setError(null);
    try {
      setReport(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return { report, loading, error, decode };
}
