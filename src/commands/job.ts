import { invoke } from "@tauri-apps/api/core";

import type {
  AppResponse,
  CancelJobResult,
  DeleteJobResult,
  JobLogEntry,
  OpenRenderArtifactResult,
  PromptJob,
  RenderArtifact,
  RetryJobResult,
  SubmitPromptJobResult,
  VideoFileUrlResult,
} from "./types";

export function submitPromptJob(
  projectId: string,
  providerId: string,
  promptText: string,
): Promise<AppResponse<SubmitPromptJobResult>> {
  return invoke<AppResponse<SubmitPromptJobResult>>("submit_prompt_job", {
    projectId,
    providerId,
    promptText,
  });
}

export function getJob(jobId: string): Promise<AppResponse<PromptJob>> {
  return invoke<AppResponse<PromptJob>>("get_job", { jobId });
}

export function listProjectJobs(
  projectId: string,
): Promise<AppResponse<PromptJob[]>> {
  return invoke<AppResponse<PromptJob[]>>("list_project_jobs", { projectId });
}

export function cancelJob(
  jobId: string,
): Promise<AppResponse<CancelJobResult>> {
  return invoke<AppResponse<CancelJobResult>>("cancel_job", { jobId });
}

export function deleteJob(
  jobId: string,
): Promise<AppResponse<DeleteJobResult>> {
  return invoke<AppResponse<DeleteJobResult>>("delete_job", { jobId });
}

export function retryJob(jobId: string): Promise<AppResponse<RetryJobResult>> {
  return invoke<AppResponse<RetryJobResult>>("retry_job", { jobId });
}

export function getJobLogs(jobId: string): Promise<AppResponse<JobLogEntry[]>> {
  return invoke<AppResponse<JobLogEntry[]>>("get_job_logs", { jobId });
}

export function getRenderArtifact(
  jobId: string,
): Promise<AppResponse<RenderArtifact>> {
  return invoke<AppResponse<RenderArtifact>>("get_render_artifact", { jobId });
}

export function getVideoFileUrl(
  artifactId: string,
): Promise<AppResponse<VideoFileUrlResult>> {
  return invoke<AppResponse<VideoFileUrlResult>>("get_video_file_url", {
    artifactId,
  });
}

export function openRenderArtifact(
  artifactId: string,
  mode: "open_file" | "reveal_in_folder",
): Promise<AppResponse<OpenRenderArtifactResult>> {
  return invoke<AppResponse<OpenRenderArtifactResult>>("open_render_artifact", {
    artifactId,
    mode,
  });
}
