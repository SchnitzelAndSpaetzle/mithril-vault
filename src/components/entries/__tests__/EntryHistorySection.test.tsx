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

import { EntryHistorySection } from "@/components/entries/EntryHistorySection";

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

  it("labels the oldest version 'Earliest kept' and suppresses its changed line when the original was pruned", async () => {
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
        // A meaningful diff exists, but the earliest-kept row must not show it.
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
    // The newest row still shows its changed line; the earliest-kept row does
    // not — so exactly one changed line renders.
    expect(screen.getAllByText("entries.detail.historyChanged")).toHaveLength(
      1
    );
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
