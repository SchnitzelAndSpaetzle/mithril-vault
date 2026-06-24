// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { useVaultHistorySettings } from "../use-vault-history-settings";
import { queryKeys } from "@/lib/query-keys";

const getHistorySettingsMock = vi.fn();
const updateHistorySettingsMock = vi.fn();
const clearAllHistoryMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  database: {
    getHistorySettings: (...args: unknown[]) => getHistorySettingsMock(...args),
    updateHistorySettings: (...args: unknown[]) =>
      updateHistorySettingsMock(...args),
    clearAllHistory: (...args: unknown[]) => clearAllHistoryMock(...args),
  },
}));

const saveWithErrorToastMock = vi.fn();
vi.mock("@/lib/save-with-error-toast", () => ({
  saveWithErrorToast: (...args: unknown[]) => saveWithErrorToastMock(...args),
}));

const toastErrorMock = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const TEST_DB_ID = "/vault.kdbx";

function renderUseVaultHistorySettings() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: Infinity } },
  });
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  const view = renderHook(() => useVaultHistorySettings(TEST_DB_ID), {
    wrapper,
  });
  return { ...view, queryClient };
}

describe("useVaultHistorySettings — clearAll", () => {
  beforeEach(() => {
    getHistorySettingsMock.mockReset().mockResolvedValue({ maxItems: null });
    updateHistorySettingsMock.mockReset();
    clearAllHistoryMock.mockReset();
    saveWithErrorToastMock.mockReset().mockResolvedValue(true);
    toastErrorMock.mockReset();
  });

  it("clears all history, persists, and invalidates this vault's entry queries", async () => {
    clearAllHistoryMock.mockResolvedValue(undefined);
    const { result, queryClient } = renderUseVaultHistorySettings();

    // Seed an open entry-history view for this vault, plus one for a different
    // vault, so we can prove the refresh is scoped to the cleared vault only.
    const thisVaultKey = queryKeys.entries.history(TEST_DB_ID, "entry-1");
    const otherVaultKey = queryKeys.entries.history("/other.kdbx", "entry-9");
    queryClient.setQueryData(thisVaultKey, []);
    queryClient.setQueryData(otherVaultKey, []);

    act(() => {
      result.current.clearAll();
    });

    await waitFor(() => {
      expect(clearAllHistoryMock).toHaveBeenCalledWith(TEST_DB_ID);
    });
    // The work lands on disk via the shared save helper...
    expect(saveWithErrorToastMock).toHaveBeenCalledWith(
      TEST_DB_ID,
      expect.anything()
    );
    // ...and the open entry-history view for this vault is refreshed, while a
    // different vault's cached history is left untouched.
    await waitFor(() => {
      expect(queryClient.getQueryState(thisVaultKey)?.isInvalidated).toBe(true);
    });
    expect(queryClient.getQueryState(otherVaultKey)?.isInvalidated).toBe(false);
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("surfaces a toast when the clear itself fails", async () => {
    clearAllHistoryMock.mockRejectedValue(new Error("DatabaseLocked"));
    const { result } = renderUseVaultHistorySettings();

    act(() => {
      result.current.clearAll();
    });

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalledWith(
        "settings.database.history.clearAllFailed"
      );
    });
  });
});
