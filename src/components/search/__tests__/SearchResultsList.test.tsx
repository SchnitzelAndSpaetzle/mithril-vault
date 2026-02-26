// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import SearchResultsList from "../SearchResultsList";

vi.mock("@/hooks/use-active-database", () => ({
  useActiveDatabase: () => ({
    dbId: "db-1",
    tab: { id: "tab-1", selectedEntryId: null },
  }),
}));

vi.mock("@/hooks/use-groups", () => ({
  useGroups: () => ({ data: [] }),
}));

vi.mock("@/hooks/use-custom-icons", () => ({
  useCustomIcons: () => ({ data: {} }),
}));

vi.mock("@/hooks/use-mobile", () => ({
  useIsMobile: () => false,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock("@/stores/database-tabs", () => ({
  useDatabaseTabs: () => vi.fn(),
}));

describe("SearchResultsList", () => {
  it("shows empty state when no results", () => {
    render(<SearchResultsList results={[]} query="test" />);

    expect(
      screen.getByText("No entries match your search.")
    ).toBeInTheDocument();
  });

  it("renders listbox role when there are results", () => {
    // Virtualizer won't render items in jsdom (no scroll dimensions),
    // but we can verify the container structure
    const results = [
      {
        entry: {
          id: "1",
          groupId: "g1",
          title: "GitHub",
          username: "user",
          url: null,
          notes: null,
          iconId: 0,
          customIconUuid: null,
          tags: [] as string[],
          customFields: {},
          customFieldMeta: [],
          createdAt: "2024-01-01T00:00:00Z",
          modifiedAt: "2024-01-01T00:00:00Z",
          accessedAt: "2024-01-01T00:00:00Z",
        },
        matchedFields: ["title" as const],
      },
    ];

    render(<SearchResultsList results={results} query="g" />);

    expect(screen.getByRole("listbox")).toBeInTheDocument();
  });
});
