export type ParameterControl =
  | "color"
  | "length"
  | "toggle"
  | "number"
  | "text"
  | "select";

export type ParameterValue = string | number | boolean;

export interface ParameterOption {
  label: string;
  value: string;
}

export interface ParameterDefinition {
  id: string;
  label: string;
  control: ParameterControl;
  binding: { cssVariable: string; [key: string]: unknown };
  defaultValue: ParameterValue;
  unit?: string;
  minimum?: number;
  maximum?: number;
  step?: number;
  options?: ParameterOption[];
  [key: string]: unknown;
}

export interface ParameterGroup {
  id: string;
  label: string;
  parameters: ParameterDefinition[];
  [key: string]: unknown;
}

export interface ParameterSchema {
  schemaVersion: 1;
  groups: ParameterGroup[];
  [key: string]: unknown;
}

interface ParameterSnapshot {
  source: string;
  values: Record<string, ParameterValue>;
}

export interface ParameterEditor extends ParameterSnapshot {
  schema: ParameterSchema;
  past: ParameterSnapshot[];
  future: ParameterSnapshot[];
  canUndo: boolean;
  canRedo: boolean;
}

const MAX_HISTORY = 100;

export function createParameterEditor(
  schema: ParameterSchema,
  source: string,
): ParameterEditor {
  return buildEditor(schema, snapshotFromSource(schema, source), [], []);
}

export function setParameterValue(
  editor: ParameterEditor,
  parameterId: string,
  value: ParameterValue,
): ParameterEditor {
  const parameter = findParameter(editor.schema, parameterId);
  if (!parameter) return editor;

  const source = patchCssVariable(
    editor.source,
    parameter.binding.cssVariable,
    formatParameterValue(parameter, value),
  );
  if (source === editor.source && Object.is(editor.values[parameterId], value)) {
    return editor;
  }

  const past = [...editor.past, toSnapshot(editor)].slice(-MAX_HISTORY);
  return buildEditor(
    editor.schema,
    { source, values: { ...editor.values, [parameterId]: value } },
    past,
    [],
  );
}

export function replaceParameterSource(
  editor: ParameterEditor,
  source: string,
): ParameterEditor {
  if (source === editor.source) return editor;
  const past = [...editor.past, toSnapshot(editor)].slice(-MAX_HISTORY);
  return buildEditor(
    editor.schema,
    snapshotFromSource(editor.schema, source),
    past,
    [],
  );
}

export function undoParameterEdit(editor: ParameterEditor): ParameterEditor {
  const previous = editor.past.at(-1);
  if (!previous) return editor;
  return buildEditor(
    editor.schema,
    previous,
    editor.past.slice(0, -1),
    [toSnapshot(editor), ...editor.future].slice(0, MAX_HISTORY),
  );
}

export function redoParameterEdit(editor: ParameterEditor): ParameterEditor {
  const next = editor.future[0];
  if (!next) return editor;
  return buildEditor(
    editor.schema,
    next,
    [...editor.past, toSnapshot(editor)].slice(-MAX_HISTORY),
    editor.future.slice(1),
  );
}

function buildEditor(
  schema: ParameterSchema,
  snapshot: ParameterSnapshot,
  past: ParameterSnapshot[],
  future: ParameterSnapshot[],
): ParameterEditor {
  return {
    schema,
    source: snapshot.source,
    values: { ...snapshot.values },
    past,
    future,
    canUndo: past.length > 0,
    canRedo: future.length > 0,
  };
}

function toSnapshot(editor: ParameterEditor): ParameterSnapshot {
  return { source: editor.source, values: { ...editor.values } };
}

function snapshotFromSource(
  schema: ParameterSchema,
  source: string,
): ParameterSnapshot {
  const values: Record<string, ParameterValue> = {};
  for (const group of schema.groups) {
    for (const parameter of group.parameters) {
      const raw = readCssVariable(source, parameter.binding.cssVariable);
      values[parameter.id] = raw === undefined
        ? parameter.defaultValue
        : parseParameterValue(parameter, raw);
    }
  }
  return { source, values };
}

function findParameter(
  schema: ParameterSchema,
  parameterId: string,
): ParameterDefinition | undefined {
  return schema.groups
    .flatMap((group) => group.parameters)
    .find((parameter) => parameter.id === parameterId);
}

function readCssVariable(source: string, variable: string): string | undefined {
  const match = new RegExp(`${escapeRegExp(variable)}\\s*:\\s*([^;{}]+);`).exec(source);
  return match?.[1]?.trim();
}

function parseParameterValue(
  parameter: ParameterDefinition,
  raw: string,
): ParameterValue {
  switch (parameter.control) {
    case "length":
    case "number": {
      const numeric = Number.parseFloat(raw);
      return Number.isFinite(numeric) ? numeric : parameter.defaultValue;
    }
    case "toggle":
      return ["1", "true", "on", "yes"].includes(raw.toLowerCase());
    default:
      return raw;
  }
}

function formatParameterValue(
  parameter: ParameterDefinition,
  value: ParameterValue,
): string {
  let formatted: string;
  switch (parameter.control) {
    case "toggle":
      formatted = value ? "1" : "0";
      break;
    case "length":
      formatted = `${asFiniteNumber(value)}${parameter.unit ?? ""}`;
      break;
    case "number":
      formatted = `${asFiniteNumber(value)}${parameter.unit ?? ""}`;
      break;
    default:
      formatted = String(value).trim();
  }
  if (!formatted || /[;{}\n\r]/.test(formatted)) {
    throw new Error(`Unsafe CSS value for ${parameter.id}`);
  }
  return formatted;
}

function asFiniteNumber(value: ParameterValue): number {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) throw new Error("Parameter value must be finite");
  return numeric;
}

function patchCssVariable(source: string, variable: string, value: string): string {
  if (!/^--[a-zA-Z0-9_-]+$/.test(variable)) {
    throw new Error(`Invalid CSS custom property: ${variable}`);
  }

  const root = /:root\s*\{/.exec(source);
  if (!root || root.index === undefined) {
    return `${source}${source.endsWith("\n") || source.length === 0 ? "" : "\n"}:root {\n  ${variable}: ${value};\n}\n`;
  }
  const bodyStart = root.index + root[0].length;
  const bodyEnd = source.indexOf("}", bodyStart);
  if (bodyEnd < 0) throw new Error("The :root CSS block is not closed");

  const body = source.slice(bodyStart, bodyEnd);
  const declaration = new RegExp(`${escapeRegExp(variable)}\\s*:`).exec(body);
  if (!declaration || declaration.index === undefined) {
    const indentation = body.includes("\n") ? "  " : " ";
    const insertion = `${body.endsWith("\n") ? "" : "\n"}${indentation}${variable}: ${value};\n`;
    return `${source.slice(0, bodyEnd)}${insertion}${source.slice(bodyEnd)}`;
  }

  const colon = bodyStart + declaration.index + declaration[0].lastIndexOf(":");
  const semicolon = source.indexOf(";", colon + 1);
  if (semicolon < 0 || semicolon > bodyEnd) {
    throw new Error(`CSS declaration for ${variable} is not terminated`);
  }
  const current = source.slice(colon + 1, semicolon);
  const leading = current.match(/^\s*/)?.[0] ?? "";
  const trailing = current.match(/\s*$/)?.[0] ?? "";
  return `${source.slice(0, colon + 1)}${leading}${value}${trailing}${source.slice(semicolon)}`;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
