// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import type { Entry, EntryHistoryItem } from "@/lib/types";

const getMock = vi.fn();
const getPasswordMock = vi.fn();
const getProtectedCustomFieldMock = vi.fn();
const getHistoryPasswordMock = vi.fn();
const getHistoryProtectedFieldMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  entries: {
    get: (...args: unknown[]) => getMock(...args),
    getPassword: (...args: unknown[]) => getPasswordMock(...args),
    getProtectedCustomField: (...args: unknown[]) =>
      getProtectedCustomFieldMock(...args),
    getHistoryPassword: (...args: unknown[]) => getHistoryPasswordMock(...args),
    getHistoryProtectedField: (...args: unknown[]) =>
      getHistoryProtectedFieldMock(...args),
  },
}));

import {
  changedSince,
  EntryHistoryCompareDialog,
} from "@/components/entries/EntryHistoryCompareDialog";

const TEST_DB_ID = "test-vault.kdbx";
const TEST_ENTRY_ID = "entry-1";

function makeEntry(overrides: Partial<Entry> = {}): Entry {
  return {
    id: TEST_ENTRY_ID,
    groupId: "group-1",
    title: "New Name",
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
    ...overrides,
  };
}

function makeVersion(
  overrides: Partial<EntryHistoryItem> & Pick<EntryHistoryItem, "index">
): EntryHistoryItem {
  return {
    modifiedAt: "2024-02-10T10:00:00Z",
    title: "New Name",
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
    defaultOptions: { queries: { retry: false, gcTime: Infinity } },
  });
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
}

function renderDialog(props: {
  version: EntryHistoryItem;
  changedFields: string[];
}) {
  return render(
    <EntryHistoryCompareDialog
      dbId={TEST_DB_ID}
      entryId={TEST_ENTRY_ID}
      version={props.version}
      changedFields={props.changedFields}
      open
      onOpenChange={vi.fn()}
    />,
    { wrapper: createWrapper() }
  );
}

describe("changedSince", () => {
  // Versions are newest-first; each version's changedFields is the diff against
  // the next-newer version (the newest against the live Entry). The union from
  // index 0 through the selected index is "what changed since this version".
  const versions = [
    makeVersion({ index: 0, changedFields: ["title"] }),
    makeVersion({ index: 1, changedFields: ["username"] }),
    makeVersion({ index: 2, changedFields: ["url", "title"] }),
  ];

  it("returns the newest version's own changed fields at index 0", () => {
    expect(changedSince(versions, 0)).toEqual(["title"]);
  });

  it("unions changed fields from the newest version through the selected one", () => {
    expect(new Set(changedSince(versions, 2))).toEqual(
      new Set(["title", "username", "url"])
    );
  });

  it("de-duplicates a field changed in more than one version", () => {
    // "title" appears in versions 0 and 2 but should be listed once.
    expect(changedSince(versions, 2).filter((f) => f === "title")).toHaveLength(
      1
    );
  });
});

describe("EntryHistoryCompareDialog", () => {
  beforeEach(() => {
    getMock.mockReset();
    getPasswordMock.mockReset();
    getProtectedCustomFieldMock.mockReset();
    getHistoryPasswordMock.mockReset();
    getHistoryProtectedFieldMock.mockReset();
  });

  it("shows a changed text field as historical → current", async () => {
    getMock.mockResolvedValue(makeEntry({ title: "New Name" }));
    renderDialog({
      version: makeVersion({ index: 1, title: "Old Name", changedFields: [] }),
      changedFields: ["title"],
    });

    expect(await screen.findByText("Old Name")).toBeInTheDocument();
    expect(screen.getByText("New Name")).toBeInTheDocument();
  });

  it("describes which version is being compared", async () => {
    getMock.mockResolvedValue(makeEntry());
    renderDialog({
      version: makeVersion({ index: 1 }),
      changedFields: [],
    });

    // A dialog description (which version, by date) gives the comparison
    // context and satisfies the dialog's accessibility contract.
    expect(
      await screen.findByText("entries.detail.compare.description")
    ).toBeInTheDocument();
  });

  it("shows an empty state when nothing differs", async () => {
    getMock.mockResolvedValue(makeEntry({ title: "Same" }));
    renderDialog({
      version: makeVersion({ index: 1, title: "Same" }),
      changedFields: ["title"],
    });

    expect(
      await screen.findByText("entries.detail.compare.noChanges")
    ).toBeInTheDocument();
  });

  it("compares username and url as historical → current", async () => {
    getMock.mockResolvedValue(
      makeEntry({ username: "bob@example.com", url: "https://new.example" })
    );
    renderDialog({
      version: makeVersion({
        index: 1,
        username: "bob",
        url: "https://old.example",
      }),
      changedFields: ["username", "url"],
    });

    expect(await screen.findByText("bob")).toBeInTheDocument();
    expect(screen.getByText("bob@example.com")).toBeInTheDocument();
    expect(screen.getByText("https://old.example")).toBeInTheDocument();
    expect(screen.getByText("https://new.example")).toBeInTheDocument();
  });

  it("shows the current value and a 'previous not available' note for value-less fields", async () => {
    getMock.mockResolvedValue(makeEntry({ notes: "Current notes text" }));
    renderDialog({
      version: makeVersion({ index: 1 }),
      changedFields: ["notes"],
    });

    expect(await screen.findByText("Current notes text")).toBeInTheDocument();
    expect(
      screen.getByText("entries.detail.compare.previousNotAvailable")
    ).toBeInTheDocument();
  });

  it("reveals both passwords only on demand and hides them again", async () => {
    getMock.mockResolvedValue(makeEntry());
    getPasswordMock.mockResolvedValue("newpass");
    getHistoryPasswordMock.mockResolvedValue("oldpass");
    renderDialog({
      version: makeVersion({ index: 1, fingerprint: "fp-1" }),
      changedFields: ["password"],
    });

    const revealButton = await screen.findByRole("button", {
      name: "entries.detail.revealPassword",
    });
    // Nothing is fetched until the explicit reveal.
    expect(getPasswordMock).not.toHaveBeenCalled();
    expect(getHistoryPasswordMock).not.toHaveBeenCalled();
    expect(screen.queryByText("oldpass")).not.toBeInTheDocument();

    fireEvent.click(revealButton);

    expect(await screen.findByText("oldpass")).toBeInTheDocument();
    expect(screen.getByText("newpass")).toBeInTheDocument();
    expect(getPasswordMock).toHaveBeenCalledWith(TEST_DB_ID, TEST_ENTRY_ID);
    expect(getHistoryPasswordMock).toHaveBeenCalledWith(
      TEST_DB_ID,
      TEST_ENTRY_ID,
      1,
      "fp-1"
    );

    const hideButton = screen.getByRole("button", {
      name: "entries.detail.hidePassword",
    });
    fireEvent.click(hideButton);

    await waitFor(() => {
      expect(screen.queryByText("oldpass")).not.toBeInTheDocument();
    });
    expect(screen.queryByText("newpass")).not.toBeInTheDocument();
  });

  it("reveals a protected custom field's values on demand", async () => {
    getMock.mockResolvedValue(
      makeEntry({
        customFields: { "API Key": "" },
        customFieldMeta: [{ key: "API Key", isProtected: true }],
      })
    );
    getHistoryProtectedFieldMock.mockResolvedValue({
      key: "API Key",
      value: "old-secret",
    });
    getProtectedCustomFieldMock.mockResolvedValue({
      key: "API Key",
      value: "new-secret",
    });
    renderDialog({
      version: makeVersion({
        index: 2,
        fingerprint: "fp-2",
        protectedFields: ["API Key"],
      }),
      changedFields: ["API Key"],
    });

    const revealButton = await screen.findByRole("button", {
      name: "entries.detail.revealField",
    });
    expect(getHistoryProtectedFieldMock).not.toHaveBeenCalled();

    fireEvent.click(revealButton);

    expect(await screen.findByText("old-secret")).toBeInTheDocument();
    expect(screen.getByText("new-secret")).toBeInTheDocument();
    expect(getHistoryProtectedFieldMock).toHaveBeenCalledWith(
      TEST_DB_ID,
      TEST_ENTRY_ID,
      2,
      "fp-2",
      "API Key"
    );
    expect(getProtectedCustomFieldMock).toHaveBeenCalledWith(
      TEST_DB_ID,
      TEST_ENTRY_ID,
      "API Key"
    );
  });

  it("shows a plain custom field's current value with a 'previous not available' note", async () => {
    getMock.mockResolvedValue(
      makeEntry({
        customFields: { Department: "Engineering" },
        customFieldMeta: [{ key: "Department", isProtected: false }],
      })
    );
    renderDialog({
      version: makeVersion({ index: 1 }),
      changedFields: ["Department"],
    });

    expect(await screen.findByText("Department")).toBeInTheDocument();
    expect(screen.getByText("Engineering")).toBeInTheDocument();
    expect(
      screen.getByText("entries.detail.compare.previousNotAvailable")
    ).toBeInTheDocument();
    // A plain field is never fetched as a secret.
    expect(getProtectedCustomFieldMock).not.toHaveBeenCalled();
  });
});
