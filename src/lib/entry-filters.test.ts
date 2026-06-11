// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import type { Entry } from "@/lib/types";
import { filterEntries } from "./entry-filters";

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

const withAttachment = { filename: "f", size: 1, mimeType: "text/plain" };

describe("filterEntries", () => {
  it("returns all entries when no filters are set", () => {
    const entries = [makeEntry(), makeEntry()];
    expect(filterEntries(entries, {})).toEqual(entries);
  });

  it("filters by tag only", () => {
    const dev = makeEntry({ tags: ["dev"] });
    const work = makeEntry({ tags: ["work"] });
    expect(filterEntries([dev, work], { tag: "dev" })).toEqual([dev]);
  });

  it("filters by has-attachments only", () => {
    const withFile = makeEntry({ attachments: [withAttachment] });
    const without = makeEntry({ attachments: [] });
    expect(
      filterEntries([withFile, without], { hasAttachments: true })
    ).toEqual([withFile]);
  });

  it("AND-combines tag and has-attachments (filters stack)", () => {
    const match = makeEntry({ tags: ["dev"], attachments: [withAttachment] });
    const tagOnly = makeEntry({ tags: ["dev"], attachments: [] });
    const attachmentOnly = makeEntry({
      tags: ["work"],
      attachments: [withAttachment],
    });
    expect(
      filterEntries([match, tagOnly, attachmentOnly], {
        tag: "dev",
        hasAttachments: true,
      })
    ).toEqual([match]);
  });

  it("ignores a falsy hasAttachments flag", () => {
    const withFile = makeEntry({ attachments: [withAttachment] });
    const without = makeEntry({ attachments: [] });
    expect(
      filterEntries([withFile, without], { hasAttachments: false })
    ).toEqual([withFile, without]);
  });
});
