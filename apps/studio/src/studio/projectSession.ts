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
}

export function createProjectSession(project: ProjectSnapshot): ProjectSession {
  const activePack = project.packs[0];
  if (!activePack) throw new Error("The project does not contain an editable style Pack");
  return {
    project,
    activePack,
    draftSource: activePack.styleSource,
    persistedSource: activePack.styleSource,
    expectedSha256: activePack.styleSha256,
    dirty: false,
    status: "ready",
    parameterEditor: activePack.parameters
      ? createParameterEditor(activePack.parameters, activePack.styleSource)
      : undefined,
  };
}

export function selectProjectPack(session: ProjectSession, packId: string): ProjectSession {
  const activePack = session.project.packs.find((pack) => pack.id === packId);
  if (!activePack || activePack.id === session.activePack.id) return session;
  return {
    ...session,
    activePack,
    draftSource: activePack.styleSource,
    persistedSource: activePack.styleSource,
    expectedSha256: activePack.styleSha256,
    dirty: false,
    status: "ready",
    message: undefined,
    parameterEditor: activePack.parameters
      ? createParameterEditor(activePack.parameters, activePack.styleSource)
      : undefined,
  };
}

export function editProjectSource(session: ProjectSession, source: string): ProjectSession {
  const parameterEditor = session.parameterEditor
    ? replaceParameterSource(session.parameterEditor, source)
    : undefined;
  return {
    ...session,
    draftSource: source,
    dirty: source !== session.persistedSource,
    status: "ready",
    message: undefined,
    parameterEditor,
  };
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
  return {
    ...session,
    parameterEditor,
    draftSource: parameterEditor.source,
    dirty: parameterEditor.source !== session.persistedSource,
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
  const updatedPack = {
    ...session.activePack,
    styleSource: session.draftSource,
    styleSha256: result.sha256,
  };
  return {
    ...session,
    project: {
      ...session.project,
      packs: session.project.packs.map((pack) =>
        pack.id === updatedPack.id ? updatedPack : pack,
      ),
    },
    activePack: updatedPack,
    persistedSource: session.draftSource,
    expectedSha256: result.sha256,
    dirty: false,
    status: "saved",
    message: "Saved atomically",
  };
}
