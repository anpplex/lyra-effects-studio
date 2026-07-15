import type { EditableDocument } from "../lib/backend";

export interface SourceDocumentState extends Omit<EditableDocument, "source" | "sha256"> {
  draftSource: string;
  persistedSource: string;
  expectedSha256: string;
  dirty: boolean;
}

export interface SourceWorkspace {
  documents: SourceDocumentState[];
  activeDocumentId: string;
  activeDocument: SourceDocumentState;
}

export interface SourceMatch {
  start: number;
  end: number;
  line: number;
  column: number;
}

export interface SourceDiagnostic {
  severity: "error" | "warning";
  message: string;
  line: number;
  column: number;
}

export function createSourceWorkspace(documents: EditableDocument[]): SourceWorkspace {
  if (documents.length === 0) throw new Error("The Pack has no editable source documents");
  const states = documents.map(toDocumentState);
  return buildWorkspace(states, states[0]!.id);
}

export function selectSourceDocument(
  workspace: SourceWorkspace,
  documentId: string,
): SourceWorkspace {
  if (documentId === workspace.activeDocumentId) return workspace;
  if (!workspace.documents.some((document) => document.id === documentId)) return workspace;
  return buildWorkspace(workspace.documents, documentId);
}

export function editActiveSourceDocument(
  workspace: SourceWorkspace,
  source: string,
): SourceWorkspace {
  return editSourceDocument(workspace, workspace.activeDocumentId, source);
}

export function editSourceDocument(
  workspace: SourceWorkspace,
  documentId: string,
  source: string,
): SourceWorkspace {
  const target = workspace.documents.find((document) => document.id === documentId);
  if (!target || source === target.draftSource) return workspace;
  const documents = workspace.documents.map((document) =>
    document.id === documentId
      ? { ...document, draftSource: source, dirty: source !== document.persistedSource }
      : document,
  );
  return buildWorkspace(documents, workspace.activeDocumentId);
}

export function markSourceDocumentSaved(
  workspace: SourceWorkspace,
  documentId: string,
  sha256: string,
): SourceWorkspace {
  const documents = workspace.documents.map((document) =>
    document.id === documentId
      ? {
          ...document,
          persistedSource: document.draftSource,
          expectedSha256: sha256,
          dirty: false,
        }
      : document,
  );
  return buildWorkspace(documents, workspace.activeDocumentId);
}

export function replaceAllSourceMatches(
  workspace: SourceWorkspace,
  query: string,
  replacement: string,
  caseSensitive = false,
): SourceWorkspace {
  if (!query) return workspace;
  const expression = new RegExp(escapeRegExp(query), caseSensitive ? "g" : "gi");
  return editActiveSourceDocument(
    workspace,
    workspace.activeDocument.draftSource.replace(expression, () => replacement),
  );
}

export function findSourceMatches(
  source: string,
  query: string,
  caseSensitive = false,
): SourceMatch[] {
  if (!query) return [];
  const expression = new RegExp(escapeRegExp(query), caseSensitive ? "g" : "gi");
  return [...source.matchAll(expression)].map((match) => {
    const start = match.index;
    const location = sourceLocation(source, start);
    return { start, end: start + match[0].length, ...location };
  });
}

export function diagnoseSourceDocument(
  document: SourceDocumentState,
): SourceDiagnostic[] {
  if (document.kind === "json") {
    try {
      JSON.parse(document.draftSource);
      return [];
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const position = Number(message.match(/position\s+(\d+)/i)?.[1] ?? 0);
      const explicitLine = Number(message.match(/line\s+(\d+)/i)?.[1] ?? 0);
      const explicitColumn = Number(message.match(/column\s+(\d+)/i)?.[1] ?? 0);
      const location = explicitLine > 0
        ? { line: explicitLine, column: Math.max(1, explicitColumn) }
        : sourceLocation(document.draftSource, position);
      return [{ severity: "error", message: `JSON: ${message}`, ...location }];
    }
  }
  if (document.kind === "css") {
    const opening = countCharacter(document.draftSource, "{");
    const closing = countCharacter(document.draftSource, "}");
    if (opening !== closing) {
      return [{ severity: "warning", message: "CSS: block braces may be unbalanced", line: 1, column: 1 }];
    }
  }
  return [];
}

export function hasDirtySourceDocuments(workspace: SourceWorkspace): boolean {
  return workspace.documents.some((document) => document.dirty);
}

function toDocumentState(document: EditableDocument): SourceDocumentState {
  const { source, sha256, ...identity } = document;
  return {
    ...identity,
    draftSource: source,
    persistedSource: source,
    expectedSha256: sha256,
    dirty: false,
  };
}

function buildWorkspace(
  documents: SourceDocumentState[],
  activeDocumentId: string,
): SourceWorkspace {
  const activeDocument = documents.find((document) => document.id === activeDocumentId);
  if (!activeDocument) throw new Error(`Unknown source document: ${activeDocumentId}`);
  return { documents, activeDocumentId, activeDocument };
}

function sourceLocation(source: string, position: number): { line: number; column: number } {
  const safePosition = Math.max(0, Math.min(position, source.length));
  const before = source.slice(0, safePosition);
  const lines = before.split("\n");
  return { line: lines.length, column: (lines.at(-1)?.length ?? 0) + 1 };
}

function countCharacter(source: string, character: string): number {
  return [...source].filter((value) => value === character).length;
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
