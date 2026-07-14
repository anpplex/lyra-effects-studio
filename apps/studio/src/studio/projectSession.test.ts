import { describe, expect, it } from "vitest";
import type { ProjectSnapshot } from "../lib/backend";
import {
  applySaveResult,
  createProjectSession,
  editProjectSource,
} from "./projectSession";

const fixture: ProjectSnapshot = {
  root: "/tmp/lyra-theme",
  effectsRoot: "/tmp/lyra-theme",
  mode: "standalone",
  packs: [
    {
      id: "io.lyra.test.theme",
      name: "Test Theme",
      version: "1.0.0",
      family: "test",
      root: "/tmp/lyra-theme",
      stylePath: "/tmp/lyra-theme/theme/lyra.css",
      styleSource: ":root {}\n",
      styleSha256: "before",
    },
  ],
};

describe("project editing session", () => {
  it("tracks dirty source separately from the persisted snapshot", () => {
    const opened = createProjectSession(fixture);
    const edited = editProjectSource(opened, ":root { color: cyan; }\n");

    expect(opened.dirty).toBe(false);
    expect(edited.dirty).toBe(true);
    expect(edited.draftSource).toContain("cyan");
    expect(edited.persistedSource).toBe(":root {}\n");
  });

  it("clears dirty state after save and preserves it on conflict", () => {
    const edited = editProjectSource(createProjectSession(fixture), ":root { color: cyan; }\n");
    const saved = applySaveResult(edited, { status: "saved", sha256: "after" });
    const conflicted = applySaveResult(edited, { status: "conflict", sha256: "external" });

    expect(saved.dirty).toBe(false);
    expect(saved.expectedSha256).toBe("after");
    expect(saved.status).toBe("saved");
    expect(conflicted.dirty).toBe(true);
    expect(conflicted.expectedSha256).toBe("before");
    expect(conflicted.status).toBe("conflict");
  });
});
