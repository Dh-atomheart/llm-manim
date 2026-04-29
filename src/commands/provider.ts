import { invoke } from "@tauri-apps/api/core";

import type {
  AppResponse,
  DeleteProviderConfigResult,
  ProviderSummary,
  ProviderTestResult,
  SaveProviderConfigInput,
  SaveProviderConfigResult,
  TestProviderConfigInput,
} from "./types";

export function listProviderConfigs(): Promise<AppResponse<ProviderSummary[]>> {
  return invoke<AppResponse<ProviderSummary[]>>("list_provider_configs");
}

export function saveProviderConfig(
  input: SaveProviderConfigInput,
): Promise<AppResponse<SaveProviderConfigResult>> {
  return invoke<AppResponse<SaveProviderConfigResult>>("save_provider_config", { input });
}

export function deleteProviderConfig(
  id: string,
): Promise<AppResponse<DeleteProviderConfigResult>> {
  return invoke<AppResponse<DeleteProviderConfigResult>>("delete_provider_config", { id });
}

export function testProviderConfig(
  input: TestProviderConfigInput,
): Promise<AppResponse<ProviderTestResult>> {
  return invoke<AppResponse<ProviderTestResult>>("test_provider_config", { input });
}