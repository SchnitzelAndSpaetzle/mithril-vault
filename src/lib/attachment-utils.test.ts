// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import type { AttachmentMeta, Entry } from "@/lib/types";
import { entryHasAttachments } from "./attachment-utils";

function makeAttachment(
  overrides: Partial<AttachmentMeta> = {}
): AttachmentMeta {
  return {
    filename: "file.txt",
    size: 10,
    mimeType: "text/plain",
    ...overrides,
  };
}

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

describe("entryHasAttachments", () => {
  it("returns false when the entry has no attachments", () => {
    expect(entryHasAttachments(makeEntry({ attachments: [] }))).toBe(false);
  });

  it("returns true when the entry has one attachment", () => {
    expect(
      entryHasAttachments(makeEntry({ attachments: [makeAttachment()] }))
    ).toBe(true);
  });

  it("returns true when the entry has multiple attachments", () => {
    expect(
      entryHasAttachments(
        makeEntry({
          attachments: [
            makeAttachment({ filename: "a.txt" }),
            makeAttachment({ filename: "b.txt" }),
          ],
        })
      )
    ).toBe(true);
  });
});
