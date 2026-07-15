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
});
