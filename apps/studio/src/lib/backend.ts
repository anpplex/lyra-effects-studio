import { invoke as tauriInvoke } from "@tauri-apps/api/core";

export interface AppInfo {
  appVersion: string;
  packContractVersion: number;
  projectContractVersion: number;
  registryContractVersion: number;
}

export type InvokeTransport = (
  command: string,
  arguments_?: Record<string, unknown>,
) => Promise<unknown>;

export interface StudioBackend {
  appInfo(): Promise<AppInfo>;
}

export function createBackend(invoke: InvokeTransport): StudioBackend {
  return {
    async appInfo() {
      return (await invoke("app_info")) as AppInfo;
    },
  };
}

export const backend = createBackend((command, arguments_) =>
  tauriInvoke(command, arguments_),
);
