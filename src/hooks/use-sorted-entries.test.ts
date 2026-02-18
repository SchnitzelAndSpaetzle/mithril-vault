// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import { sortEntries } from "./use-sorted-entries";
import type { Entry } from "@/lib/types";

function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: crypto.randomUUID(),
    groupId: "group-1",
    title: "",
    username: "",
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

describe("sortEntries", () => {
  it("returns empty array for empty input", () => {
    expect(sortEntries([], "title", "asc")).toEqual([]);
  });

  it("sorts by title ascending", () => {
    const entries = [
      makeEntry({ title: "Charlie" }),
      makeEntry({ title: "Alice" }),
      makeEntry({ title: "Bob" }),
    ];
    const sorted = sortEntries(entries, "title", "asc");
    expect(sorted.map((e) => e.title)).toEqual(["Alice", "Bob", "Charlie"]);
  });

  it("sorts by title descending", () => {
    const entries = [
      makeEntry({ title: "Alice" }),
      makeEntry({ title: "Charlie" }),
      makeEntry({ title: "Bob" }),
    ];
    const sorted = sortEntries(entries, "title", "desc");
    expect(sorted.map((e) => e.title)).toEqual(["Charlie", "Bob", "Alice"]);
  });

  it("sorts by title case-insensitively", () => {
    const entries = [
      makeEntry({ title: "banana" }),
      makeEntry({ title: "Apple" }),
      makeEntry({ title: "cherry" }),
    ];
    const sorted = sortEntries(entries, "title", "asc");
    expect(sorted.map((e) => e.title)).toEqual(["Apple", "banana", "cherry"]);
  });

  it("sorts by username ascending", () => {
    const entries = [
      makeEntry({ username: "zoe" }),
      makeEntry({ username: "alice" }),
      makeEntry({ username: "mike" }),
    ];
    const sorted = sortEntries(entries, "username", "asc");
    expect(sorted.map((e) => e.username)).toEqual(["alice", "mike", "zoe"]);
  });

  it("sorts by url ascending with null values", () => {
    const entries = [
      makeEntry({ url: "https://z.com" }),
      makeEntry({ url: null }),
      makeEntry({ url: "https://a.com" }),
    ];
    const sorted = sortEntries(entries, "url", "asc");
    expect(sorted.map((e) => e.url)).toEqual([
      null,
      "https://a.com",
      "https://z.com",
    ]);
  });

  it("sorts by modifiedAt ascending", () => {
    const entries = [
      makeEntry({ modifiedAt: "2024-03-01T00:00:00Z" }),
      makeEntry({ modifiedAt: "2024-01-01T00:00:00Z" }),
      makeEntry({ modifiedAt: "2024-02-01T00:00:00Z" }),
    ];
    const sorted = sortEntries(entries, "modifiedAt", "asc");
    expect(sorted.map((e) => e.modifiedAt)).toEqual([
      "2024-01-01T00:00:00Z",
      "2024-02-01T00:00:00Z",
      "2024-03-01T00:00:00Z",
    ]);
  });

  it("sorts by modifiedAt descending (newest first)", () => {
    const entries = [
      makeEntry({ modifiedAt: "2024-01-01T00:00:00Z" }),
      makeEntry({ modifiedAt: "2024-03-01T00:00:00Z" }),
      makeEntry({ modifiedAt: "2024-02-01T00:00:00Z" }),
    ];
    const sorted = sortEntries(entries, "modifiedAt", "desc");
    expect(sorted.map((e) => e.modifiedAt)).toEqual([
      "2024-03-01T00:00:00Z",
      "2024-02-01T00:00:00Z",
      "2024-01-01T00:00:00Z",
    ]);
  });

  it("sorts by createdAt ascending", () => {
    const entries = [
      makeEntry({ createdAt: "2024-06-01T00:00:00Z" }),
      makeEntry({ createdAt: "2024-01-01T00:00:00Z" }),
      makeEntry({ createdAt: "2024-03-01T00:00:00Z" }),
    ];
    const sorted = sortEntries(entries, "createdAt", "asc");
    expect(sorted.map((e) => e.createdAt)).toEqual([
      "2024-01-01T00:00:00Z",
      "2024-03-01T00:00:00Z",
      "2024-06-01T00:00:00Z",
    ]);
  });

  it("does not mutate original array", () => {
    const entries = [makeEntry({ title: "B" }), makeEntry({ title: "A" })];
    const original = [...entries];
    sortEntries(entries, "title", "asc");
    expect(entries.map((e) => e.title)).toEqual(original.map((e) => e.title));
  });

  it("handles single entry", () => {
    const entries = [makeEntry({ title: "Only" })];
    const sorted = sortEntries(entries, "title", "asc");
    expect(sorted).toHaveLength(1);
    expect(sorted[0].title).toBe("Only");
  });
});
