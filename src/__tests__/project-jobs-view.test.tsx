import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { PromptJob } from "../commands/types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

import ProjectJobsView from "../views/ProjectJobsView";

const invokeMock = vi.mocked(invoke);

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean })
  .IS_REACT_ACT_ENVIRONMENT = true;

function makeJob(index: number): PromptJob {
  return {
    id: `job_${index}`,
    projectId: "project_1",
    providerId: "provider_1",
    promptText: `Prompt ${index}`,
    state: "failed",
    errorCode: "E_RENDER_FAIL",
    createdAt: `2026-05-01T00:0${index}:00.000Z`,
  };
}

function renderView() {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);

  act(() => {
    root.render(
      <ProjectJobsView
        mode="history"
        projectId="project_1"
        projectName="Project"
        runtimeStatus="ready"
        onProviderChange={() => undefined}
        onOpenProviderSettings={() => undefined}
      />,
    );
  });

  return { host, root };
}

async function waitForText(text: string) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (document.body.textContent?.includes(text)) {
      return;
    }
    await act(async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 0));
    });
  }
  throw new Error(`Timed out waiting for text: ${text}`);
}

function buttonNamed(name: string): HTMLButtonElement {
  const button = Array.from(document.querySelectorAll("button")).find(
    (item) => item.textContent?.trim() === name,
  );
  if (!button) {
    throw new Error(`Button not found: ${name}`);
  }
  return button;
}

describe("ProjectJobsView job list", () => {
  let root: Root | null = null;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    root = null;
    document.body.innerHTML = "";
    invokeMock.mockReset();
  });

  it("paginates jobs in pages of five", async () => {
    const jobs = Array.from({ length: 6 }, (_, index) => makeJob(index + 1));
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_provider_configs") {
        return {
          ok: true,
          data: [
            {
              id: "provider_1",
              name: "Provider",
              providerType: "openai_compatible",
              baseUrl: "https://api.example.com",
              model: "mock",
              createdAt: "2026-05-01T00:00:00.000Z",
              updatedAt: "2026-05-01T00:00:00.000Z",
            },
          ],
        };
      }
      if (command === "list_project_jobs") {
        return { ok: true, data: jobs };
      }
      if (command === "get_job_logs") {
        return { ok: true, data: [] };
      }
      return { ok: true, data: {} };
    });

    ({ root } = renderView());

    await waitForText("第 1 / 2 页");
    expect(document.body.textContent).toContain("Prompt 6");
    expect(document.body.textContent).not.toContain("Prompt 1");

    act(() => {
      buttonNamed("下一页").click();
    });

    await waitForText("第 2 / 2 页");
    expect(document.body.textContent).toContain("Prompt 1");
  });

  it("deletes the selected job and selects the next visible job", async () => {
    const jobs = [makeJob(3), makeJob(2), makeJob(1)];
    invokeMock.mockImplementation(async (command) => {
      if (command === "list_provider_configs") {
        return {
          ok: true,
          data: [
            {
              id: "provider_1",
              name: "Provider",
              providerType: "openai_compatible",
              baseUrl: "https://api.example.com",
              model: "mock",
              createdAt: "2026-05-01T00:00:00.000Z",
              updatedAt: "2026-05-01T00:00:00.000Z",
            },
          ],
        };
      }
      if (command === "list_project_jobs") {
        return { ok: true, data: jobs };
      }
      if (command === "delete_job") {
        return { ok: true, data: { deleted: true } };
      }
      if (command === "get_job_logs") {
        return { ok: true, data: [] };
      }
      return { ok: true, data: {} };
    });

    ({ root } = renderView());

    await waitForText("Prompt 3");

    act(() => {
      buttonNamed("删除").click();
    });

    await waitForText("任务已删除。");
    expect(document.body.textContent).not.toContain("Prompt 3");
    expect(document.body.textContent).toContain("Prompt 2");
    expect(invokeMock).toHaveBeenCalledWith("delete_job", { jobId: "job_3" });
  });
});
