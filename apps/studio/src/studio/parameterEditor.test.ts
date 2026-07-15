import { describe, expect, it } from "vitest";
import {
  createParameterEditor,
  redoParameterEdit,
  setParameterValue,
  undoParameterEdit,
  type ParameterSchema,
} from "./parameterEditor";

const schema: ParameterSchema = {
  schemaVersion: 1,
  groups: [
    {
      id: "appearance",
      label: "Appearance",
      parameters: [
        { id: "accent", label: "Accent", control: "color", binding: { cssVariable: "--accent" }, defaultValue: "#ffffff" },
        { id: "size", label: "Size", control: "length", binding: { cssVariable: "--size" }, defaultValue: 40, unit: "px", minimum: 24, maximum: 72, step: 1 },
        { id: "weight", label: "Weight", control: "select", binding: { cssVariable: "--weight" }, defaultValue: "600", options: [{ label: "Regular", value: "400" }, { label: "Bold", value: "700" }] },
        { id: "glow", label: "Glow", control: "toggle", binding: { cssVariable: "--glow" }, defaultValue: true },
        { id: "family", label: "Family", control: "text", binding: { cssVariable: "--family" }, defaultValue: "Inter" },
      ],
    },
  ],
};

const source = `/* keep this comment */
:root {
  --accent: #53d6d8;
  --size: 42px; /* preserve */
  --weight: 700;
  --glow: 1;
  --family: Inter;
}
body { color: var(--accent); }
`;

describe("schema parameter editor", () => {
  it("hydrates all control values from the current CSS source", () => {
    const editor = createParameterEditor(schema, source);

    expect(editor.values).toEqual({
      accent: "#53d6d8",
      size: 42,
      weight: "700",
      glow: true,
      family: "Inter",
    });
  });

  it("patches only the requested CSS value and supports undo and redo", () => {
    const initial = createParameterEditor(schema, source);
    const edited = setParameterValue(initial, "size", 48);
    const undone = undoParameterEdit(edited);
    const redone = redoParameterEdit(undone);

    expect(edited.source).toContain("--size: 48px; /* preserve */");
    expect(edited.source).toContain("/* keep this comment */");
    expect(edited.source).toContain("body { color: var(--accent); }");
    expect(edited.canUndo).toBe(true);
    expect(edited.canRedo).toBe(false);
    expect(undone.source).toBe(source);
    expect(undone.canRedo).toBe(true);
    expect(redone.source).toBe(edited.source);
    expect(redone.values.size).toBe(48);
  });

  it("formats toggle and length controls as safe CSS values", () => {
    const initial = createParameterEditor(schema, source);
    const withoutGlow = setParameterValue(initial, "glow", false);
    const resized = setParameterValue(withoutGlow, "size", 56);

    expect(withoutGlow.source).toContain("--glow: 0;");
    expect(resized.source).toContain("--size: 56px;");
  });
});
