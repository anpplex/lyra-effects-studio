import { useMemo, useState, type ReactNode } from "react";
import "./App.css";
import {
  createStudioState,
  selectPack,
  selectTheme,
  setPreviewMode,
  updateParameter,
  type InspectorTab,
  type StudioParameters,
} from "./studio/model";

type NumericParameter = Exclude<keyof StudioParameters, "showSafeArea">;

type IconName =
  | "chevron"
  | "cloud"
  | "code"
  | "download"
  | "folder"
  | "info"
  | "layers"
  | "moon"
  | "pause"
  | "play"
  | "refresh"
  | "search"
  | "sliders"
  | "sun"
  | "warning";

const paths: Record<IconName, ReactNode> = {
  chevron: <path d="m9 18 6-6-6-6" />,
  cloud: <><path d="M17.5 19H9a7 7 0 1 1 6.7-9h1.8a4.5 4.5 0 1 1 0 9Z" /><path d="m12 12-3 3 3 3M9 15h8" /></>,
  code: <><path d="m8 9-4 3 4 3M16 9l4 3-4 3M14 5l-4 14" /></>,
  download: <><path d="M12 3v12m0 0 4-4m-4 4-4-4" /><path d="M5 19h14" /></>,
  folder: <path d="M3 6.5h6l2 2h10v9.5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />,
  info: <><circle cx="12" cy="12" r="9" /><path d="M12 11v6m0-9h.01" /></>,
  layers: <><path d="m12 3 9 5-9 5-9-5Z" /><path d="m3 12 9 5 9-5M3 16l9 5 9-5" /></>,
  moon: <path d="M20 15.5A8.5 8.5 0 0 1 8.5 4 8.5 8.5 0 1 0 20 15.5Z" />,
  pause: <><path d="M9 7v10M15 7v10" /></>,
  play: <path d="m9 7 8 5-8 5Z" />,
  refresh: <><path d="M20 7v5h-5" /><path d="M19 12a7 7 0 1 0-2 5" /></>,
  search: <><circle cx="11" cy="11" r="7" /><path d="m16 16 4 4" /></>,
  sliders: <><path d="M4 7h9M17 7h3M4 17h3M11 17h9" /><circle cx="15" cy="7" r="2" /><circle cx="9" cy="17" r="2" /></>,
  sun: <><circle cx="12" cy="12" r="3.5" /><path d="M12 2v2m0 16v2M4.9 4.9l1.4 1.4m11.4 11.4 1.4 1.4M2 12h2m16 0h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>,
  warning: <><path d="M12 3 2.8 20h18.4Z" /><path d="M12 9v5m0 3h.01" /></>,
};

function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return <svg className="icon" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>;
}

function ParameterSlider({
  label,
  name,
  min,
  max,
  step = 1,
  value,
  suffix,
  onChange,
}: {
  label: string;
  name: NumericParameter;
  min: number;
  max: number;
  step?: number;
  value: number;
  suffix: string;
  onChange: (key: NumericParameter, value: number) => void;
}) {
  return (
    <div className="parameter-control">
      <span><label htmlFor={`parameter-${name}`}>{label}</label><span className="parameter-value"><input data-testid={`parameter-${name}-value`} aria-label={`${label} exact value`} type="number" min={min} max={max} step={step} value={value} onChange={(event) => onChange(name, Number(event.currentTarget.value))} /><span>{suffix.trim()}</span></span></span>
      <input id={`parameter-${name}`} aria-label={label} data-testid={`parameter-${name}`} type="range" min={min} max={max} step={step} value={value} onChange={(event) => onChange(name, Number(event.currentTarget.value))} />
    </div>
  );
}

function App() {
  const [state, setState] = useState(createStudioState);
  const [search, setSearch] = useState("");
  const [consoleTab, setConsoleTab] = useState<"timeline" | "events" | "diagnostics">("events");

  const selectedPack = state.packs.find((pack) => pack.id === state.selectedPackId)!;
  const selectedTheme = selectedPack.themes.find((theme) => theme.id === state.selectedThemeId)!;
  const errorCount = state.diagnostics.filter((item) => item.level === "error").length;
  const visiblePacks = useMemo(() => {
    const query = search.trim().toLocaleLowerCase();
    if (!query) return state.packs;
    return state.packs
      .map((pack) => ({ ...pack, themes: pack.themes.filter((theme) => `${pack.name} ${theme.name}`.toLocaleLowerCase().includes(query)) }))
      .filter((pack) => pack.name.toLocaleLowerCase().includes(query) || pack.themes.length > 0);
  }, [search, state.packs]);

  const handleParameter = (key: NumericParameter, value: number) => {
    setState((current) => updateParameter(current, key, value));
  };

  const previewStyle = {
    "--lyric-size": `${state.parameters.fontSize}px`,
    "--lyric-glow": `${state.parameters.glow / 10}px`,
    "--lyric-zone": `${state.parameters.rightZone}%`,
    "--lyric-motion": `${state.parameters.motion}s`,
  } as React.CSSProperties;

  return (
    <main className="studio-shell" data-testid="studio-shell">
      <header className="app-bar" data-tauri-drag-region>
        <div className="brand-block">
          <div className="brand-mark" aria-hidden="true"><span /><span /><span /></div>
          <div><strong>Lyra Effects Studio</strong><span>future-lyrics / {selectedTheme.name}</span></div>
        </div>
        <div className="app-toolbar">
          <label className="select-control"><span className="sr-only">Device profile</span><select defaultValue="avatr-cluster-4032x284"><option value="avatr-cluster-4032x284">Avatr Cluster · 4032 × 284</option><option value="browser-responsive">Responsive browser</option></select></label>
          <div className="segmented" aria-label="Preview appearance">
            <button data-testid="mode-day" className={state.preview.mode === "day" ? "active" : ""} aria-pressed={state.preview.mode === "day"} onClick={() => setState((current) => setPreviewMode(current, "day"))}><Icon name="sun" />Day</button>
            <button data-testid="mode-night" className={state.preview.mode === "night" ? "active" : ""} aria-pressed={state.preview.mode === "night"} onClick={() => setState((current) => setPreviewMode(current, "night"))}><Icon name="moon" />Night</button>
          </div>
          <button className="icon-button" aria-label="Refresh preview"><Icon name="refresh" /></button>
          <button className="play-button" data-testid="play-toggle" onClick={() => setState((current) => ({ ...current, preview: { ...current.preview, playing: !current.preview.playing } }))}><Icon name={state.preview.playing ? "pause" : "play"} />{state.preview.playing ? "Pause" : "Play"}</button>
        </div>
        <div className="publish-block"><span className="connection-status"><i />Local preview</span><button className="primary-button"><Icon name="download" />Build pack</button></div>
      </header>

      <div className="workspace">
        <aside className="library-panel">
          <div className="panel-heading"><span>Library</span><button className="icon-button small" aria-label="Open project"><Icon name="folder" /></button></div>
          <label className="search-control"><Icon name="search" /><span className="sr-only">Search themes</span><input value={search} onChange={(event) => setSearch(event.currentTarget.value)} placeholder="Search themes" /></label>
          <nav className="pack-tree" aria-label="Theme library">
            {visiblePacks.map((pack) => (
              <section className="pack-group" key={pack.id}>
                <button className={`pack-row ${pack.id === state.selectedPackId ? "selected" : ""}`} onClick={() => setState((current) => selectPack(current, pack.id))}><Icon name="chevron" size={13} /><span className="pack-icon"><Icon name="layers" size={14} /></span><span><strong>{pack.name}</strong><small>{pack.themes.length} themes</small></span></button>
                <div className="theme-list">
                  {pack.themes.map((theme) => (
                    <button key={theme.id} data-testid={`theme-${theme.id}`} className={theme.id === state.selectedThemeId ? "active" : ""} onClick={() => setState((current) => selectTheme(current, theme.id))}><span className={`theme-swatch swatch-${theme.id}`} /><span><strong>{theme.name}</strong><small>v{theme.version}</small></span>{theme.status === "draft" && <em>Draft</em>}</button>
                  ))}
                </div>
              </section>
            ))}
          </nav>
          <div className="library-footer"><span><Icon name="cloud" />GitHub Registry</span><strong>3 packs · synced</strong></div>
        </aside>

        <section className="stage-panel">
          <div className="stage-heading"><div><span className="eyebrow">Live preview</span><strong>{selectedPack.name} / {selectedTheme.name}</strong></div><div className="stage-actions"><span className="fps"><i />60 FPS</span><button className="quiet-button">Fit</button><button className="quiet-button">100%</button></div></div>
          <div className={`preview-workspace ${state.preview.mode}`}>
            <div className="preview-rulers ruler-top"><span>0</span><span>1008</span><span>2016</span><span>3024</span><span>4032</span></div>
            <div className="preview-rulers ruler-left"><span>0</span><span>142</span><span>284</span></div>
            <div className="device-label"><i />AVATR CLUSTER / MAIN DISPLAY</div>
            <div className="cluster-frame">
              <div className="cluster-screen" data-testid="preview-canvas" style={previewStyle}>
                <div className="ambient-orbit orbit-one" /><div className="ambient-orbit orbit-two" />
                <div className="album-block"><div className="album-art"><span>LYRA</span></div><div className="track-meta"><strong>Midnight Galaxy</strong><span>Future Echoes</span></div></div>
                <div className="lyric-zone"><p className="previous-line">We were chasing light through the silence</p><p className="current-line">星河在此刻为你闪烁</p><p className="translation-line">The galaxy is shimmering for you</p></div>
                {state.parameters.showSafeArea && <div className="safe-area" data-testid="safe-area"><span>safe area</span></div>}
                <div className="progress-line"><span style={{ width: state.preview.playing ? "62%" : "38%" }} /></div>
              </div>
            </div>
            <div className="zoom-note">4032 × 284 px <span>·</span> 14.2:1 <span>·</span> sRGB</div>
          </div>

          <div className="console-panel">
            <div className="console-tabs" role="tablist" aria-label="Preview tools">
              {(["timeline", "events", "diagnostics"] as const).map((tab) => <button key={tab} role="tab" aria-selected={consoleTab === tab} className={consoleTab === tab ? "active" : ""} onClick={() => setConsoleTab(tab)}>{tab === "timeline" ? "Timeline" : tab === "events" ? "Bridge events" : `Diagnostics (${errorCount})`}</button>)}
              <span className="console-spacer" /><button className="console-clear">Clear</button>
            </div>
            <div className="console-body">
              {consoleTab === "timeline" ? <div className="timeline"><div className="time-track"><span style={{ width: state.preview.playing ? "62%" : "38%" }} /><i style={{ left: state.preview.playing ? "62%" : "38%" }} /></div><div><span>00:00</span><span>00:18</span><span>00:36</span></div></div> : state.diagnostics.map((item) => <div className="log-line" key={item.id}><time>{item.time}</time><span className={`log-level ${item.level}`}>{item.level === "warning" ? "WARN" : item.level.toUpperCase()}</span><span>{consoleTab === "events" && item.id === "bridge" ? "theme.render completed in 5.4ms" : item.message}</span></div>)}
            </div>
          </div>
        </section>

        <aside className="inspector-panel">
          <div className="inspector-tabs" role="tablist" aria-label="Inspector">
            {(["design", "source", "diagnostics"] as InspectorTab[]).map((tab) => <button key={tab} data-testid={`inspector-${tab}`} role="tab" aria-selected={state.inspectorTab === tab} className={state.inspectorTab === tab ? "active" : ""} onClick={() => setState((current) => ({ ...current, inspectorTab: tab }))}>{tab === "design" ? <Icon name="sliders" /> : tab === "source" ? <Icon name="code" /> : <Icon name="warning" />}{tab === "design" ? "Design" : tab === "source" ? "Source" : "Issues"}</button>)}
          </div>
          {state.inspectorTab === "design" && <div className="inspector-content">
            <section className="inspector-section"><div className="section-title"><span>Typography</span><Icon name="chevron" size={13} /></div><ParameterSlider label="Font size" name="fontSize" min={28} max={64} value={state.parameters.fontSize} suffix=" px" onChange={handleParameter} /><ParameterSlider label="Right zone" name="rightZone" min={35} max={68} value={state.parameters.rightZone} suffix="%" onChange={handleParameter} /></section>
            <section className="inspector-section"><div className="section-title"><span>Light & motion</span><Icon name="chevron" size={13} /></div><ParameterSlider label="Glow" name="glow" min={0} max={36} value={state.parameters.glow} suffix="%" onChange={handleParameter} /><ParameterSlider label="Motion" name="motion" min={0.2} max={2} step={0.1} value={state.parameters.motion} suffix=" s" onChange={handleParameter} /></section>
            <section className="inspector-section"><div className="section-title"><span>Guides</span><Icon name="chevron" size={13} /></div><label className="switch-row"><span><strong>Safe area</strong><small>Show protected rendering bounds</small></span><input data-testid="safe-area-toggle" type="checkbox" checked={state.parameters.showSafeArea} onChange={(event) => { const checked = event.currentTarget.checked; setState((current) => updateParameter(current, "showSafeArea", checked)); }} /><i /></label></section>
            <section className="inspector-section compact"><div className="section-title"><span>Theme metadata</span><Icon name="chevron" size={13} /></div><dl className="metadata"><div><dt>Contract</dt><dd>lyra.pack/v1</dd></div><div><dt>Theme</dt><dd>{selectedTheme.id}</dd></div><div><dt>Version</dt><dd>{selectedTheme.version}</dd></div><div><dt>License</dt><dd>MIT</dd></div></dl></section>
          </div>}
          {state.inspectorTab === "source" && <div className="source-panel"><div className="source-path">themes/{selectedTheme.id}/theme.css</div><pre><code><span className="code-comment">{`/* Generated parameter patch */`}</span>{`\n:root {\n  --lyra-font-size: `}<b>{state.parameters.fontSize}px</b>{`;\n  --lyra-right-zone: `}<b>{state.parameters.rightZone}%</b>{`;\n  --lyra-glow: `}<b>{state.parameters.glow}%</b>{`;\n  --lyra-motion: `}<b>{state.parameters.motion}s</b>{`;\n}`}</code></pre><div className="source-note"><Icon name="info" /><span>Only changed variables are written. Formatting and unknown fields stay intact.</span></div></div>}
          {state.inspectorTab === "diagnostics" && <div className="issues-panel"><div className="issue-summary"><strong data-testid="diagnostic-error-count">{errorCount}</strong><span>errors</span><strong>{state.diagnostics.filter((item) => item.level === "warning").length}</strong><span>warnings</span></div>{state.diagnostics.filter((item) => item.level !== "info").map((item) => <article className={`issue-card ${item.level}`} key={item.id}><Icon name="warning" /><div><strong>{item.message}</strong><p>Preview-only advisory. Pack validation remains green.</p></div></article>)}</div>}
        </aside>
      </div>

      <footer className="status-bar"><span><i className="status-dot" />Ready</span><span>Pack contract v1</span><span>Project contract v1</span><span className="status-grow" /><span>UTF-8</span><span>Spaces: 2</span><span>Lyra Studio 0.1.0-alpha.1</span></footer>
    </main>
  );
}

export default App;
