export interface AppError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
  retryable: boolean;
}

export type RuntimeComponentStatus = "ok" | "missing";

export interface RuntimeComponentInfo {
  status: RuntimeComponentStatus;
  version?: string;
}

export interface WorkspaceStatus {
  configured: boolean;
  workspacePath?: string;
  writable: boolean;
  databaseReady: boolean;
  runtimeStatus: "ready" | "broken" | "missing";
}

export interface WorkspaceInitResult {
  workspacePath: string;
  created: boolean;
  databaseReady: boolean;
}

export interface RuntimeStatus {
  status: "ready" | "broken" | "missing";
  python: RuntimeComponentInfo;
  uv: RuntimeComponentInfo;
  manim: RuntimeComponentInfo;
  uvManim: RuntimeComponentInfo;
  ffmpeg: RuntimeComponentInfo;
  ffprobe: RuntimeComponentInfo;
  latex: RuntimeComponentInfo;
  dvisvgm: RuntimeComponentInfo;
  errorCode?: string;
  message: string;
}

export interface GenerationSettings {
  strictApiNameValidation: boolean;
}

export interface Project {
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
}

export type ProviderType = "openai_compatible" | "anthropic_compatible";

export interface ProviderSummary {
  id: string;
  name: string;
  providerType: ProviderType;
  baseUrl: string;
  model: string;
  createdAt: string;
  updatedAt: string;
}

export interface SaveProviderConfigInput {
  id?: string;
  name: string;
  providerType: ProviderType;
  baseUrl: string;
  model: string;
  apiKey?: string;
}

export interface SaveProviderConfigResult {
  id: string;
}

export interface DeleteProviderConfigResult {
  deleted: boolean;
}

export interface TestProviderConfigInput {
  id?: string;
  providerType?: ProviderType;
  baseUrl?: string;
  model?: string;
  apiKey?: string;
}

export interface ProviderTestResult {
  reachable: boolean;
  modelAccepted: boolean;
  message: string;
}

export type JobState =
  | "queued"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled";

export type JobLogStage =
  | "workspace"
  | "provider"
  | "prompt"
  | "llm"
  | "parse"
  | "static_check"
  | "queue"
  | "render"
  | "artifact"
  | "runtime"
  | "user_action"
  | "security";

export interface PromptJob {
  id: string;
  projectId: string;
  providerId: string;
  promptText: string;
  state: JobState;
  errorCode?: string;
  errorSummary?: string;
  suggestion?: string;
  retryOfJobId?: string;
  createdAt: string;
  startedAt?: string;
  finishedAt?: string;
}

export interface SubmitPromptJobResult {
  jobId: string;
  state: JobState;
}

export interface CancelJobResult {
  jobId: string;
  state: JobState;
}

export interface DeleteJobResult {
  deleted: boolean;
}

export interface RetryJobResult {
  jobId: string;
  state: JobState;
  retryOfJobId: string;
}

export interface JobLogEntry {
  id: string;
  stage: JobLogStage;
  level: "debug" | "info" | "warn" | "error";
  message: string;
  timestamp: string;
}

export interface RenderArtifact {
  id: string;
  jobId: string;
  projectId: string;
  filePath: string;
  durationSecs: number;
  fileSizeBytes: number;
  createdAt: string;
}

export interface VideoFileUrlResult {
  url: string;
}

export interface OpenRenderArtifactResult {
  opened: boolean;
}

export type EmptyData = Record<string, never>;

export type AppResponse<T> =
  | {
      ok: true;
      data: T;
      error?: never;
    }
  | {
      ok: false;
      error: AppError;
      data?: never;
    };
