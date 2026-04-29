import { invoke } from "@tauri-apps/api/core";
import type { AppResponse, EmptyData, Project } from "./types";

export function createProject(name: string): Promise<AppResponse<Project>> {
  return invoke<AppResponse<Project>>("create_project", { name });
}

export function listProjects(): Promise<AppResponse<Project[]>> {
  return invoke<AppResponse<Project[]>>("list_projects");
}

export function deleteProject(id: string): Promise<AppResponse<EmptyData>> {
  return invoke<AppResponse<EmptyData>>("delete_project", { id });
}
