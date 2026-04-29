import { create } from "zustand";
import type { WorkspaceStatus } from "../commands/types";

interface WorkspaceStore {
  status: WorkspaceStatus | null;
  setStatus: (status: WorkspaceStatus | null) => void;
  clear: () => void;
}

export const useWorkspaceStore = create<WorkspaceStore>((set) => ({
  status: null,
  setStatus: (status) => set({ status }),
  clear: () => set({ status: null }),
}));
