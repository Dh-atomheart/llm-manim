import { invoke } from "@tauri-apps/api/core";
import type { AppResponse } from "./types";

export interface PingBackendData {
  message: string;
}

export function pingBackend(): Promise<AppResponse<PingBackendData>> {
  return invoke<AppResponse<PingBackendData>>("ping_backend");
}
