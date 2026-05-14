// SPDX-License-Identifier: MIT

import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import type { ReactNode } from "react";

const listMock = vi.fn();
const deleteMock = vi.fn();
const listenMock = vi.fn();
const tauriEventListeners = new Map<
  string,
  (event: { payload: unknown }) => void
>();

vi.mock("@/lib/tauri", () => ({
  backups: {
    list: (...args: unknown[]) => listMock(...args),
    delete: (...args: unknown[]) => deleteMock(...args),
  },
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

beforeAll(() => {
  if (!Element.prototype.scrollIntoView) {
    Element.prototype.scrollIntoView = vi.fn();
  }
});

import { BackupsListSection } from "@/components/settings/sections/BackupsListSection";

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
}

describe("BackupsListSection", () => {
  beforeEach(() => {
    listMock.mockReset();
    deleteMock.mockReset();
    listenMock.mockReset();
    tauriEventListeners.clear();

    // Default: listen() resolves to a noop unlisten and captures the handler
    // so individual tests can fire backup-created / backup-deleted payloads.
    listenMock.mockImplementation(
      (eventName: string, listener: (event: { payload: unknown }) => void) => {
        tauriEventListeners.set(eventName, listener);
        return Promise.resolve(() => {
          tauriEventListeners.delete(eventName);
        });
      }
    );
  });

  it("renders an empty state when no vault is open", () => {
    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <BackupsListSection dbId={null} />
      </Wrapper>
    );

    // i18n is mocked to echo keys; the no-vault-open empty state has its own
    // key, distinct from the no-backups-yet copy.
    expect(
      screen.getByText("settings.backups.list.emptyNoVault")
    ).toBeInTheDocument();
    // The backend must not be queried when there is no active vault.
    expect(listMock).not.toHaveBeenCalled();
  });

  it("renders a row per listed backup with timestamp, size and kind", async () => {
    listMock.mockResolvedValueOnce([
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx",
        timestamp: "2026-05-12T14:30:45.123Z",
        sizeBytes: 4096,
        kind: "auto",
      },
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.manual.20260101T000000.000Z.kdbx",
        timestamp: "2026-01-01T00:00:00.000Z",
        sizeBytes: 1024 * 1024,
        kind: "manual",
      },
    ]);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <BackupsListSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    // Wait for the populated list — the empty state and skeleton give way
    // to two rows once the fetch resolves.
    await waitFor(() => {
      expect(listMock).toHaveBeenCalledWith("/tmp/vault.kdbx");
    });

    const rows = await screen.findAllByRole("listitem");
    expect(rows.length).toBe(2);

    // Auto row carries the auto badge; manual row carries the manual badge.
    const autoRow = rows[0];
    const manualRow = rows[1];
    if (!autoRow || !manualRow) throw new Error("expected two rows");
    expect(
      within(autoRow).getByText("settings.backups.list.kind.auto")
    ).toBeInTheDocument();
    expect(
      within(manualRow).getByText("settings.backups.list.kind.manual")
    ).toBeInTheDocument();
  });

  it("calls backups.delete with the row path when the delete button is confirmed", async () => {
    const snapshotPath =
      "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx";
    listMock.mockResolvedValue([
      {
        path: snapshotPath,
        timestamp: "2026-05-12T14:30:45.123Z",
        sizeBytes: 4096,
        kind: "auto",
      },
    ]);
    deleteMock.mockResolvedValueOnce(undefined);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <BackupsListSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    const deleteButton = await screen.findByRole("button", {
      name: "settings.backups.list.delete",
    });
    fireEvent.click(deleteButton);

    // Inline confirm — a second button (or the same button morphing) actually
    // executes the delete.
    const confirmButton = await screen.findByRole("button", {
      name: "settings.backups.list.deleteConfirm",
    });
    fireEvent.click(confirmButton);

    await waitFor(() => {
      expect(deleteMock).toHaveBeenCalledWith(snapshotPath);
    });
  });

  it("refetches when a backup-created event fires", async () => {
    listMock.mockResolvedValueOnce([]);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <BackupsListSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledTimes(1);
    });

    // Now the backend emits a created event; the section must subscribe and
    // refetch the list without the user pressing anything.
    listMock.mockResolvedValueOnce([
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.20260513T000000.000Z.kdbx",
        timestamp: "2026-05-13T00:00:00.000Z",
        sizeBytes: 4096,
        kind: "auto",
      },
    ]);
    const handler = tauriEventListeners.get("backup-created");
    if (!handler) throw new Error("backup-created listener not registered");
    handler({ payload: { path: "/tmp/.kdbx-backups/anything" } });

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledTimes(2);
    });
  });

  it("refetches when a backup-deleted event fires", async () => {
    listMock.mockResolvedValueOnce([
      {
        path: "/tmp/.kdbx-backups/vault.kdbx.backup.20260512T143045.123Z.kdbx",
        timestamp: "2026-05-12T14:30:45.123Z",
        sizeBytes: 4096,
        kind: "auto",
      },
    ]);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <BackupsListSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledTimes(1);
    });

    listMock.mockResolvedValueOnce([]);
    const handler = tauriEventListeners.get("backup-deleted");
    if (!handler) throw new Error("backup-deleted listener not registered");
    handler({ payload: { path: "/tmp/.kdbx-backups/anything" } });

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledTimes(2);
    });
  });
});
