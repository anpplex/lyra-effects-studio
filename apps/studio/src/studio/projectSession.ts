import type {
  EditablePack,
  ProjectSnapshot,
  SaveStyleResult,
} from "../lib/backend";
import {
  createParameterEditor,
  redoParameterEdit,
  replaceParameterSource,
  setParameterValue,
  undoParameterEdit,
  type ParameterEditor,
  type ParameterValue,
} from "./parameterEditor";
import {
  createSourceWorkspace,
  editSourceDocument,
  editActiveSourceDocument,
  hasDirtySourceDocuments,
  markSourceDocumentSaved,
  selectSourceDocument,
  type SourceDocumentState,
  type SourceWorkspace,
} from "./sourceWorkspace";

export type ProjectSessionStatus = "ready" | "saving" | "saved" | "conflict" | "error";

export interface ProjectSession {
  project: ProjectSnapshot;
  activePack: EditablePack;
  draftSource: string;
  persistedSource: string;
  expectedSha256: string;
  dirty: boolean;
  status: ProjectSessionStatus;
  message?: string;
  parameterEditor?: ParameterEditor;
  sourceWorkspace: SourceWorkspace;
  activeDocument: SourceDocumentState;
}

export function createProjectSession(project: ProjectSnapshot): ProjectSession {
  const activePack = project.packs[0];
  if (!activePack) throw new Error("The project does not contain an editable style Pack");
  return createSessionForPack(project, activePack);
}

function createSessionForPack(
  project: ProjectSnapshot,
  activePack: EditablePack,
): ProjectSession {
  const sourceWorkspace = createSourceWorkspace(activePack.documents ?? [{
    id: "style",
    label: "Styles",
    kind: "css",
    path: activePack.stylePath,
    relativePath: "theme/lyra.css",
    source: activePack.styleSource,
    sha256: activePack.styleSha256,
  }]);
  const activeDocument = sourceWorkspace.activeDocument;
  return {
    project,
    activePack,
    draftSource: activeDocument.draftSource,
    persistedSource: activeDocument.persistedSource,
    expectedSha256: activeDocument.expectedSha256,
    dirty: false,
    status: "ready",
    sourceWorkspace,
    activeDocument,
    parameterEditor: activePack.parameters
      ? createParameterEditor(activePack.parameters, activePack.styleSource)
      : undefined,
  };
}

export function selectProjectPack(session: ProjectSession, packId: string): ProjectSession {
  const activePack = session.project.packs.find((pack) => pack.id === packId);
  if (!activePack || activePack.id === session.activePack.id) return session;
  return createSessionForPack(session.project, activePack);
}

export function selectProjectDocument(
  session: ProjectSession,
  documentId: string,
): ProjectSession {
  return applySourceWorkspace(
    session,
    selectSourceDocument(session.sourceWorkspace, documentId),
    session.parameterEditor,
  );
}

export function editProjectSource(session: ProjectSession, source: string): ProjectSession {
  const parameterEditor = session.parameterEditor && session.activeDocument.id === "style"
    ? replaceParameterSource(session.parameterEditor, source)
    : session.parameterEditor;
  return applySourceWorkspace(
    session,
    editActiveSourceDocument(session.sourceWorkspace, source),
    parameterEditor,
  );
}

export function editProjectParameter(
  session: ProjectSession,
  parameterId: string,
  value: ParameterValue,
): ProjectSession {
  if (!session.parameterEditor) return session;
  return applyParameterEditor(
    session,
    setParameterValue(session.parameterEditor, parameterId, value),
  );
}

export function undoProjectEdit(session: ProjectSession): ProjectSession {
  if (!session.parameterEditor) return session;
  return applyParameterEditor(session, undoParameterEdit(session.parameterEditor));
}

export function redoProjectEdit(session: ProjectSession): ProjectSession {
  if (!session.parameterEditor) return session;
  return applyParameterEditor(session, redoParameterEdit(session.parameterEditor));
}

function applyParameterEditor(
  session: ProjectSession,
  parameterEditor: ParameterEditor,
): ProjectSession {
  if (parameterEditor === session.parameterEditor) return session;
  const sourceWorkspace = selectSourceDocument(
    editSourceDocument(session.sourceWorkspace, "style", parameterEditor.source),
    "style",
  );
  return applySourceWorkspace(
    session,
    sourceWorkspace,
    parameterEditor,
  );
}

function applySourceWorkspace(
  session: ProjectSession,
  sourceWorkspace: SourceWorkspace,
  parameterEditor: ParameterEditor | undefined,
): ProjectSession {
  const activeDocument = sourceWorkspace.activeDocument;
  return {
    ...session,
    sourceWorkspace,
    activeDocument,
    parameterEditor,
    draftSource: activeDocument.draftSource,
    persistedSource: activeDocument.persistedSource,
    expectedSha256: activeDocument.expectedSha256,
    dirty: hasDirtySourceDocuments(sourceWorkspace),
    status: "ready",
    message: undefined,
  };
}

export function applySaveResult(
  session: ProjectSession,
  result: SaveStyleResult,
): ProjectSession {
  if (result.status === "conflict") {
    return {
      ...session,
      status: "conflict",
      message: "The source changed on disk. Reload or save a copy before overwriting.",
    };
  }
  const sourceWorkspace = markSourceDocumentSaved(
    session.sourceWorkspace,
    session.activeDocument.id,
    result.sha256,
  );
  const savedDocument = sourceWorkspace.activeDocument;
  const updatedPack: EditablePack = {
    ...session.activePack,
    styleSource: savedDocument.id === "style"
      ? savedDocument.draftSource
      : session.activePack.styleSource,
    styleSha256: savedDocument.id === "style"
      ? result.sha256
      : session.activePack.styleSha256,
    documents: sourceWorkspace.documents.map((document) => ({
      id: document.id,
      label: document.label,
      kind: document.kind,
      path: document.path,
      relativePath: document.relativePath,
      source: document.draftSource,
      sha256: document.expectedSha256,
    })),
  };
  const activeDocument = sourceWorkspace.activeDocument;
  const dirty = hasDirtySourceDocuments(sourceWorkspace);
  return {
    ...session,
    project: {
      ...session.project,
      packs: session.project.packs.map((pack) =>
        pack.id === updatedPack.id ? updatedPack : pack,
      ),
    },
    activePack: updatedPack,
    sourceWorkspace,
    activeDocument,
    draftSource: activeDocument.draftSource,
    persistedSource: activeDocument.persistedSource,
    expectedSha256: activeDocument.expectedSha256,
    dirty,
    status: "saved",
    message: dirty ? "Saved this document; other changes remain" : "Saved atomically",
  };
}
