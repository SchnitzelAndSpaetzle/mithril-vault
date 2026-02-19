// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useEntryDetail } from "../use-entry-detail";
import { useQuery } from "@tanstack/react-query";
import type { Entry } from "@/lib/types";

function makeMockEntry(): Entry {
  return {
    id: "entry-1",
    groupId: "group-1",
    title: "Test Entry",
    username: "user@example.com",
    url: "https://example.com",
    notes: "Some notes",
    iconId: 0,
    customIconUuid: null,
    tags: ["tag1"],
    customFields: {},
    customFieldMeta: [],
    createdAt: "2024-01-01T00:00:00Z",
    modifiedAt: "2024-01-01T00:00:00Z",
    accessedAt: "2024-01-01T00:00:00Z",
  };
}

const mockGet = vi.fn();
const mockGetPassword = vi.fn();

vi.mock("@/lib/tauri", () => ({
  entries: {
    get: (...args: unknown[]) => mockGet(...args),
    getPassword: (...args: unknown[]) => mockGetPassword(...args),
  },
}));

vi.mock("@tanstack/react-query", () => ({
  keepPreviousData: Symbol("keepPreviousData"),
  useQuery: vi.fn(({ enabled }: { enabled: boolean }) => {
    if (!enabled) {
      return {
        data: undefined,
        isLoading: false,
        isError: false,
        isPlaceholderData: false,
      };
    }
    return {
      data: makeMockEntry(),
      isLoading: false,
      isError: false,
      isPlaceholderData: false,
    };
  }),
}));

describe("useEntryDetail", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetPassword.mockResolvedValue("secret-password");
  });

  it("returns null entry when entryId is null", () => {
    const { result } = renderHook(() => useEntryDetail(null, "db-1"));
    expect(result.current.entry).toBeNull();
    expect(result.current.isPasswordVisible).toBe(false);
  });

  it("returns null entry when dbId is null", () => {
    const { result } = renderHook(() => useEntryDetail("entry-1", null));
    expect(result.current.entry).toBeNull();
    expect(result.current.isPasswordVisible).toBe(false);
  });

  it("returns entry data when both ids provided", () => {
    const { result } = renderHook(() => useEntryDetail("entry-1", "db-1"));
    expect(result.current.entry).toEqual(makeMockEntry());
    expect(result.current.isLoading).toBe(false);
    expect(result.current.isTransitioning).toBe(false);
  });

  it("marks transition while detail data still belongs to another entry", () => {
    const { result } = renderHook(() => useEntryDetail("entry-2", "db-1"));
    expect(result.current.isTransitioning).toBe(true);
  });

  it("marks transition while placeholder data is active", () => {
    vi.mocked(useQuery).mockReturnValueOnce({
      data: makeMockEntry(),
      isLoading: false,
      isError: false,
      isPlaceholderData: true,
    } as ReturnType<typeof useQuery>);

    const { result } = renderHook(() => useEntryDetail("entry-1", "db-1"));
    expect(result.current.isTransitioning).toBe(true);
  });

  it("password is hidden by default", () => {
    const { result } = renderHook(() => useEntryDetail("entry-1", "db-1"));
    expect(result.current.password).toBeNull();
    expect(result.current.isPasswordVisible).toBe(false);
  });

  it("reveals and hides password", async () => {
    const { result } = renderHook(() => useEntryDetail("entry-1", "db-1"));

    await act(async () => {
      await result.current.revealPassword();
    });

    await waitFor(() => {
      expect(result.current.password).toBe("secret-password");
      expect(result.current.isPasswordVisible).toBe(true);
    });

    act(() => {
      result.current.hidePassword();
    });

    expect(result.current.password).toBeNull();
    expect(result.current.isPasswordVisible).toBe(false);
  });

  it("clears password when entryId changes", async () => {
    const { result, rerender } = renderHook(
      ({ entryId }) => useEntryDetail(entryId, "db-1"),
      { initialProps: { entryId: "entry-1" } }
    );

    await act(async () => {
      await result.current.revealPassword();
    });

    await waitFor(() => {
      expect(result.current.isPasswordVisible).toBe(true);
    });

    rerender({ entryId: "entry-2" });

    expect(result.current.password).toBeNull();
    expect(result.current.isPasswordVisible).toBe(false);
  });

  it("does not fetch password when ids are missing", async () => {
    const { result } = renderHook(() => useEntryDetail(null, "db-1"));

    await act(async () => {
      await result.current.revealPassword();
    });

    expect(mockGetPassword).not.toHaveBeenCalled();
    expect(result.current.isPasswordLoading).toBe(false);
  });
});
