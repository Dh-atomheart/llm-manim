import { invoke } from "@tauri-apps/api/core";
import type {
  AppResponse,
  RuntimeStatus,
  WorkspaceInitResult,
  WorkspaceStatus,
} from "./types";

export function getWorkspaceStatus(): Promise<AppResponse<WorkspaceStatus>> {
  return invoke<AppResponse<WorkspaceStatus>>("get_workspace_status");
}

export function initializeWorkspace(
  workspacePath: string,
): Promise<AppResponse<WorkspaceInitResult>> {
  return invoke<AppResponse<WorkspaceInitResult>>("initialize_workspace", {
    workspacePath,
  });
}

export function checkRuntime(
  workspacePath?: string,
): Promise<AppResponse<RuntimeStatus>> {
  return invoke<AppResponse<RuntimeStatus>>("check_runtime", {
    workspacePath,
  });
}
