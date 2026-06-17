// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import type { EntryHistoryItem } from "@/lib/types";

const listHistoryMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  entries: {
    listHistory: (...args: unknown[]) => listHistoryMock(...args),
  },
}));

import {
  EntryHistorySection,
  formatChangedFields,
} from "@/components/entries/EntryHistorySection";

const TEST_DB_ID = "test-vault.kdbx";
const TEST_ENTRY_ID = "entry-1";

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

describe("EntryHistorySection", () => {
  beforeEach(() => {
    listHistoryMock.mockReset();
  });

  it("lists each version with its timestamp once expanded", async () => {
    const versions: EntryHistoryItem[] = [
      {
        index: 0,
        modifiedAt: "2024-02-17T15:58:43Z",
        title: "Example",
        username: "bob",
        url: null,
        changedFields: ["username"],
        isCreation: false,
      },
      {
        index: 1,
        modifiedAt: "2024-01-05T09:30:00Z",
        title: "Example",
        username: "alice",
        url: null,
        changedFields: ["title"],
        isCreation: true,
      },
    ];
    listHistoryMock.mockResolvedValue(versions);

    renderSection();

    // Collapsed by default — expand to reveal the versions.
    const trigger = await screen.findByRole("button", {
      name: "entries.detail.history",
    });
    fireEvent.click(trigger);

    await waitFor(() => {
      expect(screen.getAllByRole("listitem")).toHaveLength(2);
    });
    // Each row carries a human-readable timestamp derived from modifiedAt.
    expect(screen.getByText(/Feb 17, 2024/)).toBeInTheDocument();
    expect(screen.getByText(/Jan 5, 2024/)).toBeInTheDocument();
  });

  it("renders a changed-fields line listing the field names that changed", async () => {
    const versions: EntryHistoryItem[] = [
      {
        index: 0,
        modifiedAt: "2024-02-17T15:58:43Z",
        title: "Example",
        username: "bob",
        url: null,
        changedFields: ["password", "username"],
        // The lone version after a first edit is the creation snapshot, which
        // keeps its changed line (only the earliest-kept case suppresses it).
        isCreation: true,
      },
    ];
    listHistoryMock.mockResolvedValue(versions);

    renderSection();

    const trigger = await screen.findByRole("button", {
      name: "entries.detail.history",
    });
    fireEvent.click(trigger);

    // The interpolated field names are surfaced (i18n is mocked to echo keys,
    // so the changed line is keyed by `entries.detail.historyChanged`).
    await waitFor(() => {
      expect(
        screen.getByText("entries.detail.historyChanged")
      ).toBeInTheDocument();
    });
  });

  it("labels the oldest version 'Created' when it is the original snapshot", async () => {
    const versions: EntryHistoryItem[] = [
      {
        index: 0,
        modifiedAt: "2024-02-17T15:58:43Z",
        title: "Example",
        username: "bob",
        url: null,
        changedFields: ["username"],
        isCreation: false,
      },
      {
        index: 1,
        modifiedAt: "2024-01-05T09:30:00Z",
        title: "Example",
        username: "alice",
        url: null,
        changedFields: ["title"],
        isCreation: true,
      },
    ];
    listHistoryMock.mockResolvedValue(versions);

    renderSection();

    const trigger = await screen.findByRole("button", {
      name: "entries.detail.history",
    });
    fireEvent.click(trigger);

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
    const versions: EntryHistoryItem[] = [
      {
        index: 0,
        modifiedAt: "2024-02-17T15:58:43Z",
        title: "Example",
        username: "bob",
        url: null,
        changedFields: ["username"],
        isCreation: false,
      },
      {
        index: 1,
        modifiedAt: "2024-01-05T09:30:00Z",
        title: "Example",
        username: "alice",
        url: null,
        // changedFields is diffed against the next-newer version, which still
        // exists — so it's accurate even though the original predecessor was
        // pruned. The earliest-kept row keeps its changed line.
        changedFields: ["title"],
        isCreation: false,
      },
    ];
    listHistoryMock.mockResolvedValue(versions);

    renderSection();

    const trigger = await screen.findByRole("button", {
      name: "entries.detail.history",
    });
    fireEvent.click(trigger);

    await waitFor(() => {
      expect(
        screen.getByText("entries.detail.historyEarliestKept")
      ).toBeInTheDocument();
    });
    expect(
      screen.queryByText("entries.detail.historyCreated")
    ).not.toBeInTheDocument();
    // Both rows have a non-empty changedFields, so both render a changed line.
    expect(screen.getAllByText("entries.detail.historyChanged")).toHaveLength(
      2
    );
  });

  it("localizes known changed-field tokens and leaves custom field names verbatim", () => {
    // The backend emits canonical lowercase tokens; the view maps the known
    // ones through localized labels (so non-English locales don't show
    // mixed-language text) while passing user-defined custom field names
    // through untouched.
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

    const trigger = await screen.findByRole("button", {
      name: "entries.detail.history",
    });
    fireEvent.click(trigger);

    await waitFor(() => {
      expect(screen.getByText("entries.detail.noHistory")).toBeInTheDocument();
    });
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });
});
