// SPDX-License-Identifier: MIT

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useSearchEntries } from "../use-search-entries";
import type { Entry } from "@/lib/types";

function makeMockEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: crypto.randomUUID(),
    groupId: "group-1",
    title: "Title",
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
    expires: false,
    attachments: [],
    ...overrides,
  };
}

const mockEntries = [
  makeMockEntry({
    title: "GitHub",
    username: "admin",
    tags: ["dev"],
    attachments: [{ filename: "key.pem", size: 10, mimeType: "text/plain" }],
  }),
  makeMockEntry({ title: "Gmail", username: "user@gmail.com", tags: ["work"] }),
];

vi.mock("@/hooks/use-entries", () => ({
  useEntries: vi.fn(() => ({
    data: mockEntries,
    isLoading: false,
    isError: false,
    error: null,
  })),
}));

describe("useSearchEntries", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("starts with empty query and inactive search", () => {
    const { result } = renderHook(() => useSearchEntries("db-1"));
    expect(result.current.query).toBe("");
    expect(result.current.isSearchActive).toBe(false);
    expect(result.current.results).toEqual([]);
  });

  it("returns results after debounce when query is set", () => {
    const { result } = renderHook(() => useSearchEntries("db-1"));

    act(() => {
      result.current.setQuery("git");
    });

    expect(result.current.isSearchActive).toBe(true);
    // Results not yet updated (debounce)
    expect(result.current.results).toEqual([]);

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(result.current.results).toHaveLength(1);
    expect(result.current.results[0]!.entry.title).toBe("GitHub");
  });

  it("clearSearch resets query and deactivates search", () => {
    const { result } = renderHook(() => useSearchEntries("db-1"));

    act(() => {
      result.current.setQuery("git");
      vi.advanceTimersByTime(200);
    });

    expect(result.current.isSearchActive).toBe(true);

    act(() => {
      result.current.clearSearch();
    });

    expect(result.current.query).toBe("");
    expect(result.current.isSearchActive).toBe(false);
  });

  it("filters entries by tag when tag is provided", () => {
    const { result } = renderHook(() => useSearchEntries("db-1", null, "dev"));

    act(() => {
      result.current.setQuery("g");
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    // Only GitHub has tag "dev", Gmail has "work"
    expect(result.current.results).toHaveLength(1);
    expect(result.current.results[0]!.entry.title).toBe("GitHub");
  });

  it("filters entries by attachments when hasAttachments is true", () => {
    const { result } = renderHook(() =>
      useSearchEntries("db-1", null, null, true)
    );

    act(() => {
      result.current.setQuery("g");
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    // Both GitHub and Gmail match "g", but only GitHub has an attachment.
    expect(result.current.results).toHaveLength(1);
    expect(result.current.results[0]!.entry.title).toBe("GitHub");
  });

  it("does not filter by attachments when hasAttachments is false", () => {
    const { result } = renderHook(() =>
      useSearchEntries("db-1", null, null, false)
    );

    act(() => {
      result.current.setQuery("g");
    });

    act(() => {
      vi.advanceTimersByTime(200);
    });

    expect(result.current.results).toHaveLength(2);
  });

  it("clears query when hasAttachments changes", () => {
    let hasAttachments = false;
    const { result, rerender } = renderHook(() =>
      useSearchEntries("db-1", null, null, hasAttachments)
    );

    act(() => {
      result.current.setQuery("g");
      vi.advanceTimersByTime(200);
    });

    expect(result.current.isSearchActive).toBe(true);

    hasAttachments = true;
    rerender();

    expect(result.current.query).toBe("");
    expect(result.current.isSearchActive).toBe(false);
  });

  it("clears query when groupId changes", () => {
    let groupId: string | null = "group-a";
    const { result, rerender } = renderHook(() =>
      useSearchEntries("db-1", groupId)
    );

    act(() => {
      result.current.setQuery("git");
      vi.advanceTimersByTime(200);
    });

    expect(result.current.isSearchActive).toBe(true);

    groupId = "group-b";
    rerender();

    expect(result.current.query).toBe("");
    expect(result.current.isSearchActive).toBe(false);
  });

  it("clears query when tag changes", () => {
    let tag: string | null = "dev";
    const { result, rerender } = renderHook(() =>
      useSearchEntries("db-1", null, tag)
    );

    act(() => {
      result.current.setQuery("git");
      vi.advanceTimersByTime(200);
    });

    expect(result.current.isSearchActive).toBe(true);

    tag = "work";
    rerender();

    expect(result.current.query).toBe("");
    expect(result.current.isSearchActive).toBe(false);
  });
});
