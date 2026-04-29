import { create } from "zustand";
import type { Project } from "../commands/types";

interface ProjectStore {
  projects: Project[];
  selectedProjectId: string | null;
  setProjects: (projects: Project[]) => void;
  selectProject: (projectId: string | null) => void;
  clear: () => void;
}

export const useProjectStore = create<ProjectStore>((set) => ({
  projects: [],
  selectedProjectId: null,
  setProjects: (projects) =>
    set((state) => {
      const hasSelectedProject =
        state.selectedProjectId !== null &&
        projects.some((project) => project.id === state.selectedProjectId);

      return {
        projects,
        selectedProjectId: hasSelectedProject
          ? state.selectedProjectId
          : (projects[0]?.id ?? null),
      };
    }),
  selectProject: (selectedProjectId) => set({ selectedProjectId }),
  clear: () => set({ projects: [], selectedProjectId: null }),
}));
