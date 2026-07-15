// @vitest-environment jsdom

import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { StrictMode } from "react";
import App from "./App";

afterEach(cleanup);

describe("Studio workspace", () => {
  it("keeps rendering when the safe-area control is toggled", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = screen.getByTestId("safe-area-toggle");
    await user.click(control);

    expect(screen.getByTestId("studio-shell")).toBeInTheDocument();
    expect(screen.queryByTestId("safe-area")).not.toBeInTheDocument();
  });

  it("supports exact numeric parameter entry", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const input = screen.getByTestId("parameter-fontSize-value");
    await user.clear(input);
    await user.type(input, "48");

    expect(input).toHaveValue(48);
    expect(screen.getByTestId("parameter-fontSize")).toHaveValue("48");
  });

  it("starts and stops the local device bridge without displaying provisioning data", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = await screen.findByTestId("device-bridge-control");
    expect(await screen.findByText("Bridge off")).toBeInTheDocument();
    expect(control).not.toHaveTextContent("Bearer");

    await user.click(screen.getByTestId("device-bridge-toggle"));
    expect(await screen.findByText("Waiting for Lyra")).toBeInTheDocument();

    await user.click(screen.getByTestId("device-bridge-toggle"));
    expect(await screen.findByText("Bridge off")).toBeInTheDocument();
  });

  it("requires explicit ADB selection before checking a device", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = await screen.findByTestId("device-adb-control");
    expect(await screen.findByText("ADB not configured")).toBeInTheDocument();
    const check = screen.getByTestId("device-adb-check");
    expect(check).toBeDisabled();
    expect(control).not.toHaveTextContent("/Users/");
    expect(control).not.toHaveTextContent("Bearer");

    await user.click(screen.getByTestId("device-adb-select"));
    expect(await screen.findByText("ADB selected")).toBeInTheDocument();
    expect(check).toBeEnabled();

    await user.click(check);
    expect(await screen.findByText("1 device ready")).toBeInTheDocument();
  });

  it("creates and removes an explicit mapping only after bridge and ADB preflight", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = await screen.findByTestId("device-mapping-control");
    const action = screen.getByTestId("device-mapping-toggle");
    expect(await screen.findByText("Mapping off")).toBeInTheDocument();
    expect(action).toBeDisabled();
    expect(control).not.toHaveTextContent("/Users/");
    expect(control).not.toHaveTextContent("Bearer");

    await user.click(screen.getByTestId("device-bridge-toggle"));
    expect(await screen.findByText("Waiting for Lyra")).toBeInTheDocument();
    await user.click(screen.getByTestId("device-adb-select"));
    expect(await screen.findByText("ADB selected")).toBeInTheDocument();
    await user.click(screen.getByTestId("device-adb-check"));
    expect(await screen.findByText("1 device ready")).toBeInTheDocument();
    expect(action).toBeEnabled();

    await user.click(action);
    expect(await screen.findByText("Mapping active")).toBeInTheDocument();
    expect(action).toHaveTextContent("Remove mapping");

    await user.click(action);
    expect(await screen.findByText("Mapping off")).toBeInTheDocument();
  });

  it("generates project controls from the parameter schema with undo support", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    await user.click(screen.getByTestId("open-project"));
    await user.click(await screen.findByTestId("inspector-design"));
    const size = await screen.findByTestId("schema-parameter-font-size-value");
    await user.clear(size);
    await user.type(size, "48");

    await user.click(screen.getByTestId("inspector-source"));
    expect((screen.getByTestId("source-editor") as HTMLTextAreaElement).value).toContain("--lyra-font-size: 48px;");

    await user.click(screen.getByTestId("inspector-design"));
    await user.click(screen.getByTestId("undo-parameter"));
    expect(screen.getByTestId("schema-parameter-font-size-value")).toHaveValue(42);
  });

  it("edits manifest-declared source documents with find, replace, and diagnostics", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    await user.click(screen.getByTestId("open-project"));
    expect(await screen.findByTestId("source-document-style")).toBeInTheDocument();
    await user.click(screen.getByTestId("source-document-parameters"));
    expect((screen.getByTestId("source-editor") as HTMLTextAreaElement).value).toContain("schemaVersion");

    await user.type(screen.getByTestId("source-find"), "schemaVersion");
    await user.type(screen.getByTestId("source-replace"), "contractVersion");
    await user.click(screen.getByTestId("source-replace-all"));
    expect((screen.getByTestId("source-editor") as HTMLTextAreaElement).value).toContain("contractVersion");

    await user.clear(screen.getByTestId("source-editor"));
    await user.paste("{");
    expect(await screen.findByTestId("source-diagnostic")).toHaveTextContent("JSON");
  });

  it("renders project themes inside an opaque-origin scenario preview", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    await user.click(screen.getByTestId("open-project"));
    const frame = await screen.findByTestId("preview-frame");

    expect(frame).toHaveAttribute("sandbox", "allow-scripts");
    expect(frame).not.toHaveAttribute("sandbox", expect.stringContaining("allow-same-origin"));
    expect(frame.getAttribute("srcdoc")).toContain("default-src 'none'");
    expect(frame.getAttribute("srcdoc")).toContain("Midnight Galaxy");
    expect(frame.getAttribute("srcdoc")).toContain("window.lyraBridge");
  });

  it("refreshes the preview from an edited scenario document", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    await user.click(screen.getByTestId("open-project"));
    await user.click(await screen.findByTestId("source-document-scenario-0"));
    const editor = screen.getByTestId("source-editor");
    await user.clear(editor);
    await user.paste(JSON.stringify({
      schemaVersion: 1,
      id: "org.lyra.scenario.midnight-galaxy",
      track: { title: "Edited in Studio", artist: "Future Echoes" },
      lyrics: [{ startMilliseconds: 0, endMilliseconds: 4000, text: "即时预览" }],
      events: [],
    }));

    expect(screen.getByTestId("preview-frame").getAttribute("srcdoc")).toContain("Edited in Studio");
  });
});
