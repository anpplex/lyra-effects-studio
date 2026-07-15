import { describe, expect, it, vi } from "vitest";
import {
  createBackend,
  type AdbPreflightStatus,
  type AppInfo,
  type DevBridgeMappingStatus,
  type DeviceBridgeStatus,
  type ProjectSnapshot,
  type SaveDocumentRequest,
  type SaveStyleRequest,
} from "./backend";

describe("typed backend facade", () => {
  it("requests app_info through the supplied transport", async () => {
    const expected: AppInfo = {
      appVersion: "0.1.0-alpha.1",
      packContractVersion: 1,
      projectContractVersion: 1,
      registryContractVersion: 1,
    };
    const invoke = vi.fn(async () => expected);
    const backend = createBackend(invoke);

    await expect(backend.appInfo()).resolves.toEqual(expected);
    expect(invoke).toHaveBeenCalledWith("app_info");
  });

  it("uses explicit typed commands for project reads and writes", async () => {
    const snapshot: ProjectSnapshot = {
      root: "/tmp/theme",
      effectsRoot: "/tmp/theme",
      mode: "standalone",
      packs: [],
    };
    const invoke = vi.fn(async (command: string) =>
      command === "open_project" ? snapshot : { status: "saved", sha256: "next" },
    );
    const backend = createBackend(invoke);
    const saveRequest: SaveStyleRequest = {
      packRoot: "/tmp/theme",
      expectedSha256: "before",
      source: ":root {}\n",
    };

    await expect(backend.openProject("/tmp/theme")).resolves.toEqual(snapshot);
    await expect(backend.saveStyle(saveRequest)).resolves.toEqual({
      status: "saved",
      sha256: "next",
    });
    const documentRequest: SaveDocumentRequest = {
      packRoot: "/tmp/theme",
      documentPath: "/tmp/theme/parameters.json",
      expectedSha256: "json-before",
      source: "{}\n",
    };
    await expect(backend.saveDocument(documentRequest)).resolves.toEqual({
      status: "saved",
      sha256: "next",
    });
    expect(invoke).toHaveBeenNthCalledWith(1, "open_project", { path: "/tmp/theme" });
    expect(invoke).toHaveBeenNthCalledWith(2, "save_project_style", { request: saveRequest });
    expect(invoke).toHaveBeenNthCalledWith(3, "save_project_document", { request: documentRequest });
  });

  it("uses narrow typed commands for local Dev Bridge controls", async () => {
    const stopped: DeviceBridgeStatus = { state: "stopped", session: null };
    const invoke = vi.fn(async () => stopped);
    const backend = createBackend(invoke);

    await expect(backend.deviceBridgeStatus()).resolves.toEqual(stopped);
    await expect(backend.startDeviceBridge()).resolves.toEqual(stopped);
    await expect(backend.stopDeviceBridge()).resolves.toEqual(stopped);
    expect(invoke).toHaveBeenNthCalledWith(1, "get_device_bridge_status");
    expect(invoke).toHaveBeenNthCalledWith(2, "start_device_bridge");
    expect(invoke).toHaveBeenNthCalledWith(3, "stop_device_bridge");
  });

  it("uses no-argument commands for the user-gated ADB preflight", async () => {
    const unconfigured: AdbPreflightStatus = {
      configured: false,
      readiness: "unconfigured",
    };
    const invoke = vi.fn(async () => unconfigured);
    const backend = createBackend(invoke);

    await expect(backend.deviceBridgeAdbStatus()).resolves.toEqual(unconfigured);
    await expect(backend.chooseDeviceBridgeAdbExecutable()).resolves.toEqual(unconfigured);
    await expect(backend.checkDeviceBridgeAdb()).resolves.toEqual(unconfigured);
    expect(invoke).toHaveBeenNthCalledWith(1, "get_device_bridge_adb_status");
    expect(invoke).toHaveBeenNthCalledWith(2, "choose_device_bridge_adb_executable");
    expect(invoke).toHaveBeenNthCalledWith(3, "check_device_bridge_adb");
    expect(invoke).not.toHaveBeenCalledWith(expect.any(String), expect.anything());
  });

  it("uses no-argument commands for explicit Dev Bridge mapping", async () => {
    const inactive: DevBridgeMappingStatus = { readiness: "inactive" };
    const invoke = vi.fn(async () => inactive);
    const backend = createBackend(invoke);

    await expect(backend.deviceBridgeMappingStatus()).resolves.toEqual(inactive);
    await expect(backend.enableDeviceBridgeMapping()).resolves.toEqual(inactive);
    await expect(backend.disableDeviceBridgeMapping()).resolves.toEqual(inactive);
    expect(invoke).toHaveBeenNthCalledWith(1, "get_device_bridge_mapping_status");
    expect(invoke).toHaveBeenNthCalledWith(2, "enable_device_bridge_mapping");
    expect(invoke).toHaveBeenNthCalledWith(3, "disable_device_bridge_mapping");
    expect(invoke).not.toHaveBeenCalledWith(expect.any(String), expect.anything());
  });
});
