// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";
import { useEntryListHeader } from "../use-entry-list-header";
import type { Entry } from "@/lib/types";

const mocks = vi.hoisted(() => ({
  entries: [] as Entry[],
  groups: [] as unknown[],
  search: {},
}));

vi.mock("@/hooks/use-active-database", () => ({
  useActiveDatabase: () => ({ dbId: "db-1" }),
}));

vi.mock("@/hooks/use-entries", () => ({
  useEntries: () => ({ data: mocks.entries }),
}));

vi.mock("@/hooks/use-groups", () => ({
  useGroups: () => ({ data: mocks.groups }),
}));

vi.mock("@tanstack/react-router", () => ({
  useSearch: () => mocks.search,
}));

function makeEntry(overrides: Partial<Entry> = {}): Entry {
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

const withFile = { filename: "f", size: 1, mimeType: "text/plain" };

beforeEach(() => {
  mocks.entries = [];
  mocks.groups = [];
  mocks.search = {};
});

describe("useEntryListHeader entryCount", () => {
  it("counts all entries when no filter is active", () => {
    mocks.entries = [makeEntry(), makeEntry()];
    const { result } = renderHook(() => useEntryListHeader());
    expect(result.current.entryCount).toBe(2);
  });

  it("counts only entries with attachments when hasAttachments is set", () => {
    mocks.entries = [
      makeEntry({ attachments: [withFile] }),
      makeEntry({ attachments: [] }),
    ];
    mocks.search = { hasAttachments: true };
    const { result } = renderHook(() => useEntryListHeader());
    expect(result.current.entryCount).toBe(1);
  });

  it("stacks the tag and has-attachments filters in the count", () => {
    mocks.entries = [
      makeEntry({ tags: ["dev"], attachments: [withFile] }),
      makeEntry({ tags: ["dev"], attachments: [] }),
      makeEntry({ tags: ["work"], attachments: [withFile] }),
    ];
    mocks.search = { tag: "dev", hasAttachments: true };
    const { result } = renderHook(() => useEntryListHeader());
    expect(result.current.entryCount).toBe(1);
  });
});
