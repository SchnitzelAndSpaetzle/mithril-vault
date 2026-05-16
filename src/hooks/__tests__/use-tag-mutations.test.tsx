// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";

const { mockTagsRename, mockTagsDelete, mockDatabaseSave, mockToastError } =
  vi.hoisted(() => ({
    mockTagsRename: vi.fn(),
    mockTagsDelete: vi.fn(),
    mockDatabaseSave: vi.fn(),
    mockToastError: vi.fn(),
  }));

vi.mock("@/lib/tauri", () => ({
  tags: { rename: mockTagsRename, delete: mockTagsDelete },
  database: { save: mockDatabaseSave },
}));

vi.mock("sonner", () => ({
  toast: { error: mockToastError, success: vi.fn() },
}));

function wrapper({ children }: { children: React.ReactNode }) {
  const client = new QueryClient({
    defaultOptions: { mutations: { retry: false } },
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}

describe("useTagMutations", () => {
  beforeEach(() => {
    mockTagsRename.mockReset();
    mockTagsDelete.mockReset();
    mockDatabaseSave.mockReset();
    mockToastError.mockReset();
  });

  it("renameTag resolves with affected count even when save fails", async () => {
    // The tag rename mutated the in-memory entries. The hook must resolve
    // with the count so the caller closes the rename dialog and navigates
    // to the renamed tag's URL — otherwise the dialog stays open and the
    // user can re-submit, doubling the rename.
    mockTagsRename.mockResolvedValueOnce(7);
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: permission denied")
    );

    const { useTagMutations } = await import("../use-tag-mutations");
    const { result } = renderHook(() => useTagMutations("db-1"), { wrapper });

    const affected = await result.current.renameTag.mutateAsync({
      dbId: "db-1",
      oldName: "work",
      newName: "office",
    });

    expect(affected).toBe(7);
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledTimes(1);
    });
    expect(mockToastError.mock.calls[0]?.[0]).toContain(
      "settings.backups.error.failed"
    );
  });

  it("deleteTag invalidates entry queries on success", async () => {
    mockTagsDelete.mockResolvedValueOnce(3);
    mockDatabaseSave.mockResolvedValueOnce(undefined);

    const { useTagMutations } = await import("../use-tag-mutations");

    const client = new QueryClient({
      defaultOptions: { mutations: { retry: false } },
    });
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const localWrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useTagMutations("db-1"), {
      wrapper: localWrapper,
    });

    const affected = await result.current.deleteTag.mutateAsync({
      dbId: "db-1",
      tagName: "stale",
    });

    expect(affected).toBe(3);
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalled();
    });
  });
});
