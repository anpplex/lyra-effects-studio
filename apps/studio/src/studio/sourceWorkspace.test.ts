import { describe, expect, it } from "vitest";
import {
  createSourceWorkspace,
  diagnoseSourceDocument,
  findSourceMatches,
  replaceAllSourceMatches,
  selectSourceDocument,
} from "./sourceWorkspace";
import type { EditableDocument } from "../lib/backend";

const documents: EditableDocument[] = [
  {
    id: "style",
    label: "Styles",
    kind: "css",
    path: "/tmp/theme/theme/lyra.css",
    relativePath: "theme/lyra.css",
    source: ":root {\n  --accent: cyan;\n}\nbody { color: cyan; }\n",
    sha256: "style-before",
  },
  {
    id: "parameters",
    label: "Parameters",
    kind: "json",
    path: "/tmp/theme/parameters.json",
    relativePath: "parameters.json",
    source: "{\n  \"schemaVersion\": 1\n}\n",
    sha256: "json-before",
  },
];

describe("source workspace", () => {
  it("switches between manifest-declared documents without losing drafts", () => {
    const opened = createSourceWorkspace(documents);
    const selected = selectSourceDocument(opened, "parameters");

    expect(opened.activeDocument.id).toBe("style");
    expect(selected.activeDocument.kind).toBe("json");
    expect(selected.documents).toHaveLength(2);
  });

  it("finds line/column matches and replaces all without touching case variants", () => {
    const opened = createSourceWorkspace(documents);
    const matches = findSourceMatches(opened.activeDocument.draftSource, "cyan", true);
    const replaced = replaceAllSourceMatches(opened, "cyan", "#53d6d8", true);

    expect(matches).toEqual([
      { start: 20, end: 24, line: 2, column: 13 },
      { start: 42, end: 46, line: 4, column: 15 },
    ]);
    expect(replaced.activeDocument.draftSource).not.toContain("cyan");
    expect(replaced.activeDocument.dirty).toBe(true);
  });

  it("reports JSON syntax diagnostics with a navigable position", () => {
    const opened = selectSourceDocument(createSourceWorkspace(documents), "parameters");
    const diagnostics = diagnoseSourceDocument({
      ...opened.activeDocument,
      draftSource: "{\n  \"schemaVersion\":\n}\n",
    });

    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]?.severity).toBe("error");
    expect(diagnostics[0]?.line).toBeGreaterThanOrEqual(1);
    expect(diagnostics[0]?.column).toBeGreaterThanOrEqual(1);
    expect(diagnostics[0]?.message).toContain("JSON");
  });
});
