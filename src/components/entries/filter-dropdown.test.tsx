// SPDX-License-Identifier: MIT

import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import FilterDropdown from "./filter-dropdown";

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  search: {},
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mocks.navigate,
  useSearch: () => mocks.search,
}));

vi.mock("@/hooks/use-active-database", () => ({
  useActiveDatabase: () => ({ dbId: "db-1" }),
}));

// Radix's DropdownMenu uses pointer-capture and scroll APIs that jsdom
// doesn't implement; stub them so the menu can open in tests.
beforeAll(() => {
  Element.prototype.hasPointerCapture = vi.fn(() => false);
  Element.prototype.releasePointerCapture = vi.fn();
  Element.prototype.scrollIntoView = vi.fn();
});

beforeEach(() => {
  mocks.navigate.mockReset();
  mocks.search = {};
});

function openMenuAndToggle() {
  const trigger = screen.getByLabelText("entries.filter.filterEntries");
  fireEvent.pointerDown(
    trigger,
    new MouseEvent("pointerdown", { bubbles: true, button: 0 })
  );
  fireEvent.click(trigger);
  const item = screen.getByRole("menuitemcheckbox", {
    name: "entries.filter.hasAttachments",
  });
  fireEvent.click(item);
}

describe("FilterDropdown", () => {
  it("renders the trigger with an accessible label", () => {
    render(<FilterDropdown />);
    expect(
      screen.getByLabelText("entries.filter.filterEntries")
    ).toBeInTheDocument();
  });

  it("accents the trigger while the has-attachments filter is active", () => {
    mocks.search = { hasAttachments: true };
    render(<FilterDropdown />);
    const trigger = screen.getByLabelText("entries.filter.filterEntries");
    expect(trigger.className).toContain("border-primary");
  });

  it("does not accent the trigger when no filter is active", () => {
    mocks.search = {};
    render(<FilterDropdown />);
    const trigger = screen.getByLabelText("entries.filter.filterEntries");
    expect(trigger.className).not.toContain("border-primary");
  });

  it("enables the filter via the URL search param when toggled on", () => {
    mocks.search = {};
    render(<FilterDropdown />);

    openMenuAndToggle();

    expect(mocks.navigate).toHaveBeenCalledTimes(1);
    const updater = mocks.navigate.mock.calls[0]![0].search as (
      prev: Record<string, unknown>
    ) => Record<string, unknown>;
    expect(updater({ tag: "dev" })).toEqual({
      tag: "dev",
      hasAttachments: true,
    });
  });

  it("clears the filter param when toggled off", () => {
    mocks.search = { hasAttachments: true };
    render(<FilterDropdown />);

    openMenuAndToggle();

    const updater = mocks.navigate.mock.calls[0]![0].search as (
      prev: Record<string, unknown>
    ) => Record<string, unknown>;
    expect(updater({ tag: "dev", hasAttachments: true })).toEqual({
      tag: "dev",
      hasAttachments: undefined,
    });
  });
});
