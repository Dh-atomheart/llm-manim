import { invoke } from "@tauri-apps/api/core";
import type { AppResponse, GenerationSettings } from "./types";

export function getGenerationSettings(): Promise<
  AppResponse<GenerationSettings>
> {
  return invoke<AppResponse<GenerationSettings>>("get_generation_settings");
}

export function updateGenerationSettings(
  input: GenerationSettings,
): Promise<AppResponse<GenerationSettings>> {
  return invoke<AppResponse<GenerationSettings>>("update_generation_settings", {
    input,
  });
}
