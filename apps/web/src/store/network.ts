import { create } from "zustand";
import type { Network } from "@/lib/types";

interface NetworkState {
  network: Network;
  customRpcUrl: string;
  setNetwork: (network: Network) => void;
  setCustomRpcUrl: (url: string) => void;
}

export const useNetworkStore = create<NetworkState>((set) => ({
  network: "testnet",
  customRpcUrl: "",
  setNetwork: (network) => set({ network }),
  setCustomRpcUrl: (url) => set({ customRpcUrl: url }),
}));
