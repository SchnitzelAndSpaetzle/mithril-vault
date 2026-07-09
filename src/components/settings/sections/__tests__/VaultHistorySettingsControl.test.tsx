// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { VaultHistorySettingsControl } from "../VaultHistorySettingsControl";

describe("VaultHistorySettingsControl", () => {
  it("reflects an absent limit as 'keep newest' with the default of 10", () => {
    render(<VaultHistorySettingsControl maxItems={null} onChange={vi.fn()} />);

    const keep = screen.getByRole("radio", {
      name: "settings.database.history.keepNewest",
    });
    expect(keep).toBeChecked();
    expect(screen.getByRole("spinbutton")).toHaveValue(10);
  });

  it("selecting Unlimited persists a negative value", () => {
    const onChange = vi.fn();
    render(<VaultHistorySettingsControl maxItems={10} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("radio", {
        name: "settings.database.history.unlimited",
      })
    );
    expect(onChange).toHaveBeenCalledWith(-1);
  });

  it("selecting Disabled persists zero", () => {
    const onChange = vi.fn();
    render(<VaultHistorySettingsControl maxItems={10} onChange={onChange} />);

    fireEvent.click(
      screen.getByRole("radio", {
        name: "settings.database.history.disabled",
      })
    );
    expect(onChange).toHaveBeenCalledWith(0);
  });

  it("editing the count persists the new positive limit", () => {
    const onChange = vi.fn();
    render(<VaultHistorySettingsControl maxItems={10} onChange={onChange} />);

    fireEvent.change(screen.getByRole("spinbutton"), {
      target: { value: "25" },
    });
    expect(onChange).toHaveBeenCalledWith(25);
  });

  it("reflects unlimited and disables the count field", () => {
    render(<VaultHistorySettingsControl maxItems={-1} onChange={vi.fn()} />);

    expect(
      screen.getByRole("radio", {
        name: "settings.database.history.unlimited",
      })
    ).toBeChecked();
    expect(screen.getByRole("spinbutton")).toBeDisabled();
  });

  it("reflects disabled when the limit is zero", () => {
    render(<VaultHistorySettingsControl maxItems={0} onChange={vi.fn()} />);

    expect(
      screen.getByRole("radio", {
        name: "settings.database.history.disabled",
      })
    ).toBeChecked();
  });
});
