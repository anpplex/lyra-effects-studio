import { describe, expect, it } from "vitest";
import { buildPreviewDocument, type PreviewScenario } from "./previewDocument";

const scenario: PreviewScenario = {
  schemaVersion: 1,
  id: "org.lyra.preview.test",
  track: { title: "Midnight Galaxy", artist: "Future Echoes" },
  lyrics: [
    { startMilliseconds: 0, endMilliseconds: 4000, text: "星河在此刻为你闪烁", translation: "The galaxy is shimmering for you" },
  ],
  events: [],
  expectedDiagnostics: [],
};

describe("isolated preview document", () => {
  it("uses an opaque-origin CSP and only grants the injected bridge nonce", () => {
    const document = buildPreviewDocument({
      css: ":root { --accent: cyan; }",
      html: "<main id=\"blyrics-wrapper\"><script>window.evil = true</script></main>",
      scenario,
      mode: "night",
      playing: true,
      nonce: "random-preview-nonce",
    });

    expect(document).toContain("default-src 'none'");
    expect(document).toContain("connect-src 'none'");
    expect(document).toContain("script-src 'nonce-random-preview-nonce'");
    expect(document).toContain('<script nonce="random-preview-nonce">');
    expect(document).toContain("window.lyraBridge");
    expect(document).toContain("renderTimeline");
    expect(document).toContain("subscribers.add(subscriber)");
    expect(document).toContain("data-lyra-current-line");
    expect(document).toContain('parent.postMessage({ source: "lyra-preview"');
    expect(document).toContain("securitypolicyviolation");
    expect(document).toContain('id="lyra-preview-canvas"');
    expect(document).toContain("width: 4032px; height: 284px");
    expect(document).toContain("window.evil = true");
    expect(document).not.toContain("script-src 'unsafe-inline'");
  });

  it("escapes style end tags and serializes scenario text without creating markup", () => {
    const document = buildPreviewDocument({
      css: "body::after { content: '</style><script>bad()</script>'; }",
      scenario: { ...scenario, track: { ...scenario.track, title: "<img src=x>" } },
      mode: "day",
      playing: false,
      nonce: "nonce",
    });

    expect(document).not.toContain("</style><script>bad()");
    expect(document).toContain("<\\/style><script>bad()");
    expect(document).toContain("\\u003cimg src=x\\u003e");
  });
});
