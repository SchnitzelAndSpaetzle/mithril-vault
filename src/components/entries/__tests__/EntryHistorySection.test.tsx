// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import type { EntryHistoryItem } from "@/lib/types";
import { queryKeys } from "@/lib/query-keys";

const listHistoryMock = vi.fn();
const getHistoryPasswordMock = vi.fn();
const getHistoryProtectedFieldMock = vi.fn();
const restoreHistoryMock = vi.fn();
const clearHistoryMock = vi.fn();
const getMock = vi.fn();
const getPasswordMock = vi.fn();
const getProtectedCustomFieldMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  entries: {
    listHistory: (...args: unknown[]) => listHistoryMock(...args),
    getHistoryPassword: (...args: unknown[]) => getHistoryPasswordMock(...args),
    getHistoryProtectedField: (...args: unknown[]) =>
      getHistoryProtectedFieldMock(...args),
    restoreHistory: (...args: unknown[]) => restoreHistoryMock(...args),
    clearHistory: (...args: unknown[]) => clearHistoryMock(...args),
    get: (...args: unknown[]) => getMock(...args),
    getPassword: (...args: unknown[]) => getPasswordMock(...args),
    getProtectedCustomField: (...args: unknown[]) =>
      getProtectedCustomFieldMock(...args),
  },
}));

const askMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: (...args: unknown[]) => askMock(...args),
}));

vi.mock("@/lib/save-with-error-toast", () => ({
  saveWithErrorToast: vi.fn().mockResolvedValue(undefined),
}));

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
const toastInfoMock = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
    info: (...args: unknown[]) => toastInfoMock(...args),
  },
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
    clearHistoryMock.mockReset();
    getMock.mockReset();
    getPasswordMock.mockReset();
    getProtectedCustomFieldMock.mockReset();
    askMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
    toastInfoMock.mockReset();
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

  it("shows a neutral info message (not a success) when the version is unchanged", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({ index: 0, fingerprint: "fp-zero", changedFields: [] }),
    ]);
    askMock.mockResolvedValue(true);
    // The backend rejects a no-op (e.g. a move-only version) with this message.
    restoreHistoryMock.mockRejectedValue(
      "History version unchanged: this version's content matches the current entry"
    );

    renderSection();
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.restoreVersion",
      })
    );

    await waitFor(() => {
      expect(toastInfoMock).toHaveBeenCalledWith(
        "entries.detail.restoreHistoryUnchanged"
      );
    });
    expect(toastSuccessMock).not.toHaveBeenCalled();
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("clears this Entry's history after the user confirms", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({ index: 0, fingerprint: "fp-zero", changedFields: ["url"] }),
    ]);
    askMock.mockResolvedValue(true);
    clearHistoryMock.mockResolvedValue(undefined);

    renderSection();
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.clearHistory",
      })
    );

    // The destructive action is gated on an explicit confirmation.
    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    await waitFor(() => {
      expect(clearHistoryMock).toHaveBeenCalledWith(TEST_DB_ID, TEST_ENTRY_ID);
    });
  });

  it("does not clear this Entry's history when the user cancels", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({ index: 0, fingerprint: "fp-zero", changedFields: ["url"] }),
    ]);
    askMock.mockResolvedValue(false);

    renderSection();
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.clearHistory",
      })
    );

    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    expect(clearHistoryMock).not.toHaveBeenCalled();
  });

  it("invalidates the password-health report after a restore", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({
        index: 0,
        fingerprint: "fp-zero",
        changedFields: ["password"],
      }),
    ]);
    askMock.mockResolvedValue(true);
    restoreHistoryMock.mockResolvedValue({ id: TEST_ENTRY_ID });

    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false, gcTime: Infinity } },
    });
    // Seed a cached password-health report so we can prove the restore marks
    // it stale (a restore can replace the live password/expiry).
    queryClient.setQueryData(queryKeys.passwordHealth.report(TEST_DB_ID), {
      entries: [],
    });

    render(<EntryHistorySection dbId={TEST_DB_ID} entryId={TEST_ENTRY_ID} />, {
      wrapper: ({ children }: { children: ReactNode }) => (
        <QueryClientProvider client={queryClient}>
          {children}
        </QueryClientProvider>
      ),
    });
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.restoreVersion",
      })
    );

    await waitFor(() => {
      expect(
        queryClient.getQueryState(queryKeys.passwordHealth.report(TEST_DB_ID))
          ?.isInvalidated
      ).toBe(true);
    });
  });

  it("opens the compare dialog for a version, diffing it against the current entry", async () => {
    listHistoryMock.mockResolvedValue([
      makeVersion({ index: 0, title: "Old Title", changedFields: ["title"] }),
    ]);
    getMock.mockResolvedValue({
      id: TEST_ENTRY_ID,
      groupId: "group-1",
      title: "New Title",
      username: "bob",
      url: null,
      notes: null,
      iconId: 0,
      customIconUuid: null,
      tags: [],
      customFields: {},
      customFieldMeta: [],
      createdAt: "2024-01-01T00:00:00Z",
      modifiedAt: "2024-02-17T15:58:43Z",
      accessedAt: "2024-02-17T15:58:43Z",
      expires: false,
      expiryTime: null,
      attachments: [],
    });

    renderSection();
    await expand();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "entries.detail.compare.action",
      })
    );

    // The current title (only rendered inside the dialog) confirms the compare
    // view opened and diffed against the current entry.
    expect(await screen.findByText("New Title")).toBeInTheDocument();
    // "Old Title" appears twice now: the history row and the dialog's before.
    expect(screen.getAllByText("Old Title").length).toBeGreaterThanOrEqual(2);
  });
});
