import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { ParameterSchema } from "../studio/parameterEditor";

export interface AppInfo {
  appVersion: string;
  packContractVersion: number;
  projectContractVersion: number;
  registryContractVersion: number;
}

export interface EditablePack {
  id: string;
  name: string;
  version: string;
  family: string;
  root: string;
  stylePath: string;
  styleSource: string;
  styleSha256: string;
  parameters?: ParameterSchema | null;
}

export interface ProjectSnapshot {
  root: string;
  effectsRoot: string;
  mode: "repo-bound" | "standalone";
  packs: EditablePack[];
}

export interface SaveStyleRequest {
  packRoot: string;
  expectedSha256: string;
  source: string;
}

export interface SaveStyleResult {
  status: "saved" | "conflict";
  sha256: string;
}

export type InvokeTransport = (
  command: string,
  arguments_?: Record<string, unknown>,
) => Promise<unknown>;

export interface StudioBackend {
  appInfo(): Promise<AppInfo>;
  openProject(path: string): Promise<ProjectSnapshot>;
  saveStyle(request: SaveStyleRequest): Promise<SaveStyleResult>;
}

export function createBackend(invoke: InvokeTransport): StudioBackend {
  return {
    async appInfo() {
      return (await invoke("app_info")) as AppInfo;
    },
    async openProject(path) {
      return (await invoke("open_project", { path })) as ProjectSnapshot;
    },
    async saveStyle(request) {
      return (await invoke("save_project_style", { request })) as SaveStyleResult;
    },
  };
}

const fixtureProject: ProjectSnapshot = {
  root: "/browser-fixture/future-lyrics",
  effectsRoot: "/browser-fixture/future-lyrics",
  mode: "repo-bound",
  packs: [
    {
      id: "io.github.better-lyrics.theme-sustain",
      name: "Sustain",
      version: "1.0.12",
      family: "better-lyrics",
      root: "/browser-fixture/future-lyrics/sustain",
      stylePath: "/browser-fixture/future-lyrics/sustain/theme/lyra.css",
      styleSource: ":root {\n  --lyra-font-size: 42px;\n  --lyra-glow: 18%;\n}\n",
      styleSha256: "fixture-sustain",
      parameters: {
        schemaVersion: 1,
        groups: [
          {
            id: "typography",
            label: "Typography",
            parameters: [
              { id: "font-size", label: "Font size", control: "length", binding: { cssVariable: "--lyra-font-size" }, defaultValue: 42, unit: "px", minimum: 28, maximum: 64, step: 1 },
              { id: "font-family", label: "Font family", control: "text", binding: { cssVariable: "--lyra-font-family" }, defaultValue: "Inter" },
              { id: "font-weight", label: "Font weight", control: "select", binding: { cssVariable: "--lyra-font-weight" }, defaultValue: "600", options: [{ label: "Regular", value: "400" }, { label: "Semibold", value: "600" }, { label: "Bold", value: "700" }] },
            ],
          },
          {
            id: "light",
            label: "Light & motion",
            parameters: [
              { id: "accent", label: "Accent", control: "color", binding: { cssVariable: "--lyra-accent" }, defaultValue: "#53d6d8" },
              { id: "glow", label: "Glow", control: "number", binding: { cssVariable: "--lyra-glow" }, defaultValue: 18, unit: "%", minimum: 0, maximum: 36, step: 1 },
              { id: "show-orbit", label: "Ambient orbit", control: "toggle", binding: { cssVariable: "--lyra-show-orbit" }, defaultValue: true },
            ],
          },
        ],
      },
    },
    {
      id: "io.github.chengggit.youtube-music-dynamic-theme",
      name: "Dynamic Background",
      version: "3.2.2",
      family: "better-lyrics",
      root: "/browser-fixture/future-lyrics/dynamic-background",
      stylePath: "/browser-fixture/future-lyrics/dynamic-background/theme/lyra.css",
      styleSource: ":root {\n  --lyra-motion: 0.8s;\n}\n",
      styleSha256: "fixture-dynamic",
    },
    {
      id: "io.github.snw-mint.better-lyrics-modern-player",
      name: "ModernPlayer",
      version: "1.0.1",
      family: "better-lyrics",
      root: "/browser-fixture/future-lyrics/modern-player",
      stylePath: "/browser-fixture/future-lyrics/modern-player/theme/lyra.css",
      styleSource: ":root {\n  --lyra-accent: #53d6d8;\n}\n",
      styleSha256: "fixture-modern",
    },
  ],
};

export function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in globalThis;
}

function createFixtureBackend(): StudioBackend {
  let project = structuredClone(fixtureProject);
  return {
    async appInfo() {
      return {
        appVersion: "0.1.0-alpha.1",
        packContractVersion: 1,
        projectContractVersion: 1,
        registryContractVersion: 1,
      };
    },
    async openProject() {
      return structuredClone(project);
    },
    async saveStyle(request) {
      const pack = project.packs.find((item) => item.root === request.packRoot);
      if (!pack) throw new Error("Fixture Pack not found");
      if (pack.styleSha256 !== request.expectedSha256) {
        return { status: "conflict", sha256: pack.styleSha256 };
      }
      pack.styleSource = request.source;
      pack.styleSha256 = `fixture-${request.source.length}`;
      project = structuredClone(project);
      return { status: "saved", sha256: pack.styleSha256 };
    },
  };
}

export const backend = isTauriRuntime()
  ? createBackend((command, arguments_) => tauriInvoke(command, arguments_))
  : createFixtureBackend();
