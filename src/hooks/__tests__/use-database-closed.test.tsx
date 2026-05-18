// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";

const listenMock = vi.fn();
const navigateMock = vi.fn();
const lockTabMock = vi.fn();
const tauriEventListeners = new Map<
  string,
  (event: { payload: unknown }) => void
>();

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => navigateMock,
}));

vi.mock("@/stores/database-tabs", () => {
  interface Tab {
    id: string;
    path?: string;
    dbId?: string;
    info?: { path?: string };
  }
  const state: { tabs: Tab[]; activeTabId: string | null } = {
    tabs: [],
    activeTabId: null,
  };
  const store = {
    getState: () => state,
  };
  return {
    useDatabaseTabs: Object.assign(
      (selector?: (s: unknown) => unknown) =>
        selector
          ? selector({ tabs: state.tabs, lockTab: lockTabMock })
          : { tabs: state.tabs, lockTab: lockTabMock },
      store
    ),
    __setTabsState: (next: { tabs: Tab[]; activeTabId: string | null }) => {
      state.tabs = next.tabs;
      state.activeTabId = next.activeTabId;
    },
  };
});

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { queryKeys } from "@/lib/query-keys";
import { useDatabaseClosed } from "@/hooks/use-database-closed";

import * as databaseTabsMock from "@/stores/database-tabs";

function HookHost() {
  useDatabaseClosed();
  return null;
}

function renderWithClient(client: QueryClient) {
  return render(
    <QueryClientProvider client={client}>
      <HookHost />
    </QueryClientProvider>
  );
}

describe("useDatabaseClosed", () => {
  beforeEach(() => {
    listenMock.mockReset();
    navigateMock.mockReset();
    lockTabMock.mockReset();
    tauriEventListeners.clear();

    listenMock.mockImplementation(
      (eventName: string, listener: (event: { payload: unknown }) => void) => {
        tauriEventListeners.set(eventName, listener);
        return Promise.resolve(() => {
          tauriEventListeners.delete(eventName);
        });
      }
    );
  });

  it("evicts every cached query scoped to the closed Vault path", async () => {
    const path = "/mock/vault.kdbx";
    const otherPath = "/mock/other.kdbx";
    const client = new QueryClient();

    // Pre-populate caches for the affected Vault across every domain that
    // gets evicted, plus a sibling Vault to confirm scoping.
    client.setQueryData(queryKeys.database.info(path), { stale: "yes" });
    client.setQueryData(queryKeys.entries.list(path, null), ["e1", "e2"]);
    client.setQueryData(queryKeys.groups.list(path, null), ["g1"]);
    client.setQueryData(queryKeys.backups.list(path), [{ path: "b" }]);
    client.setQueryData(queryKeys.database.info(otherPath), { keep: "me" });

    // No matching tab — keeps the test focused on the cache eviction. The
    // tab-locking and navigation paths are exercised separately.
    (
      databaseTabsMock as unknown as {
        __setTabsState: (s: { tabs: never[]; activeTabId: null }) => void;
      }
    ).__setTabsState({ tabs: [], activeTabId: null });

    renderWithClient(client);

    // Wait for the async listen() to register before firing the event.
    await Promise.resolve();
    const handler = tauriEventListeners.get("database-closed");
    if (!handler)
      throw new Error("database-closed listener not registered yet");
    handler({ payload: { path, reason: "restore" } });

    expect(client.getQueryData(queryKeys.database.info(path))).toBeUndefined();
    expect(
      client.getQueryData(queryKeys.entries.list(path, null))
    ).toBeUndefined();
    expect(
      client.getQueryData(queryKeys.groups.list(path, null))
    ).toBeUndefined();
    expect(client.getQueryData(queryKeys.backups.list(path))).toBeUndefined();
    // The sibling Vault's data must be untouched.
    expect(client.getQueryData(queryKeys.database.info(otherPath))).toEqual({
      keep: "me",
    });
  });
});
