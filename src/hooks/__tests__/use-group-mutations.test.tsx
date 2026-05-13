// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";

const {
  mockGroupsCreate,
  mockGroupsDelete,
  mockGroupsMove,
  mockDatabaseSave,
  mockToastError,
} = vi.hoisted(() => ({
  mockGroupsCreate: vi.fn(),
  mockGroupsDelete: vi.fn(),
  mockGroupsMove: vi.fn(),
  mockDatabaseSave: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  groups: {
    create: mockGroupsCreate,
    update: vi.fn(),
    rename: vi.fn(),
    delete: mockGroupsDelete,
    move: mockGroupsMove,
  },
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

describe("useGroupMutations", () => {
  beforeEach(() => {
    mockGroupsCreate.mockReset();
    mockGroupsDelete.mockReset();
    mockGroupsMove.mockReset();
    mockDatabaseSave.mockReset();
    mockToastError.mockReset();
  });

  it("createGroup resolves with the group even when save fails", async () => {
    // The backend created the group in memory. If save throws, the mutation
    // must still resolve so the caller's success path (close dialog, navigate,
    // show success toast) runs against the real backend state. Otherwise the
    // user is stuck and may retry, creating a duplicate.
    mockGroupsCreate.mockResolvedValueOnce({ id: "grp-1", name: "Work" });
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: read-only fs")
    );

    const { useGroupMutations } = await import("../use-group-mutations");
    const { result } = renderHook(() => useGroupMutations("db-1"), { wrapper });

    const group = await result.current.createGroup.mutateAsync({
      dbId: "db-1",
      parentId: "root",
      name: "Work",
    });

    expect(group).toEqual({ id: "grp-1", name: "Work" });
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledTimes(1);
    });
    expect(mockToastError.mock.calls[0]?.[0]).toContain(
      "settings.backups.error.failed"
    );
  });

  it("deleteGroup invalidates group + entry queries on success", async () => {
    mockGroupsDelete.mockResolvedValueOnce(undefined);
    mockDatabaseSave.mockResolvedValueOnce(undefined);

    const { useGroupMutations } = await import("../use-group-mutations");

    const client = new QueryClient({
      defaultOptions: { mutations: { retry: false } },
    });
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const localWrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useGroupMutations("db-1"), {
      wrapper: localWrapper,
    });

    await result.current.deleteGroup.mutateAsync({
      dbId: "db-1",
      id: "grp-1",
    });

    await waitFor(() => {
      // group list, entry counts, recycle-bin id, and entry queries — 4 invalidations.
      expect(invalidateSpy.mock.calls.length).toBeGreaterThanOrEqual(3);
    });
  });

  it("moveGroup rejects when the backend mutation itself fails", async () => {
    // Distinct from a save failure: the in-memory mutation rejected, so there
    // is nothing to keep in the UI. The mutation should reject and onError
    // can surface the failure as a real error toast.
    mockGroupsMove.mockRejectedValueOnce(new Error("Circular reference"));

    const { useGroupMutations } = await import("../use-group-mutations");
    const { result } = renderHook(() => useGroupMutations("db-1"), { wrapper });

    await expect(
      result.current.moveGroup.mutateAsync({
        dbId: "db-1",
        id: "grp-1",
        targetParentId: "grp-1-child",
      })
    ).rejects.toThrow("Circular reference");

    // saveWithErrorToast never ran — the backend mutation rejected first.
    expect(mockDatabaseSave).not.toHaveBeenCalled();
    expect(mockToastError).not.toHaveBeenCalled();
  });
});
