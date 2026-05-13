// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";

const {
  mockEntriesCreate,
  mockEntriesUpdate,
  mockEntriesMove,
  mockEntriesDelete,
  mockDatabaseSave,
  mockToastError,
} = vi.hoisted(() => ({
  mockEntriesCreate: vi.fn(),
  mockEntriesUpdate: vi.fn(),
  mockEntriesMove: vi.fn(),
  mockEntriesDelete: vi.fn(),
  mockDatabaseSave: vi.fn(),
  mockToastError: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  entries: {
    create: mockEntriesCreate,
    update: mockEntriesUpdate,
    move: mockEntriesMove,
    delete: mockEntriesDelete,
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

describe("useEntryMutations.createEntry", () => {
  beforeEach(() => {
    mockEntriesCreate.mockReset();
    mockEntriesUpdate.mockReset();
    mockEntriesMove.mockReset();
    mockEntriesDelete.mockReset();
    mockDatabaseSave.mockReset();
    mockToastError.mockReset();
  });

  it("resolves with the entry even when save fails, and surfaces the backup toast", async () => {
    // The backend mutation succeeded in memory; only the post-mutation save
    // step failed (backup directory unwritable). The hook must still resolve
    // with the entity so the caller's success path (close form, navigate)
    // runs — staying in the form would let the user retry and create a
    // duplicate. The error toast is the user's signal to act.
    mockEntriesCreate.mockResolvedValueOnce({ id: "entry-1" });
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: disk full")
    );

    const { useEntryMutations } = await import("../use-entry-mutations");

    const { result } = renderHook(() => useEntryMutations("db-1"), {
      wrapper,
    });

    const entry = await result.current.createEntry.mutateAsync({
      dbId: "db-1",
      groupId: "g-1",
      data: {} as never,
    });

    expect(entry).toEqual({ id: "entry-1" });

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledTimes(1);
    });
    expect(mockToastError.mock.calls[0]?.[0]).toContain(
      "settings.backups.error.failed"
    );
    expect(mockEntriesCreate).toHaveBeenCalledTimes(1);
    expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
  });

  it("invalidates entry queries on the in-memory success path", async () => {
    mockEntriesCreate.mockResolvedValueOnce({ id: "entry-1" });
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: disk full")
    );

    const { useEntryMutations } = await import("../use-entry-mutations");

    const client = new QueryClient({
      defaultOptions: { mutations: { retry: false } },
    });
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const localWrapper = ({ children }: { children: React.ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );

    const { result } = renderHook(() => useEntryMutations("db-1"), {
      wrapper: localWrapper,
    });

    await result.current.createEntry.mutateAsync({
      dbId: "db-1",
      groupId: "g-1",
      data: {} as never,
    });

    // saveWithErrorToast does not reject; mutation resolves; onSuccess fires;
    // cache is invalidated so the UI reflects the in-memory new entry.
    await waitFor(() => {
      expect(invalidateSpy).toHaveBeenCalled();
    });
  });

  it("updateEntry resolves with the updated entry even when save fails", async () => {
    mockEntriesUpdate.mockResolvedValueOnce({
      id: "entry-1",
      title: "Renamed",
    });
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: disk full")
    );

    const { useEntryMutations } = await import("../use-entry-mutations");
    const { result } = renderHook(() => useEntryMutations("db-1"), { wrapper });

    const entry = await result.current.updateEntry.mutateAsync({
      dbId: "db-1",
      id: "entry-1",
      data: { title: "Renamed" } as never,
    });

    expect(entry).toEqual({ id: "entry-1", title: "Renamed" });
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledTimes(1);
    });
  });

  it("moveEntry resolves with the moved entry even when save fails", async () => {
    mockEntriesMove.mockResolvedValueOnce({ id: "entry-1", groupId: "g-2" });
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: disk full")
    );

    const { useEntryMutations } = await import("../use-entry-mutations");
    const { result } = renderHook(() => useEntryMutations("db-1"), { wrapper });

    const entry = await result.current.moveEntry.mutateAsync({
      dbId: "db-1",
      id: "entry-1",
      targetGroupId: "g-2",
    });

    expect(entry).toEqual({ id: "entry-1", groupId: "g-2" });
    expect(mockEntriesMove).toHaveBeenCalledWith("db-1", "entry-1", "g-2");
  });

  it("deleteEntry resolves even when save fails so the dialog can close", async () => {
    // The delete already removed the entry from the in-memory database. The
    // mutation must resolve so the caller closes the confirmation dialog and
    // clears the selection — otherwise the user sees a stale entry that no
    // longer exists in memory and may try to act on it.
    mockEntriesDelete.mockResolvedValueOnce(undefined);
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: read-only fs")
    );

    const { useEntryMutations } = await import("../use-entry-mutations");
    const { result } = renderHook(() => useEntryMutations("db-1"), {
      wrapper,
    });

    await expect(
      result.current.deleteEntry.mutateAsync({
        dbId: "db-1",
        id: "entry-1",
      })
    ).resolves.toBeUndefined();

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledTimes(1);
    });
    expect(mockEntriesDelete).toHaveBeenCalledWith("db-1", "entry-1");
  });

  it("succeeds quietly (no error toast) when create + save both succeed", async () => {
    mockEntriesCreate.mockResolvedValueOnce({ id: "entry-1" });
    mockDatabaseSave.mockResolvedValueOnce(undefined);

    const { useEntryMutations } = await import("../use-entry-mutations");
    const { result } = renderHook(() => useEntryMutations("db-1"), {
      wrapper,
    });

    const entry = await result.current.createEntry.mutateAsync({
      dbId: "db-1",
      groupId: "g-1",
      data: {} as never,
    });

    expect(entry).toEqual({ id: "entry-1" });
    expect(mockToastError).not.toHaveBeenCalled();
    expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
  });
});
