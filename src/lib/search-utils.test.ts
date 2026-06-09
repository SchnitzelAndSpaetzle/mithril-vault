// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import type { Entry, Group } from "@/lib/types";
import {
  buildGroupPathMap,
  highlightMatches,
  searchEntries,
} from "./search-utils";

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

function makeGroup(id: string, name: string, children: Group[] = []): Group {
  return {
    id,
    parentId: null,
    name,
    icon: null,
    customIconUuid: null,
    children,
  };
}

describe("searchEntries", () => {
  it("returns empty array for empty query", () => {
    const entries = [makeEntry({ title: "test" })];
    expect(searchEntries(entries, "")).toEqual([]);
  });

  it("returns empty array for whitespace-only query", () => {
    const entries = [makeEntry({ title: "test" })];
    expect(searchEntries(entries, "   ")).toEqual([]);
  });

  it("matches title case-insensitively", () => {
    const entries = [makeEntry({ title: "GitHub Login" })];
    const results = searchEntries(entries, "github");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("title");
  });

  it("matches username", () => {
    const entries = [makeEntry({ username: "admin@example.com" })];
    const results = searchEntries(entries, "admin");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("username");
  });

  it("matches url", () => {
    const entries = [makeEntry({ url: "https://github.com" })];
    const results = searchEntries(entries, "github");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("url");
  });

  it("matches notes", () => {
    const entries = [makeEntry({ notes: "Remember to update password" })];
    const results = searchEntries(entries, "update");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("notes");
  });

  it("matches tags", () => {
    const entries = [makeEntry({ tags: ["work", "important"] })];
    const results = searchEntries(entries, "important");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("tags");
  });

  it("matches tags from custom fields", () => {
    const entries = [
      makeEntry({ tags: [], customFields: { Tags: "work;important" } }),
    ];
    const results = searchEntries(entries, "important");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("tags");
  });

  it("does not match entries without query in any field", () => {
    const entries = [
      makeEntry({ title: "GitHub", username: "admin", url: "https://gh.com" }),
    ];
    const results = searchEntries(entries, "nonexistent");
    expect(results).toHaveLength(0);
  });

  it("reports multiple matched fields", () => {
    const entries = [
      makeEntry({ title: "myapp", username: "myapp-admin", url: null }),
    ];
    const results = searchEntries(entries, "myapp");
    expect(results).toHaveLength(1);
    expect(results[0]!.matchedFields).toContain("title");
    expect(results[0]!.matchedFields).toContain("username");
  });

  it("sorts by relevance: title matches first", () => {
    const entryA = makeEntry({ title: "Other", username: "github-user" });
    const entryB = makeEntry({ title: "GitHub", username: "other" });
    const results = searchEntries([entryA, entryB], "github");
    expect(results[0]!.entry.title).toBe("GitHub");
  });

  it("sorts alphabetically when relevance is equal", () => {
    const entryA = makeEntry({ title: "Banana" });
    const entryB = makeEntry({ title: "Apple" });
    const results = searchEntries([entryA, entryB], "a");
    // Both match title, so sorted alphabetically
    expect(results[0]!.entry.title).toBe("Apple");
    expect(results[1]!.entry.title).toBe("Banana");
  });

  it("handles null url and notes gracefully", () => {
    const entries = [makeEntry({ url: null, notes: null })];
    const results = searchEntries(entries, "test");
    expect(results).toHaveLength(0);
  });
});

describe("buildGroupPathMap", () => {
  it("returns empty map for empty groups", () => {
    const map = buildGroupPathMap([]);
    expect(map.size).toBe(0);
  });

  it("builds path for flat groups", () => {
    const groups = [makeGroup("1", "Root"), makeGroup("2", "Other")];
    const map = buildGroupPathMap(groups);
    expect(map.get("1")).toBe("Root");
    expect(map.get("2")).toBe("Other");
  });

  it("builds nested paths", () => {
    const groups = [
      makeGroup("1", "Root", [makeGroup("2", "Sub", [makeGroup("3", "Leaf")])]),
    ];
    const map = buildGroupPathMap(groups);
    expect(map.get("1")).toBe("Root");
    expect(map.get("2")).toBe("Root > Sub");
    expect(map.get("3")).toBe("Root > Sub > Leaf");
  });
});

describe("highlightMatches", () => {
  it("returns single segment for empty query", () => {
    expect(highlightMatches("hello", "")).toEqual([
      { text: "hello", highlighted: false },
    ]);
  });

  it("returns single segment for empty text", () => {
    expect(highlightMatches("", "test")).toEqual([
      { text: "", highlighted: false },
    ]);
  });

  it("highlights matching portion", () => {
    expect(highlightMatches("Hello World", "world")).toEqual([
      { text: "Hello ", highlighted: false },
      { text: "World", highlighted: true },
    ]);
  });

  it("highlights case-insensitively preserving original case", () => {
    expect(highlightMatches("GitHub Login", "git")).toEqual([
      { text: "Git", highlighted: true },
      { text: "Hub Login", highlighted: false },
    ]);
  });

  it("highlights multiple occurrences", () => {
    expect(highlightMatches("foo bar foo", "foo")).toEqual([
      { text: "foo", highlighted: true },
      { text: " bar ", highlighted: false },
      { text: "foo", highlighted: true },
    ]);
  });

  it("returns full text highlighted when entire text matches", () => {
    expect(highlightMatches("test", "test")).toEqual([
      { text: "test", highlighted: true },
    ]);
  });

  it("treats special characters in query as literal text", () => {
    expect(
      highlightMatches("prefix .*+?^${}()|[]\\ suffix", ".*+?^${}()|[]\\")
    ).toEqual([
      { text: "prefix ", highlighted: false },
      { text: ".*+?^${}()|[]\\", highlighted: true },
      { text: " suffix", highlighted: false },
    ]);
  });

  it("uses non-overlapping matching for repeated patterns", () => {
    expect(highlightMatches("banana", "ana")).toEqual([
      { text: "b", highlighted: false },
      { text: "ana", highlighted: true },
      { text: "na", highlighted: false },
    ]);
  });
});
