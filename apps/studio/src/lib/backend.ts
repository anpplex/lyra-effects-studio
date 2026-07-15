import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import type { ParameterSchema } from "../studio/parameterEditor";
import type { PreviewScenario } from "../studio/previewDocument";

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
  scenarios?: PreviewScenario[];
  documents?: EditableDocument[];
}

export interface EditableDocument {
  id: string;
  label: string;
  kind: "css" | "html" | "json" | "javascript";
  path: string;
  relativePath: string;
  source: string;
  sha256: string;
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

export interface SaveDocumentRequest {
  packRoot: string;
  documentPath: string;
  expectedSha256: string;
  source: string;
}

export interface SaveStyleResult {
  status: "saved" | "conflict";
  sha256: string;
}

export type DeviceBridgeState = "stopped" | "waiting" | "connected";

export interface DeviceBridgeSession {
  deviceProfileId: string;
  protocolVersion: string;
  capabilities: string[];
}

export interface DeviceBridgeStatus {
  state: DeviceBridgeState;
  session: DeviceBridgeSession | null;
}

export type AdbPreflightReadiness =
  | "unconfigured"
  | "notChecked"
  | "noReadyDevice"
  | "oneReadyDevice"
  | "multipleReadyDevices"
  | "error";

export interface AdbPreflightStatus {
  configured: boolean;
  readiness: AdbPreflightReadiness;
}

export type DevBridgeMappingReadiness =
  | "inactive"
  | "enabling"
  | "active"
  | "removing"
  | "cleanupFailed";

export interface DevBridgeMappingStatus {
  readiness: DevBridgeMappingReadiness;
}

export type InvokeTransport = (
  command: string,
  arguments_?: Record<string, unknown>,
) => Promise<unknown>;

export interface StudioBackend {
  appInfo(): Promise<AppInfo>;
  deviceBridgeStatus(): Promise<DeviceBridgeStatus>;
  startDeviceBridge(): Promise<DeviceBridgeStatus>;
  stopDeviceBridge(): Promise<DeviceBridgeStatus>;
  deviceBridgeAdbStatus(): Promise<AdbPreflightStatus>;
  chooseDeviceBridgeAdbExecutable(): Promise<AdbPreflightStatus>;
  checkDeviceBridgeAdb(): Promise<AdbPreflightStatus>;
  deviceBridgeMappingStatus(): Promise<DevBridgeMappingStatus>;
  enableDeviceBridgeMapping(): Promise<DevBridgeMappingStatus>;
  disableDeviceBridgeMapping(): Promise<DevBridgeMappingStatus>;
  openProject(path: string): Promise<ProjectSnapshot>;
  saveStyle(request: SaveStyleRequest): Promise<SaveStyleResult>;
  saveDocument(request: SaveDocumentRequest): Promise<SaveStyleResult>;
}

export function createBackend(invoke: InvokeTransport): StudioBackend {
  return {
    async appInfo() {
      return (await invoke("app_info")) as AppInfo;
    },
    async deviceBridgeStatus() {
      return (await invoke("get_device_bridge_status")) as DeviceBridgeStatus;
    },
    async startDeviceBridge() {
      return (await invoke("start_device_bridge")) as DeviceBridgeStatus;
    },
    async stopDeviceBridge() {
      return (await invoke("stop_device_bridge")) as DeviceBridgeStatus;
    },
    async deviceBridgeAdbStatus() {
      return (await invoke("get_device_bridge_adb_status")) as AdbPreflightStatus;
    },
    async chooseDeviceBridgeAdbExecutable() {
      return (await invoke("choose_device_bridge_adb_executable")) as AdbPreflightStatus;
    },
    async checkDeviceBridgeAdb() {
      return (await invoke("check_device_bridge_adb")) as AdbPreflightStatus;
    },
    async deviceBridgeMappingStatus() {
      return (await invoke("get_device_bridge_mapping_status")) as DevBridgeMappingStatus;
    },
    async enableDeviceBridgeMapping() {
      return (await invoke("enable_device_bridge_mapping")) as DevBridgeMappingStatus;
    },
    async disableDeviceBridgeMapping() {
      return (await invoke("disable_device_bridge_mapping")) as DevBridgeMappingStatus;
    },
    async openProject(path) {
      return (await invoke("open_project", { path })) as ProjectSnapshot;
    },
    async saveStyle(request) {
      return (await invoke("save_project_style", { request })) as SaveStyleResult;
    },
    async saveDocument(request) {
      return (await invoke("save_project_document", { request })) as SaveStyleResult;
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
      scenarios: [
        {
          schemaVersion: 1,
          id: "org.lyra.scenario.midnight-galaxy",
          track: { title: "Midnight Galaxy", artist: "Future Echoes" },
          lyrics: [{ startMilliseconds: 0, endMilliseconds: 4000, text: "星河在此刻为你闪烁", translation: "The galaxy is shimmering for you" }],
          events: [],
          expectedDiagnostics: [],
        },
      ],
      documents: [
        {
          id: "style",
          label: "Styles",
          kind: "css",
          path: "/browser-fixture/future-lyrics/sustain/theme/lyra.css",
          relativePath: "theme/lyra.css",
          source: ":root {\n  --lyra-font-size: 42px;\n  --lyra-glow: 18%;\n}\n",
          sha256: "fixture-sustain",
        },
        {
          id: "html",
          label: "HTML",
          kind: "html",
          path: "/browser-fixture/future-lyrics/sustain/theme/index.html",
          relativePath: "theme/index.html",
          source: "<main class=\"lyra-blyrics-stage\"><section class=\"lyra-track\"><div class=\"lyra-art\">LYRA</div><div class=\"lyra-meta\"><strong data-lyra-track-title></strong><span data-lyra-track-artist></span></div></section><section id=\"blyrics-wrapper\" class=\"lyra-blyrics-viewport\"></section></main>\n",
          sha256: "fixture-html",
        },
        {
          id: "parameters",
          label: "Parameters",
          kind: "json",
          path: "/browser-fixture/future-lyrics/sustain/parameters.json",
          relativePath: "parameters.json",
          source: "{\n  \"schemaVersion\": 1,\n  \"groups\": []\n}\n",
          sha256: "fixture-parameters",
        },
        {
          id: "scenario-0",
          label: "Scenario 1",
          kind: "json",
          path: "/browser-fixture/future-lyrics/sustain/scenarios/midnight-galaxy.json",
          relativePath: "scenarios/midnight-galaxy.json",
          source: "{\n  \"schemaVersion\": 1,\n  \"id\": \"org.lyra.scenario.midnight-galaxy\",\n  \"track\": { \"title\": \"Midnight Galaxy\", \"artist\": \"Future Echoes\" },\n  \"lyrics\": [{ \"startMilliseconds\": 0, \"endMilliseconds\": 4000, \"text\": \"星河在此刻为你闪烁\", \"translation\": \"The galaxy is shimmering for you\" }],\n  \"events\": []\n}\n",
          sha256: "fixture-scenario",
        },
      ],
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
  let deviceBridge: DeviceBridgeStatus = { state: "stopped", session: null };
  let adbPreflight: AdbPreflightStatus = { configured: false, readiness: "unconfigured" };
  let deviceMapping: DevBridgeMappingStatus = { readiness: "inactive" };
  return {
    async appInfo() {
      return {
        appVersion: "0.1.0-alpha.1",
        packContractVersion: 1,
        projectContractVersion: 1,
        registryContractVersion: 1,
      };
    },
    async deviceBridgeStatus() {
      return structuredClone(deviceBridge);
    },
    async startDeviceBridge() {
      if (deviceBridge.state === "stopped") {
        deviceBridge = { state: "waiting", session: null };
      }
      return structuredClone(deviceBridge);
    },
    async stopDeviceBridge() {
      deviceBridge = { state: "stopped", session: null };
      deviceMapping = { readiness: "inactive" };
      return structuredClone(deviceBridge);
    },
    async deviceBridgeAdbStatus() {
      return structuredClone(adbPreflight);
    },
    async chooseDeviceBridgeAdbExecutable() {
      adbPreflight = { configured: true, readiness: "notChecked" };
      return structuredClone(adbPreflight);
    },
    async checkDeviceBridgeAdb() {
      adbPreflight = { configured: true, readiness: "oneReadyDevice" };
      return structuredClone(adbPreflight);
    },
    async deviceBridgeMappingStatus() {
      return structuredClone(deviceMapping);
    },
    async enableDeviceBridgeMapping() {
      deviceMapping = { readiness: "active" };
      return structuredClone(deviceMapping);
    },
    async disableDeviceBridgeMapping() {
      deviceMapping = { readiness: "inactive" };
      return structuredClone(deviceMapping);
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
      const styleDocument = pack.documents?.find((document) => document.id === "style");
      if (styleDocument) {
        styleDocument.source = request.source;
        styleDocument.sha256 = pack.styleSha256;
      }
      project = structuredClone(project);
      return { status: "saved", sha256: pack.styleSha256 };
    },
    async saveDocument(request) {
      const pack = project.packs.find((item) => item.root === request.packRoot);
      const document = pack?.documents?.find((item) => item.path === request.documentPath);
      if (!pack || !document) throw new Error("Fixture document not found");
      if (document.sha256 !== request.expectedSha256) {
        return { status: "conflict", sha256: document.sha256 };
      }
      document.source = request.source;
      document.sha256 = `fixture-${request.source.length}`;
      if (document.id === "style") {
        pack.styleSource = request.source;
        pack.styleSha256 = document.sha256;
      }
      project = structuredClone(project);
      return { status: "saved", sha256: document.sha256 };
    },
  };
}

export const backend = isTauriRuntime()
  ? createBackend((command, arguments_) => tauriInvoke(command, arguments_))
  : createFixtureBackend();
