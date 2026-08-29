import { create } from "zustand";
import type { DiagnosticReport } from "@/lib/types";

interface SessionState {
  txHash: string | null;
  report: DiagnosticReport | null;
  setTxHash: (hash: string) => void;
  setReport: (report: DiagnosticReport) => void;
  clear: () => void;
}

export const useSessionStore = create<SessionState>((set) => ({
  txHash: null,
  report: null,
  setTxHash: (hash) => set({ txHash: hash }),
  setReport: (report) => set({ report }),
  clear: () => set({ txHash: null, report: null }),
}));
