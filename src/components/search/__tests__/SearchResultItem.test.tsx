// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import SearchResultItem from "../SearchResultItem";
import type { SearchResult } from "@/lib/search-utils";
import type { Entry } from "@/lib/types";

function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: "entry-1",
    groupId: "group-1",
    title: "GitHub Login",
    username: "admin@github.com",
    url: "https://github.com",
    notes: null,
    iconId: 0,
    customIconUuid: null,
    tags: [],
    customFields: {},
    customFieldMeta: [],
    createdAt: "2024-01-01T00:00:00Z",
    modifiedAt: "2024-01-01T00:00:00Z",
    accessedAt: "2024-01-01T00:00:00Z",
    expires: false,
    attachments: [],
    ...overrides,
  };
}

function makeResult(entryOverrides: Partial<Entry> = {}): SearchResult {
  return {
    entry: makeEntry(entryOverrides),
    matchedFields: ["title"],
  };
}

describe("SearchResultItem", () => {
  it("renders title and username text content", () => {
    const { container } = render(
      <SearchResultItem
        result={makeResult()}
        query=""
        groupPath="Root > Work"
        customIcons={{}}
        isSelected={false}
        onClick={vi.fn()}
      />
    );

    expect(container.textContent).toContain("GitHub Login");
    expect(container.textContent).toContain("admin@github.com");
  });

  it("displays group path", () => {
    render(
      <SearchResultItem
        result={makeResult()}
        query=""
        groupPath="Root > Work > Dev"
        customIcons={{}}
        isSelected={false}
        onClick={vi.fn()}
      />
    );

    expect(screen.getByText("Root > Work > Dev")).toBeInTheDocument();
  });

  it("calls onClick with entry id when clicked", () => {
    const onClick = vi.fn();
    const { container } = render(
      <SearchResultItem
        result={makeResult({ id: "test-id" })}
        query=""
        groupPath=""
        customIcons={{}}
        isSelected={false}
        onClick={onClick}
      />
    );

    const link = container.querySelector("a")!;
    fireEvent.click(link);
    expect(onClick).toHaveBeenCalledWith("test-id");
  });

  it("applies selected styles", () => {
    const { container } = render(
      <SearchResultItem
        result={makeResult()}
        query=""
        groupPath=""
        customIcons={{}}
        isSelected={true}
        onClick={vi.fn()}
      />
    );

    const item = container.querySelector("[class*='bg-accent']");
    expect(item).toBeInTheDocument();
  });

  it("renders highlighted text when query matches", () => {
    const { container } = render(
      <SearchResultItem
        result={makeResult()}
        query="git"
        groupPath=""
        customIcons={{}}
        isSelected={false}
        onClick={vi.fn()}
      />
    );

    const marks = container.querySelectorAll("mark");
    expect(marks.length).toBeGreaterThan(0);
    expect(marks[0]!.textContent).toBe("Git");
  });
});
