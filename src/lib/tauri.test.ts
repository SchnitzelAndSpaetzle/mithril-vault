// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { groups, tags } from "./tauri";

describe("tauri wrappers validation", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("validates groups.update payload before invoking", async () => {
    await expect(
      groups.update(crypto.randomUUID(), crypto.randomUUID(), {})
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();

    await expect(
      groups.update(crypto.randomUUID(), crypto.randomUUID(), {
        name: "   ",
      })
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();

    await expect(
      groups.update(crypto.randomUUID(), crypto.randomUUID(), {
        icon: "folder",
      })
    ).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });

  it("trims and validates tag names for rename/delete", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(1) // tags.rename
      .mockResolvedValueOnce(1); // tags.delete

    await tags.rename(crypto.randomUUID(), "  old-tag  ", "  new-tag  ");
    expect(invoke).toHaveBeenNthCalledWith(1, "rename_tag", {
      dbId: expect.any(String),
      oldName: "old-tag",
      newName: "new-tag",
    });

    await tags.delete(crypto.randomUUID(), "  stale-tag  ");
    expect(invoke).toHaveBeenNthCalledWith(2, "delete_tag", {
      dbId: expect.any(String),
      tagName: "stale-tag",
    });
  });

  it("rejects empty tag names before invoking backend", async () => {
    await expect(
      tags.rename(crypto.randomUUID(), " ", "new")
    ).rejects.toThrow();
    await expect(
      tags.rename(crypto.randomUUID(), "old", " ")
    ).rejects.toThrow();
    await expect(tags.delete(crypto.randomUUID(), " ")).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });
});
