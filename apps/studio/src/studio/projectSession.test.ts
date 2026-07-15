import { describe, expect, it } from "vitest";
import type { ProjectSnapshot } from "../lib/backend";
import {
  applySaveResult,
  createProjectSession,
  editProjectParameter,
  editProjectSource,
  redoProjectEdit,
  selectProjectDocument,
  undoProjectEdit,
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
      parameters: {
        schemaVersion: 1,
        groups: [
          {
            id: "type",
            label: "Typography",
            parameters: [
              {
                id: "font-size",
                label: "Font size",
                control: "length",
                binding: { cssVariable: "--lyra-font-size" },
                defaultValue: 42,
                unit: "px",
              },
            ],
          },
        ],
      },
      documents: [
        {
          id: "style",
          label: "Styles",
          kind: "css",
          path: "/tmp/lyra-theme/theme/lyra.css",
          relativePath: "theme/lyra.css",
          source: ":root {}\n",
          sha256: "before",
        },
        {
          id: "parameters",
          label: "Parameters",
          kind: "json",
          path: "/tmp/lyra-theme/parameters.json",
          relativePath: "parameters.json",
          source: "{\"schemaVersion\":1,\"groups\":[]}",
          sha256: "parameters-before",
        },
      ],
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

  it("applies schema parameters to CSS and shares undo history with source edits", () => {
    const opened = createProjectSession(fixture);
    const resized = editProjectParameter(opened, "font-size", 48);
    const sourceEdited = editProjectSource(resized, `${resized.draftSource}/* note */\n`);
    const undoneSource = undoProjectEdit(sourceEdited);
    const undoneParameter = undoProjectEdit(undoneSource);
    const redoneParameter = redoProjectEdit(undoneParameter);

    expect(resized.draftSource).toContain("--lyra-font-size: 48px;");
    expect(resized.dirty).toBe(true);
    expect(sourceEdited.parameterEditor?.values["font-size"]).toBe(48);
    expect(undoneSource.draftSource).toBe(resized.draftSource);
    expect(undoneParameter.draftSource).toBe(fixture.packs[0]?.styleSource);
    expect(redoneParameter.draftSource).toBe(resized.draftSource);
  });

  it("switches active source documents and tracks their drafts independently", () => {
    const opened = createProjectSession(fixture);
    const selected = selectProjectDocument(opened, "parameters");
    const parameterEdited = editProjectParameter(selected, "font-size", 48);
    const edited = editProjectSource(selected, "{\"schemaVersion\":1,\"groups\":[]}\n");
    const restoredStyle = selectProjectDocument(edited, "style");

    expect(selected.activeDocument?.kind).toBe("json");
    expect(parameterEdited.activeDocument.id).toBe("style");
    expect(parameterEdited.draftSource).toContain("48px");
    expect(edited.dirty).toBe(true);
    expect(restoredStyle.draftSource).toBe(":root {}\n");
    expect(restoredStyle.sourceWorkspace?.documents.find((item) => item.id === "parameters")?.dirty).toBe(true);
  });
});
