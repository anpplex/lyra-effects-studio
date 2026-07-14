import { describe, expect, it, vi } from "vitest";
import { createBackend, type AppInfo } from "./backend";

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
});
