import { afterEach, describe, expect, it } from "vitest";

import { useProjectStore } from "../store/project";
import { useProviderStore } from "../store/provider";
import { useWorkspaceStore } from "../store/workspace";

describe("stores", () => {
  afterEach(() => {
    useProjectStore.getState().clear();
    useProviderStore.setState({ count: 0, lastTestStatus: null });
    useWorkspaceStore.getState().clear();
  });

  it("selects the first project when current selection disappears", () => {
    useProjectStore.getState().setProjects([
      {
        id: "project-a",
        name: "A",
        createdAt: "2026-04-30T00:00:00Z",
        updatedAt: "2026-04-30T00:00:00Z",
      },
      {
        id: "project-b",
        name: "B",
        createdAt: "2026-04-30T00:00:00Z",
        updatedAt: "2026-04-30T00:00:00Z",
      },
    ]);
    useProjectStore.getState().selectProject("project-b");

    useProjectStore.getState().setProjects([
      {
        id: "project-c",
        name: "C",
        createdAt: "2026-04-30T00:00:00Z",
        updatedAt: "2026-04-30T00:00:00Z",
      },
    ]);

    expect(useProjectStore.getState().selectedProjectId).toBe("project-c");
  });

  it("tracks provider count and last test status", () => {
    useProviderStore.getState().setCount(2);
    useProviderStore.getState().setLastTestStatus("ok");

    expect(useProviderStore.getState().count).toBe(2);
    expect(useProviderStore.getState().lastTestStatus).toBe("ok");
  });

  it("stores and clears workspace status", () => {
    useWorkspaceStore.getState().setStatus({
      configured: true,
      workspacePath: "F:/workspace",
      writable: true,
      databaseReady: true,
      runtimeStatus: "ready",
    });

    expect(useWorkspaceStore.getState().status?.workspacePath).toBe(
      "F:/workspace",
    );

    useWorkspaceStore.getState().clear();
    expect(useWorkspaceStore.getState().status).toBeNull();
  });
});
