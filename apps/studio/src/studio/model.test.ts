import { describe, expect, it } from "vitest";
import {
  createStudioState,
  selectTheme,
  setPreviewMode,
  updateParameter,
} from "./model";

describe("studio state", () => {
  it("opens the bundled Better Lyrics theme by default", () => {
    const state = createStudioState();

    expect(state.selectedPackId).toBe("better-lyrics");
    expect(state.selectedThemeId).toBe("apple-music");
    expect(state.preview.mode).toBe("day");
    expect(state.diagnostics.filter((item) => item.level === "error")).toHaveLength(0);
  });

  it("switches themes without losing the selected device profile", () => {
    const state = createStudioState();
    const next = selectTheme(state, "minimal");

    expect(next.selectedThemeId).toBe("minimal");
    expect(next.preview.profileId).toBe("avatr-cluster-4032x284");
  });

  it("updates preview mode and a theme parameter immutably", () => {
    const state = createStudioState();
    const night = setPreviewMode(state, "night");
    const tuned = updateParameter(night, "fontSize", 48);

    expect(tuned.preview.mode).toBe("night");
    expect(tuned.parameters.fontSize).toBe(48);
    expect(state.preview.mode).toBe("day");
    expect(state.parameters.fontSize).toBe(42);
  });
});
