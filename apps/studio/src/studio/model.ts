export type PreviewMode = "day" | "night";
export type InspectorTab = "design" | "source" | "diagnostics";

export type ThemeSummary = {
  id: string;
  name: string;
  version: string;
  status: "ready" | "draft";
};

export type PackSummary = {
  id: string;
  name: string;
  description: string;
  themes: ThemeSummary[];
};

export type Diagnostic = {
  id: string;
  level: "info" | "warning" | "error";
  message: string;
  time: string;
};

export type StudioParameters = {
  fontSize: number;
  glow: number;
  rightZone: number;
  motion: number;
  showSafeArea: boolean;
};

export type StudioState = {
  packs: PackSummary[];
  selectedPackId: string;
  selectedThemeId: string;
  inspectorTab: InspectorTab;
  preview: {
    mode: PreviewMode;
    profileId: string;
    scenarioId: string;
    playing: boolean;
  };
  parameters: StudioParameters;
  diagnostics: Diagnostic[];
};

const bundledPacks: PackSummary[] = [
  {
    id: "better-lyrics",
    name: "Better Lyrics",
    description: "Immersive single-line lyric themes",
    themes: [
      { id: "apple-music", name: "Apple Music", version: "1.2.0", status: "ready" },
      { id: "minimal", name: "Minimal", version: "1.0.3", status: "ready" },
    ],
  },
  {
    id: "salt-player",
    name: "Salt Player",
    description: "Layered typography and soft motion",
    themes: [{ id: "salt-multi", name: "Salt Multi", version: "0.9.1", status: "draft" }],
  },
  {
    id: "lyricify",
    name: "Lyricify",
    description: "Clean high-contrast cluster layouts",
    themes: [{ id: "lyricify-glass", name: "Lyricify Glass", version: "1.1.0", status: "ready" }],
  },
];

export function createStudioState(): StudioState {
  return {
    packs: bundledPacks,
    selectedPackId: "better-lyrics",
    selectedThemeId: "apple-music",
    inspectorTab: "design",
    preview: {
      mode: "day",
      profileId: "avatr-cluster-4032x284",
      scenarioId: "playing",
      playing: true,
    },
    parameters: {
      fontSize: 42,
      glow: 18,
      rightZone: 46,
      motion: 0.8,
      showSafeArea: true,
    },
    diagnostics: [
      { id: "bridge", level: "info", message: "Preview bridge ready", time: "02:18:04" },
      { id: "assets", level: "info", message: "9 assets validated", time: "02:18:05" },
      { id: "contrast", level: "warning", message: "Night-mode glow may bloom on OLED", time: "02:18:06" },
    ],
  };
}

export function selectPack(state: StudioState, packId: string): StudioState {
  const pack = state.packs.find((item) => item.id === packId);
  if (!pack || pack.themes.length === 0) return state;
  return { ...state, selectedPackId: packId, selectedThemeId: pack.themes[0].id };
}

export function selectTheme(state: StudioState, themeId: string): StudioState {
  const theme = state.packs.flatMap((pack) => pack.themes).find((item) => item.id === themeId);
  if (!theme) return state;
  const pack = state.packs.find((item) => item.themes.some((item) => item.id === themeId));
  return { ...state, selectedPackId: pack?.id ?? state.selectedPackId, selectedThemeId: themeId };
}

export function setPreviewMode(state: StudioState, mode: PreviewMode): StudioState {
  return { ...state, preview: { ...state.preview, mode } };
}

export function updateParameter<K extends keyof StudioParameters>(
  state: StudioState,
  key: K,
  value: StudioParameters[K],
): StudioState {
  return { ...state, parameters: { ...state.parameters, [key]: value } };
}
