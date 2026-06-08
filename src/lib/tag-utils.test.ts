// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import type { Entry } from "@/lib/types";
import {
  collectUniqueTags,
  entryHasTag,
  getNormalizedEntryTags,
} from "./tag-utils";

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
    ...overrides,
  };
}

describe("tag-utils", () => {
  it("normalizes tags from entry tags and custom fields", () => {
    const entry = makeEntry({
      tags: ["work; dev", "personal"],
      customFields: { Tags: "travel, finance" },
    });

    expect(getNormalizedEntryTags(entry)).toEqual([
      "work",
      "dev",
      "personal",
      "travel",
      "finance",
    ]);
  });

  it("matches tags from delimited values and lowercase custom field key", () => {
    const entry = makeEntry({
      tags: ["work,ops"],
      customFields: { tags: "infra;security" },
    });

    expect(entryHasTag(entry, "ops")).toBe(true);
    expect(entryHasTag(entry, "security")).toBe(true);
    expect(entryHasTag(entry, "missing")).toBe(false);
  });

  it("collects unique sorted tags and ignores blank fragments", () => {
    const entries = [
      makeEntry({
        tags: ["Beta", "alpha; ", "alpha"],
        customFields: { Tags: "  gamma,  " },
      }),
      makeEntry({
        tags: ["delta"],
        customFields: { tags: "beta; epsilon" },
      }),
    ];

    expect(collectUniqueTags(entries)).toEqual([
      "alpha",
      "Beta",
      "beta",
      "delta",
      "epsilon",
      "gamma",
    ]);
  });
});
