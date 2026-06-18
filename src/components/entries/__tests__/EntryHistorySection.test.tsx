// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import type { EntryHistoryItem } from "@/lib/types";

const listHistoryMock = vi.fn();
const getHistoryPasswordMock = vi.fn();
const getHistoryProtectedFieldMock = vi.fn();
const restoreHistoryMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  entries: {
    listHistory: (...args: unknown[]) => listHistoryMock(...args),
    getHistoryPassword: (...args: unknown[]) => getHistoryPasswordMock(...args),
    getHistoryProtectedField: (...args: unknown[]) =>
      getHistoryProtectedFieldMock(...args),
    restoreHistory: (...args: unknown[]) => restoreHistoryMock(...args),
  },
}));

const askMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: (...args: unknown[]) => askMock(...args),
}));

vi.mock("@/lib/save-with-error-toast", () => ({
  saveWithErrorToast: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

import {
  EntryHistorySection,
  formatChangedFields,
} from "@/components/entries/EntryHistorySection";

const TEST_DB_ID = "test-vault.kdbx";
const TEST_ENTRY_ID = "entry-1";

/**
 * Builds an {@link EntryHistoryItem} with sensible defaults, so each test only
 * spells out the fields it cares about (every version now also carries a
 * `fingerprint` and a `protectedFields` list).
 */
function makeVersion(
  overrides: Partial<EntryHistoryItem> & Pick<EntryHistoryItem, "index">
): EntryHistoryItem {
  return {
    modifiedAt: "2024-02-17T15:58:43Z",
    title: "Example",
    username: "bob",
    url: null,
    changedFields: [],
    isCreation: false,
    fingerprint: `fp-${overrides.index}`,
    protectedFields: [],
    ...overrides,
  };
}

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: Infinity },
    },
  });
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
}

function renderSection() {
  return render(
    <EntryHistorySection dbId={TEST_DB_ID} entryId={TEST_ENTRY_ID} />,
    { wrapper: createWrapper() }
  );
}

async function expand() {
  const trigger = await screen.findByRole("button", {
    name: "entries.detail.history",
  });
  fireEvent.click(trigger);
}

describe("EntryHistorySection", () => {
  beforeEach(() => {
    listHistoryMock.mockReset();
    getHistoryPasswordMock.mockReset();
    getHistoryProtectedFieldMock.mockReset();
    restoreHistoryMock.mockReset();
    askMock.mockReset();
  });

  it("lists each version with its timestamp once expanded", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        modifiedAt: "2024-02-17T15:58:43Z",
        username: "bob",
        changedFields: ["username"],
      }),
      makeVersion({
        index: 1,
        modifiedAt: "2024-01-05T09:30:00Z",
        username: "alice",
        changedFields: ["title"],
        isCreation: true,
      }),
    ]);

    renderSection();
    await expand();

    await waitFor(() => {
      expect(screen.getAllByRole("listitem")).toHaveLength(2);
    });
    expect(screen.getByText(/Feb 17, 2024/)).toBeInTheDocument();
    expect(screen.getByText(/Jan 5, 2024/)).toBeInTheDocument();
  });

  it("renders a changed-fields line listing the field names that changed", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        changedFields: ["password", "username"],
        isCreation: true,
      }),
    ]);

    renderSection();
    await expand();

    await waitFor(() => {
      expect(
        screen.getByText("entries.detail.historyChanged")
      ).toBeInTheDocument();
    });
  });

  it("labels the oldest version 'Created' when it is the original snapshot", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({ index: 0, changedFields: ["username"] }),
      makeVersion({
        index: 1,
        modifiedAt: "2024-01-05T09:30:00Z",
        username: "alice",
        changedFields: ["title"],
        isCreation: true,
      }),
    ]);

    renderSection();
    await expand();

    await waitFor(() => {
      expect(
        screen.getByText("entries.detail.historyCreated")
      ).toBeInTheDocument();
    });
    expect(
      screen.queryByText("entries.detail.historyEarliestKept")
    ).not.toBeInTheDocument();
  });

  it("labels the oldest version 'Earliest kept' but still shows its changed line", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({ index: 0, changedFields: ["username"] }),
      makeVersion({
        index: 1,
        modifiedAt: "2024-01-05T09:30:00Z",
        username: "alice",
        changedFields: ["title"],
      }),
    ]);

    renderSection();
    await expand();

    await waitFor(() => {
      expect(
        screen.getByText("entries.detail.historyEarliestKept")
      ).toBeInTheDocument();
    });
    expect(
      screen.queryByText("entries.detail.historyCreated")
    ).not.toBeInTheDocument();
    expect(screen.getAllByText("entries.detail.historyChanged")).toHaveLength(
      2
    );
  });

  it("localizes known changed-field tokens and leaves custom field names verbatim", () => {
    const labels = {
      password: "Passwort",
      location: "Speicherort",
    };
    expect(
      formatChangedFields(["password", "location", "MyCustomField"], labels)
    ).toBe("Passwort, Speicherort, MyCustomField");
  });

  it("shows an empty state when the entry has no history", async () => {
    listHistoryMock.mockResolvedValue([]);

    renderSection();
    await expand();

    await waitFor(() => {
      expect(screen.getByText("entries.detail.noHistory")).toBeInTheDocument();
    });
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });

  it("reveals a version's password on demand, guarded by its fingerprint", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        fingerprint: "fp-zero",
        changedFields: ["password"],
      }),
    ]);
    getHistoryPasswordMock.mockResolvedValue("orig-pw");

    renderSection();
    await expand();

    // The password is masked until the explicit reveal action.
    await waitFor(() => {
      expect(screen.getByText("••••••••")).toBeInTheDocument();
    });
    expect(screen.queryByText("orig-pw")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "entries.detail.revealPassword" })
    );

    await waitFor(() => {
      expect(screen.getByText("orig-pw")).toBeInTheDocument();
    });
    // Addressed by index + the version's fingerprint (the stale-edit guard).
    expect(getHistoryPasswordMock).toHaveBeenCalledWith(
      TEST_DB_ID,
      TEST_ENTRY_ID,
      0,
      "fp-zero"
    );
  });

  it("reveals a version's protected custom field on demand", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        fingerprint: "fp-zero",
        protectedFields: ["PIN"],
      }),
    ]);
    getHistoryProtectedFieldMock.mockResolvedValue({
      key: "PIN",
      value: "0451",
    });

    renderSection();
    await expand();

    expect(screen.queryByText("0451")).not.toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "entries.detail.revealField" })
    );

    await waitFor(() => {
      expect(screen.getByText("0451")).toBeInTheDocument();
    });
    expect(getHistoryProtectedFieldMock).toHaveBeenCalledWith(
      TEST_DB_ID,
      TEST_ENTRY_ID,
      0,
      "fp-zero",
      "PIN"
    );
  });

  it("restores a version after the user confirms, addressed by index + fingerprint", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        fingerprint: "fp-zero",
        changedFields: ["password"],
      }),
    ]);
    askMock.mockResolvedValue(true);
    restoreHistoryMock.mockResolvedValue({ id: TEST_ENTRY_ID });

    renderSection();
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.restoreVersion",
      })
    );

    // The destructive action is gated on an explicit confirmation.
    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(restoreHistoryMock).toHaveBeenCalledWith(
        TEST_DB_ID,
        TEST_ENTRY_ID,
        0,
        "fp-zero"
      );
    });
  });

  it("does not restore when the user cancels the confirmation", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        fingerprint: "fp-zero",
        changedFields: ["password"],
      }),
    ]);
    askMock.mockResolvedValue(false);

    renderSection();
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.restoreVersion",
      })
    );

    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    expect(restoreHistoryMock).not.toHaveBeenCalled();
  });
});
