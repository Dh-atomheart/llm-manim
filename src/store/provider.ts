import { create } from "zustand";

interface ProviderStore {
  count: number;
  lastTestStatus: "ok" | "failed" | null;
  setCount: (count: number) => void;
  setLastTestStatus: (status: "ok" | "failed") => void;
}

export const useProviderStore = create<ProviderStore>((set) => ({
  count: 0,
  lastTestStatus: null,
  setCount: (count) => set({ count }),
  setLastTestStatus: (lastTestStatus) => set({ lastTestStatus }),
}));
