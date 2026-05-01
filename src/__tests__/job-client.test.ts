import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

import { getVideoFileUrl, openRenderArtifact } from "../commands/job";

describe("job command client", () => {
  const invokeMock = vi.mocked(invoke);

  afterEach(() => {
    invokeMock.mockReset();
  });

  it("returns backend video url directly", async () => {
    invokeMock.mockResolvedValue({
      ok: true,
      data: { url: "asset://preview.mp4" },
    });

    await expect(getVideoFileUrl("artifact-1")).resolves.toEqual({
      ok: true,
      data: { url: "asset://preview.mp4" },
    });
    expect(invokeMock).toHaveBeenCalledWith("get_video_file_url", {
      artifactId: "artifact-1",
    });
  });

  it("passes through backend command errors", async () => {
    invokeMock.mockResolvedValue({
      ok: false,
      error: {
        code: "E_ARTIFACT_INVALID",
        message: "artifact missing",
        retryable: false,
      },
    });

    await expect(
      openRenderArtifact("artifact-2", "reveal_in_folder"),
    ).resolves.toEqual({
      ok: false,
      error: {
        code: "E_ARTIFACT_INVALID",
        message: "artifact missing",
        retryable: false,
      },
    });
  });
});
