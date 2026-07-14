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
});
