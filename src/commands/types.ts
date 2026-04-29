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
  ffmpeg: RuntimeComponentInfo;
  message: string;
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
