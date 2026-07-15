export interface PreviewScenario {
  schemaVersion: 1;
  id: string;
  track: {
    title: string;
    artist: string;
    artwork?: string;
    [key: string]: unknown;
  };
  lyrics: Array<{
    startMilliseconds: number;
    endMilliseconds: number;
    text: string;
    translation?: string;
    transliteration?: string;
    [key: string]: unknown;
  }>;
  events: Array<{ atMilliseconds: number; kind: string; [key: string]: unknown }>;
  expectedDiagnostics?: string[];
  [key: string]: unknown;
}

export interface PreviewDocumentInput {
  css: string;
  html?: string;
  scenario: PreviewScenario;
  mode: "day" | "night";
  playing: boolean;
  nonce: string;
}

export const DEFAULT_PREVIEW_SCENARIO: PreviewScenario = {
  schemaVersion: 1,
  id: "org.lyra.scenario.default",
  track: {
    title: "Midnight Galaxy",
    artist: "Future Echoes",
  },
  lyrics: [{
    startMilliseconds: 0,
    endMilliseconds: 4_000,
    text: "星河在此刻为你闪烁",
    translation: "The galaxy is shimmering for you",
  }],
  events: [],
  expectedDiagnostics: [],
};

export function parsePreviewScenarioDocument(source: string): PreviewScenario | undefined {
  try {
    const value: unknown = JSON.parse(source);
    if (!isRecord(value) || value.schemaVersion !== 1 || typeof value.id !== "string") return undefined;
    if (!isRecord(value.track) || typeof value.track.title !== "string" || typeof value.track.artist !== "string") return undefined;
    if (!Array.isArray(value.lyrics) || !Array.isArray(value.events)) return undefined;
    const lyricsAreValid = value.lyrics.every((line) => isRecord(line)
      && typeof line.startMilliseconds === "number"
      && typeof line.endMilliseconds === "number"
      && typeof line.text === "string");
    return lyricsAreValid ? value as unknown as PreviewScenario : undefined;
  } catch {
    return undefined;
  }
}

export function buildPreviewDocument(input: PreviewDocumentInput): string {
  if (!/^[a-zA-Z0-9_-]+$/.test(input.nonce)) {
    throw new Error("Preview nonce contains unsupported characters");
  }
  const current = input.scenario.lyrics[0] ?? {
    startMilliseconds: 0,
    endMilliseconds: 1,
    text: "No lyric lines in this scenario",
  };
  const markup = input.html ?? defaultMarkup(input.scenario, current);
  const scenario = serializeForInlineScript(input.scenario);
  const css = escapeStyleText(input.css);
  const csp = [
    "default-src 'none'",
    "connect-src 'none'",
    "img-src data: blob:",
    "font-src data:",
    "media-src 'none'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "style-src 'unsafe-inline'",
    `script-src 'nonce-${input.nonce}'`,
  ].join("; ");

  return `<!doctype html>
<html lang="en" data-mode="${input.mode}" data-playing="${input.playing}">
<head>
  <meta charset="utf-8">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <style>${BASE_PREVIEW_CSS}\n${css}</style>
</head>
<body class="cluster-bar ${input.mode}">
  <script nonce="${input.nonce}">
    (() => {
      const scenario = ${scenario};
      const clone = (value) => JSON.parse(JSON.stringify(value));
      const emit = (type, message) => parent.postMessage({ source: "lyra-preview", token: "${input.nonce}", type, message }, "*");
      const fit = () => document.documentElement.style.setProperty("--lyra-preview-scale", String(innerWidth / 4032));
      const subscribers = new Set();
      const duration = Math.max(1, ...scenario.lyrics.map((line) => line.endMilliseconds), ...scenario.events.map((event) => event.atMilliseconds));
      let positionMilliseconds = 0;
      let currentLineId = -1;
      const renderTimeline = () => {
        const lineIndex = Math.max(0, scenario.lyrics.findIndex((line) => positionMilliseconds >= line.startMilliseconds && positionMilliseconds < line.endMilliseconds));
        const line = scenario.lyrics[lineIndex];
        if (line && lineIndex !== currentLineId) {
          document.querySelectorAll("[data-lyra-current-line]").forEach((node) => { node.textContent = line.text; });
          document.querySelectorAll("[data-lyra-translation]").forEach((node) => { node.textContent = line.translation || ""; });
          currentLineId = lineIndex;
        }
        const event = clone({ type: "timeline", positionMilliseconds, lyric: line || null });
        subscribers.forEach((subscriber) => subscriber(event));
      };
      const hydrate = () => {
        document.querySelectorAll("[data-lyra-track-title]").forEach((node) => { node.textContent = scenario.track.title; });
        document.querySelectorAll("[data-lyra-track-artist]").forEach((node) => { node.textContent = scenario.track.artist; });
        const wrapper = document.querySelector("#blyrics-wrapper");
        if (wrapper && !wrapper.querySelector("[data-lyra-current-line]")) {
          const container = document.createElement("div");
          container.className = "blyrics-container";
          const lyric = document.createElement("div");
          lyric.className = "blyrics--line blyrics--active blyrics--animating";
          lyric.dataset.lyraCurrentLine = "";
          const translation = document.createElement("div");
          translation.className = "blyrics--line lyra-translation";
          translation.dataset.lyraTranslation = "";
          container.append(lyric, translation);
          wrapper.append(container);
        }
        renderTimeline();
      };
      window.lyraBridge = Object.freeze({
        version: "1.0.0",
        getScenario: () => clone(scenario),
        getNowPlaying: () => clone({ ...scenario.track, isPlaying: ${input.playing}, positionMilliseconds }),
        getLyrics: () => clone(scenario.lyrics),
        subscribe: (subscriber) => {
          if (typeof subscriber !== "function") throw new TypeError("Preview subscriber must be a function");
          subscribers.add(subscriber);
          return () => subscribers.delete(subscriber);
        }
      });
      addEventListener("resize", fit);
      addEventListener("error", (event) => emit("error", event.message || "Theme preview error"));
      addEventListener("unhandledrejection", (event) => emit("error", String(event.reason || "Unhandled theme promise rejection")));
      addEventListener("securitypolicyviolation", (event) => emit("warning", "CSP blocked " + event.violatedDirective));
      addEventListener("DOMContentLoaded", () => {
        fit();
        hydrate();
        window.dispatchEvent(new CustomEvent("lyra:bridge-ready"));
        emit("ready", "Preview bridge ready");
        ${input.playing ? "setInterval(() => { positionMilliseconds = (positionMilliseconds + 100) % duration; renderTimeline(); }, 100);" : ""}
      });
    })();
  </script>
  <div id="lyra-preview-canvas">${markup}</div>
</body>
</html>`;
}

function defaultMarkup(
  scenario: PreviewScenario,
  current: PreviewScenario["lyrics"][number],
): string {
  return `<main class="lyra-blyrics-stage">
    <section class="lyra-track">
      <div class="lyra-art" aria-hidden="true">LYRA</div>
      <div class="lyra-meta"><strong data-lyra-track-title>${escapeHtml(scenario.track.title)}</strong><span data-lyra-track-artist>${escapeHtml(scenario.track.artist)}</span></div>
    </section>
    <section id="blyrics-wrapper" class="lyra-blyrics-viewport">
      <div class="blyrics-container">
        <div class="blyrics--line blyrics--active blyrics--animating"><span class="blyrics--word" data-lyra-current-line>${escapeHtml(current.text)}</span></div>
        <div class="blyrics--line lyra-translation" data-lyra-translation>${current.translation ? escapeHtml(current.translation) : ""}</div>
      </div>
    </section>
  </main>`;
}

function escapeStyleText(source: string): string {
  return source.replace(/<\/style/gi, "<\\/style");
}

function serializeForInlineScript(value: unknown): string {
  return JSON.stringify(value)
    .replace(/</g, "\\u003c")
    .replace(/>/g, "\\u003e")
    .replace(/&/g, "\\u0026")
    .replace(/\u2028/g, "\\u2028")
    .replace(/\u2029/g, "\\u2029");
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const BASE_PREVIEW_CSS = `
:root { color-scheme: dark; font-family: Inter, "PingFang SC", system-ui, sans-serif; }
* { box-sizing: border-box; }
html, body { width: 100%; height: 100%; margin: 0; overflow: hidden; background: transparent; }
body { color: #f4ffff; }
#lyra-preview-canvas { width: 4032px; height: 284px; overflow: hidden; transform: scale(var(--lyra-preview-scale, .25)); transform-origin: 0 0; }
.lyra-blyrics-stage { width: 100%; height: 100%; display: grid; grid-template-columns: 28% 72%; align-items: center; padding: 24px 56px; background: radial-gradient(circle at 68% 110%, rgb(58 76 126 / .42), transparent 50%), linear-gradient(105deg, #071019, #10152b 55%, #080d18); }
.lyra-track { min-width: 0; display: flex; align-items: center; gap: 22px; }
.lyra-art { width: 150px; aspect-ratio: 1; display: grid; place-items: center; border-radius: 22px; color: rgb(255 255 255 / .7); background: linear-gradient(145deg, #6e5da7, #25335c 58%, #0c7880); font-weight: 700; letter-spacing: .18em; box-shadow: 0 20px 60px rgb(0 0 0 / .4); }
.lyra-meta { min-width: 0; display: grid; gap: 8px; }
.lyra-meta strong { overflow: hidden; font-size: 32px; text-overflow: ellipsis; white-space: nowrap; }
.lyra-meta span { color: rgb(220 237 245 / .55); font-size: 22px; }
.lyra-blyrics-viewport { min-width: 0; overflow: hidden; }
.blyrics-container { width: 100%; display: grid; place-items: center; gap: 16px; text-align: center; }
.blyrics--line { max-width: 95%; opacity: .55; font-size: 30px; }
.blyrics--active { opacity: 1; font-size: 58px; font-weight: 700; text-shadow: 0 0 28px rgb(112 235 238 / .3); }
.lyra-translation { color: rgb(220 237 245 / .6); }
html[data-mode="day"] .lyra-blyrics-stage { background: radial-gradient(circle at 68% 110%, rgb(63 124 151 / .4), transparent 50%), linear-gradient(105deg, #10202d, #152640 55%, #101827); }
`;
