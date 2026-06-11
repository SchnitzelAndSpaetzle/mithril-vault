// SPDX-License-Identifier: MIT

import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import SortDropdown from "./sort-dropdown";

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  search: {} as { hasAttachments?: boolean },
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

function openMenu() {
  // The trigger's label changes when the filter is active, so find it
  // by role rather than by a fixed label.
  const trigger = screen.getByRole("button");
  fireEvent.pointerDown(
    trigger,
    new MouseEvent("pointerdown", { bubbles: true, button: 0 })
  );
  fireEvent.click(trigger);
}

describe("SortDropdown has-attachments filter", () => {
  it("enables the filter via the URL search param when toggled on", () => {
    mocks.search = {};
    render(<SortDropdown />);

    openMenu();
    fireEvent.click(
      screen.getByRole("menuitemcheckbox", {
        name: "entries.filter.hasAttachments",
      })
    );

    expect(mocks.navigate).toHaveBeenCalledTimes(1);
    const updater = mocks.navigate.mock.calls[0]![0].search as (
      prev: Record<string, unknown>
    ) => Record<string, unknown>;
    expect(updater({ sortBy: "title" })).toEqual({
      sortBy: "title",
      hasAttachments: true,
    });
  });

  it("clears the filter param when toggled off", () => {
    mocks.search = { hasAttachments: true };
    render(<SortDropdown />);

    openMenu();
    fireEvent.click(
      screen.getByRole("menuitemcheckbox", {
        name: "entries.filter.hasAttachments",
      })
    );

    const updater = mocks.navigate.mock.calls[0]![0].search as (
      prev: Record<string, unknown>
    ) => Record<string, unknown>;
    expect(updater({ sortBy: "title", hasAttachments: true })).toEqual({
      sortBy: "title",
      hasAttachments: undefined,
    });
  });

  it("accents the trigger and updates its aria-label while the filter is active", () => {
    mocks.search = { hasAttachments: true };
    render(<SortDropdown />);

    const trigger = screen.getByLabelText("entries.sort.sortEntriesFiltered");
    expect(trigger.className).toContain("border-primary");
  });

  it("leaves the trigger unaccented with the plain label when inactive", () => {
    mocks.search = {};
    render(<SortDropdown />);

    const trigger = screen.getByLabelText("entries.sort.sortEntries");
    expect(trigger.className).not.toContain("border-primary");
  });

  it("reflects the active filter as a checked item", () => {
    mocks.search = { hasAttachments: true };
    render(<SortDropdown />);

    openMenu();
    expect(
      screen
        .getByRole("menuitemcheckbox", {
          name: "entries.filter.hasAttachments",
        })
        .getAttribute("aria-checked")
    ).toBe("true");
  });
});
