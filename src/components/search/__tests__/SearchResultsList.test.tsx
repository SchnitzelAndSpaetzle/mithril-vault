// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { SearchResult } from "@/lib/search-utils";
import type { Entry, Group } from "@/lib/types";
import SearchResultsList from "../SearchResultsList";

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  updateTabState: vi.fn(),
  scrollToIndex: vi.fn(),
  isMobile: false,
  selectedEntryId: null as string | null,
  groups: [] as Group[],
}));

interface MockVirtualizerOptions {
  count: number;
}

interface MockVirtualItem {
  index: number;
  start: number;
}

interface MockSearchResultItemProps {
  result: SearchResult;
  query: string;
  groupPath: string;
  isSelected: boolean;
  onClick: (id: string) => void;
}

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (options: MockVirtualizerOptions) => ({
    getTotalSize: () => options.count * 80,
    getVirtualItems: (): MockVirtualItem[] =>
      Array.from({ length: options.count }, (_, index) => ({
        index,
        start: index * 80,
      })),
    scrollToIndex: mocks.scrollToIndex,
    measureElement: vi.fn(),
  }),
}));

vi.mock("@/components/search/SearchResultItem", () => ({
  default: ({
    result,
    query,
    groupPath,
    isSelected,
    onClick,
  }: MockSearchResultItemProps) => (
    <button
      type="button"
      onClick={() => onClick(result.entry.id)}
      data-selected={isSelected}
    >
      {`${result.entry.title}|${groupPath}|${query}`}
    </button>
  ),
}));

vi.mock("@/hooks/use-active-database", () => ({
  useActiveDatabase: () => ({
    dbId: "db-1",
    tab: { id: "tab-1", selectedEntryId: mocks.selectedEntryId },
  }),
}));

vi.mock("@/hooks/use-groups", () => ({
  useGroups: () => ({ data: mocks.groups }),
}));

vi.mock("@/hooks/use-custom-icons", () => ({
  useCustomIcons: () => ({ data: {} }),
}));

vi.mock("@/hooks/use-mobile", () => ({
  useIsMobile: () => mocks.isMobile,
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mocks.navigate,
}));

vi.mock("@/stores/database-tabs", () => ({
  useDatabaseTabs: (
    selector: (state: {
      updateTabState: typeof mocks.updateTabState;
    }) => (tabId: string, updates: { selectedEntryId: string }) => void
  ) => selector({ updateTabState: mocks.updateTabState }),
}));

function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: "entry-1",
    groupId: "group-work",
    title: "GitHub",
    username: "user",
    url: null,
    notes: null,
    iconId: 0,
    customIconUuid: null,
    tags: [],
    customFields: {},
    customFieldMeta: [],
    createdAt: "2024-01-01T00:00:00Z",
    modifiedAt: "2024-01-01T00:00:00Z",
    accessedAt: "2024-01-01T00:00:00Z",
    ...overrides,
  };
}

function makeResult(overrides: Partial<Entry> = {}): SearchResult {
  return {
    entry: makeEntry(overrides),
    matchedFields: ["title"],
  };
}

describe("SearchResultsList", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.isMobile = false;
    mocks.selectedEntryId = null;
    mocks.groups = [];
  });

  it("shows empty state when no results", () => {
    render(<SearchResultsList results={[]} query="test" />);

    expect(screen.getByText("entries.search.noResults")).toBeInTheDocument();
  });

  it("renders virtualized items and group path", () => {
    mocks.groups = [
      {
        id: "group-root",
        parentId: null,
        name: "Root",
        icon: null,
        customIconUuid: null,
        children: [
          {
            id: "group-work",
            parentId: "group-root",
            name: "Work",
            icon: null,
            customIconUuid: null,
            children: [],
          },
        ],
      },
    ];

    render(<SearchResultsList results={[makeResult()]} query="git" />);

    expect(screen.getByRole("listbox")).toBeInTheDocument();
    expect(screen.getByText("GitHub|Root > Work|git")).toBeInTheDocument();
  });

  it("calls provided onEntrySelect when clicking an item", async () => {
    const onEntrySelect = vi.fn();

    render(
      <SearchResultsList
        results={[makeResult({ id: "entry-click" })]}
        query="git"
        onEntrySelect={onEntrySelect}
      />
    );

    fireEvent.click(screen.getByText("GitHub||git"));

    await waitFor(() => {
      expect(onEntrySelect).toHaveBeenCalledWith("entry-click");
    });
    expect(mocks.navigate).not.toHaveBeenCalled();
  });

  it("updates tab selection when onEntrySelect is not provided", () => {
    render(
      <SearchResultsList
        results={[makeResult({ id: "entry-keyboard" })]}
        query="git"
      />
    );

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "ArrowDown" });

    expect(mocks.updateTabState).toHaveBeenCalledWith("tab-1", {
      selectedEntryId: "entry-keyboard",
    });
    expect(mocks.scrollToIndex).toHaveBeenCalledWith(0, { align: "auto" });
  });

  it("navigates to mobile detail on click", async () => {
    mocks.isMobile = true;
    const onEntrySelect = vi.fn();

    render(
      <SearchResultsList
        results={[makeResult({ id: "entry-mobile" })]}
        query="git"
        onEntrySelect={onEntrySelect}
      />
    );

    fireEvent.click(screen.getByText("GitHub||git"));

    await waitFor(() => {
      expect(onEntrySelect).toHaveBeenCalledWith("entry-mobile");
      expect(mocks.navigate).toHaveBeenCalledWith({
        to: "/dashboard/entry/$id",
        params: { id: "entry-mobile" },
      });
    });
  });

  it("navigates on Enter when selected entry exists on mobile", () => {
    mocks.isMobile = true;
    mocks.selectedEntryId = "entry-enter";

    render(
      <SearchResultsList
        results={[
          makeResult({ id: "entry-enter", title: "Mail" }),
          makeResult({ id: "entry-other", title: "Other" }),
        ]}
        query="mail"
      />
    );

    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Enter" });

    expect(mocks.navigate).toHaveBeenCalledWith({
      to: "/dashboard/entry/$id",
      params: { id: "entry-enter" },
    });
  });
});
