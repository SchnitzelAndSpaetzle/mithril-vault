// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import React from "react";

const { mockEntriesCreate, mockDatabaseSave, mockToastError } = vi.hoisted(
  () => ({
    mockEntriesCreate: vi.fn(),
    mockDatabaseSave: vi.fn(),
    mockToastError: vi.fn(),
  })
);

vi.mock("@/lib/tauri", () => ({
  entries: {
    create: mockEntriesCreate,
    update: vi.fn(),
    move: vi.fn(),
    delete: vi.fn(),
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
    mockDatabaseSave.mockReset();
    mockToastError.mockReset();
  });

  it("rejects with SaveError and toasts backup failure when save fails after create", async () => {
    mockEntriesCreate.mockResolvedValueOnce({ id: "entry-1" });
    mockDatabaseSave.mockRejectedValueOnce(
      new Error("Backup failed for /v/db.kdbx: disk full")
    );

    const { useEntryMutations } = await import("../use-entry-mutations");
    const { SaveError } = await import("@/lib/save-with-error-toast");

    const { result } = renderHook(() => useEntryMutations("db-1"), {
      wrapper,
    });

    await expect(
      result.current.createEntry.mutateAsync({
        dbId: "db-1",
        groupId: "g-1",
        // minimal payload — mutationFn is mocked, shape doesn't matter
        data: {} as never,
      })
    ).rejects.toBeInstanceOf(SaveError);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledTimes(1);
    });
    expect(mockToastError.mock.calls[0]?.[0]).toContain(
      "settings.backups.error.failed"
    );
    expect(mockEntriesCreate).toHaveBeenCalledTimes(1);
    expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
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
