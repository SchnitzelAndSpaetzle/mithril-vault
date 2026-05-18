// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { queryKeys } from "@/lib/query-keys";

const listMock = vi.fn();
const clearMock = vi.fn();
const getStatusMock = vi.fn();
const entriesListMock = vi.fn().mockResolvedValue([]);
const getRecentDatabasesMock = vi.fn().mockResolvedValue([]);
const askMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  audit: {
    list: (...args: unknown[]) => listMock(...args),
    clear: (...args: unknown[]) => clearMock(...args),
    getStatus: (...args: unknown[]) => getStatusMock(...args),
  },
  entries: {
    list: (...args: unknown[]) => entriesListMock(...args),
  },
  settings: {
    getRecentDatabases: (...args: unknown[]) => getRecentDatabasesMock(...args),
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: (...args: unknown[]) => askMock(...args),
}));

vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

import { AuditLogSection } from "@/components/settings/sections/AuditLogSection";

const TEST_DB_ID = "test-vault.kdbx";

function createWrapper(setup?: (queryClient: QueryClient) => void) {
  const queryClient = new QueryClient({
    defaultOptions: {
      // gcTime: Infinity keeps observer-less, seeded queries (e.g. entries
      // list) alive long enough for the renderer to resolve entry_id →
      // title. The default `0` would let the cache evict them between
      // setup and the first render.
      queries: { retry: false, gcTime: Infinity },
      mutations: { retry: false },
    },
  });
  setup?.(queryClient);
  return function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  };
}

describe("AuditLogSection", () => {
  beforeEach(() => {
    listMock.mockReset();
    clearMock.mockReset();
    getStatusMock.mockReset();
    getStatusMock.mockResolvedValue({ enabled: true, degraded: false });
    entriesListMock.mockReset();
    entriesListMock.mockResolvedValue([]);
    getRecentDatabasesMock.mockReset();
    getRecentDatabasesMock.mockResolvedValue([]);
    askMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
  });

  it("renders a no-vault empty state and does not query when no vault is open", () => {
    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={null} />
      </Wrapper>
    );

    expect(screen.getByText("audit.emptyNoVault")).toBeInTheDocument();
    expect(listMock).not.toHaveBeenCalled();
  });

  it("renders the empty state when the open vault has no events", async () => {
    listMock.mockResolvedValueOnce({ events: [], degraded: false });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledWith(TEST_DB_ID);
    });
    expect(await screen.findByText("audit.empty")).toBeInTheDocument();
    // The degraded banner must NOT appear when degraded is false.
    expect(screen.queryByText("audit.degradedWarning")).toBeNull();
  });

  it("renders one row per vault.unlock_failed event with kind and attempt count", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "vaultUnlockFailed",
          timestamp: "2026-05-15T12:00:00.000Z",
          attemptCount: 2,
        },
        {
          kind: "vaultUnlockFailed",
          timestamp: "2026-05-15T11:59:00.000Z",
          attemptCount: 1,
        },
      ],
      degraded: false,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    const rows = await screen.findAllByRole("listitem");
    expect(rows.length).toBe(2);
    rows.forEach((row) => {
      expect(row.getAttribute("data-kind")).toBe("vaultUnlockFailed");
      // The kind label is rendered inside each row.
      expect(row.textContent).toContain("audit.kind.vaultUnlockFailed");
    });
  });

  it("renders the degraded banner above the empty state when degraded is true", async () => {
    listMock.mockResolvedValueOnce({ events: [], degraded: true });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    expect(
      await screen.findByText("audit.degradedWarning")
    ).toBeInTheDocument();
    expect(screen.getByText("audit.empty")).toBeInTheDocument();
  });

  it("renders the degraded banner alongside events when degraded is true", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "vaultUnlockFailed",
          timestamp: "2026-05-15T12:00:00.000Z",
          attemptCount: 1,
        },
      ],
      degraded: true,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    expect(
      await screen.findByText("audit.degradedWarning")
    ).toBeInTheDocument();
    expect((await screen.findAllByRole("listitem")).length).toBe(1);
  });

  it("renders vault.opened and vault.locked rows with localized labels and reason", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "vaultLocked",
          timestamp: "2026-05-15T12:05:00.000Z",
          reason: "manual",
        },
        {
          kind: "vaultLocked",
          timestamp: "2026-05-15T12:04:00.000Z",
          reason: "autoLock",
        },
        {
          kind: "vaultLocked",
          timestamp: "2026-05-15T12:03:00.000Z",
          reason: "appQuit",
        },
        {
          kind: "vaultLocked",
          timestamp: "2026-05-15T12:02:00.000Z",
          reason: "screenLock",
        },
        {
          kind: "vaultOpened",
          timestamp: "2026-05-15T12:00:00.000Z",
        },
      ],
      degraded: false,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    const rows = await screen.findAllByRole("listitem");
    expect(rows.length).toBe(5);

    // Each kind label is rendered inside its row. Scope by data-kind so
    // the kind-filter checkboxes (which also render labels) are excluded.
    expect(
      rows.filter((r) => r.getAttribute("data-kind") === "vaultOpened")
    ).toHaveLength(1);
    expect(
      rows.filter((r) => r.getAttribute("data-kind") === "vaultLocked")
    ).toHaveLength(4);

    // Reason labels appear only on locked rows — one per reason variant.
    expect(screen.getByText("audit.reason.manual")).toBeInTheDocument();
    expect(screen.getByText("audit.reason.autoLock")).toBeInTheDocument();
    expect(screen.getByText("audit.reason.appQuit")).toBeInTheDocument();
    expect(screen.getByText("audit.reason.screenLock")).toBeInTheDocument();

    // No attempt-count chip on these kinds.
    expect(screen.queryByText("audit.attemptCount")).toBeNull();
  });

  it("resolves entry_id to the entry's title from the open vault's React Query cache for entry.password_revealed", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "entryPasswordRevealed",
          timestamp: "2026-05-16T10:00:00.000Z",
          entryId: "uuid-abc-1234567890",
        },
      ],
      degraded: false,
    });

    const dbId = TEST_DB_ID;
    const Wrapper = createWrapper((qc) => {
      // Populate the entries list cache the way useEntriesByGroup does.
      qc.setQueryData(queryKeys.entries.list(dbId, null), [
        { id: "uuid-abc-1234567890", title: "GitHub" },
        { id: "uuid-other", title: "Email" },
      ]);
    });
    render(
      <Wrapper>
        <AuditLogSection dbId={dbId} />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.getAttribute("data-kind")).toBe("entryPasswordRevealed");
    expect(row.getAttribute("data-entry-id")).toBe("uuid-abc-1234567890");
    // Title comes from the cache.
    expect(row.textContent).toContain("GitHub");
  });

  it("falls back to the UUID prefix when the entry is not in the cache (vault locked or different vault)", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "entryPasswordRevealed",
          timestamp: "2026-05-16T10:00:00.000Z",
          entryId: "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7",
        },
      ],
      degraded: false,
    });

    // No cache seeded — the entries query is "cold" so the renderer has
    // nothing to resolve against (mirrors the "vault locked" state).
    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.getAttribute("data-kind")).toBe("entryPasswordRevealed");
    // UUID prefix (first 8 chars) as the visible fallback identifier.
    expect(row.textContent).toContain("8f1c2e3a");
    // The full UUID must NOT be rendered — otherwise the row is unreadably long.
    expect(row.textContent).not.toContain(
      "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7"
    );
  });

  it("renders an entry.password_copied row with the resolved title", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "entryPasswordCopied",
          timestamp: "2026-05-16T11:00:00.000Z",
          entryId: "uuid-copied-1",
        },
      ],
      degraded: false,
    });

    const dbId = TEST_DB_ID;
    const Wrapper = createWrapper((qc) => {
      qc.setQueryData(queryKeys.entries.list(dbId, null), [
        { id: "uuid-copied-1", title: "Bank" },
      ]);
    });
    render(
      <Wrapper>
        <AuditLogSection dbId={dbId} />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.getAttribute("data-kind")).toBe("entryPasswordCopied");
    expect(row.textContent).toContain("Bank");
    expect(row.textContent).toContain("audit.kind.entryPasswordCopied");
  });

  it("renders entry.protected_field_revealed with a resolved title from the cache", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "entryProtectedFieldRevealed",
          timestamp: "2026-05-16T12:00:00.000Z",
          entryId: "uuid-pf-1",
        },
      ],
      degraded: false,
    });

    const dbId = TEST_DB_ID;
    const Wrapper = createWrapper((qc) => {
      qc.setQueryData(queryKeys.entries.list(dbId, null), [
        { id: "uuid-pf-1", title: "Recovery codes" },
      ]);
    });
    render(
      <Wrapper>
        <AuditLogSection dbId={dbId} />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.getAttribute("data-kind")).toBe("entryProtectedFieldRevealed");
    expect(row.textContent).toContain("Recovery codes");
    expect(row.textContent).toContain("audit.kind.entryProtectedFieldRevealed");
  });

  it("masks entry titles to UUID prefix when the open vault is currently locked, even if the entries cache is warm", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "entryPasswordRevealed",
          timestamp: "2026-05-16T10:00:00.000Z",
          entryId: "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7",
        },
      ],
      degraded: false,
    });

    const dbId = TEST_DB_ID;
    // Cache is warm — entries were loaded earlier this session — but the
    // vault is now locked. PRD US #16: locked vaults must render entry
    // rows as UUID prefixes so the on-disk log never carries titles
    // outside the unlocked Vault.
    const Wrapper = createWrapper((qc) => {
      qc.setQueryData(queryKeys.entries.list(dbId, null), [
        { id: "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7", title: "GitHub" },
      ]);
    });
    render(
      <Wrapper>
        <AuditLogSection dbId={dbId} isLocked />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.textContent).not.toContain("GitHub");
    expect(row.textContent).toContain("8f1c2e3a");
  });

  it("hydrates entry titles reactively once entries.list resolves after the audit panel mounts", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "entryPasswordRevealed",
          timestamp: "2026-05-16T10:00:00.000Z",
          entryId: "uuid-late",
        },
      ],
      degraded: false,
    });
    // entries.list resolves to data carrying the matching title, but we
    // do NOT pre-seed the cache — the section must subscribe to the
    // entries query, not snapshot it at render time, so the row updates
    // from UUID prefix to title once the IPC settles.
    entriesListMock.mockResolvedValue([{ id: "uuid-late", title: "Late" }]);

    const dbId = TEST_DB_ID;
    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={dbId} />
      </Wrapper>
    );

    // Eventually the row carries the title — pure render-time peek would
    // never reach this state because the cache is empty at mount.
    await waitFor(() => {
      const row = screen.getByRole("listitem");
      expect(row.textContent).toContain("Late");
    });
  });

  it("hides the Clear Audit Log button when no vault is open", () => {
    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={null} />
      </Wrapper>
    );

    expect(screen.queryByText("audit.clearButton")).toBeNull();
  });

  it("renders the Clear Audit Log button when a vault is open", async () => {
    listMock.mockResolvedValueOnce({ events: [], degraded: false });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    expect(
      await screen.findByRole("button", { name: "audit.clearButton" })
    ).toBeInTheDocument();
  });

  it("does not call clear when the confirmation dialog is dismissed", async () => {
    listMock.mockResolvedValueOnce({ events: [], degraded: false });
    askMock.mockResolvedValueOnce(false);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    const btn = await screen.findByRole("button", {
      name: "audit.clearButton",
    });
    fireEvent.click(btn);

    await waitFor(() => {
      expect(askMock).toHaveBeenCalledTimes(1);
    });
    expect(clearMock).not.toHaveBeenCalled();
  });

  it("invokes audit.clear and refetches when the user confirms", async () => {
    // First load: pre-clear log with one event. Second load (after
    // invalidation): the surviving auditCleared event.
    listMock
      .mockResolvedValueOnce({
        events: [
          {
            kind: "vaultUnlockFailed",
            timestamp: "2026-05-16T11:00:00.000Z",
            attemptCount: 1,
          },
        ],
        degraded: false,
      })
      .mockResolvedValueOnce({
        events: [
          {
            kind: "auditCleared",
            timestamp: "2026-05-17T12:00:00.000Z",
          },
        ],
        degraded: false,
      });
    askMock.mockResolvedValueOnce(true);
    clearMock.mockResolvedValueOnce(undefined);

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    // Wait for the initial load to settle.
    await screen.findAllByRole("listitem");

    fireEvent.click(
      await screen.findByRole("button", { name: "audit.clearButton" })
    );

    await waitFor(() => {
      expect(clearMock).toHaveBeenCalledWith(TEST_DB_ID);
    });

    // The panel re-fetches and ends up showing exactly the auditCleared
    // surviving event.
    await waitFor(() => {
      const rows = screen.getAllByRole("listitem");
      expect(rows.length).toBe(1);
      expect(rows[0]?.getAttribute("data-kind")).toBe("auditCleared");
    });
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it("toasts an error and leaves the panel intact when audit.clear rejects", async () => {
    listMock.mockResolvedValue({
      events: [
        {
          kind: "vaultUnlockFailed",
          timestamp: "2026-05-16T11:00:00.000Z",
          attemptCount: 1,
        },
      ],
      degraded: false,
    });
    askMock.mockResolvedValueOnce(true);
    clearMock.mockRejectedValueOnce(new Error("disk full"));

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    await screen.findAllByRole("listitem");
    fireEvent.click(
      await screen.findByRole("button", { name: "audit.clearButton" })
    );

    await waitFor(() => {
      expect(toastErrorMock).toHaveBeenCalled();
    });
    // Pre-clear event still visible — the original log was preserved.
    const rows = screen.getAllByRole("listitem");
    expect(rows[0]?.getAttribute("data-kind")).toBe("vaultUnlockFailed");
  });

  it("renders an auditCleared row with the localized kind label", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "auditCleared",
          timestamp: "2026-05-17T12:00:00.000Z",
        },
      ],
      degraded: false,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.getAttribute("data-kind")).toBe("auditCleared");
    expect(row.textContent).toContain("audit.kind.auditCleared");
  });

  it("renders a preferences.security_changed row with the localized setting-name label", async () => {
    listMock.mockResolvedValueOnce({
      events: [
        {
          kind: "preferencesSecurityChanged",
          timestamp: "2026-05-17T10:00:00.000Z",
          settingName: "security.preventScreenCapture",
        },
      ],
      degraded: false,
    });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    const row = await screen.findByRole("listitem");
    expect(row.getAttribute("data-kind")).toBe("preferencesSecurityChanged");
    expect(row.textContent).toContain("audit.kind.preferencesSecurityChanged");
    // i18n mock echoes keys; the row must render the per-setting label key
    // so each allowlisted leaf gets a human-readable name in production.
    expect(row.textContent).toContain(
      "audit.settingName.security.preventScreenCapture"
    );
  });

  it("renders the loadError state when the backend fails", async () => {
    listMock.mockRejectedValueOnce(new Error("boom"));

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId={TEST_DB_ID} />
      </Wrapper>
    );

    expect(await screen.findByText("audit.loadError")).toBeInTheDocument();
    // No empty-state copy when the load itself failed — distinct UX state.
    expect(screen.queryByText("audit.empty")).toBeNull();
  });

  describe("event-kind filter", () => {
    it("renders all kind checkboxes checked by default", async () => {
      listMock.mockResolvedValue({ events: [], degraded: false });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      const checkbox = await screen.findByRole("checkbox", {
        name: "audit.kind.vaultUnlockFailed",
      });
      expect(checkbox.getAttribute("data-state")).toBe("checked");
    });

    it("hides rows whose kind is unchecked", async () => {
      listMock.mockResolvedValue({
        events: [
          {
            kind: "vaultUnlockFailed",
            timestamp: "2026-05-15T12:00:00.000Z",
            attemptCount: 1,
          },
          {
            kind: "vaultOpened",
            timestamp: "2026-05-15T11:59:00.000Z",
          },
        ],
        degraded: false,
      });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      // Both visible initially.
      const rows = await screen.findAllByRole("listitem");
      expect(rows.length).toBe(2);

      // Uncheck vault.unlock_failed.
      fireEvent.click(
        screen.getByRole("checkbox", {
          name: "audit.kind.vaultUnlockFailed",
        })
      );

      await waitFor(() => {
        const remaining = screen.getAllByRole("listitem");
        expect(remaining.length).toBe(1);
        expect(remaining[0]?.getAttribute("data-kind")).toBe("vaultOpened");
      });
    });

    it("shows the empty state when every kind is unchecked", async () => {
      listMock.mockResolvedValue({
        events: [
          {
            kind: "vaultUnlockFailed",
            timestamp: "2026-05-15T12:00:00.000Z",
            attemptCount: 1,
          },
          {
            kind: "vaultOpened",
            timestamp: "2026-05-15T11:59:00.000Z",
          },
        ],
        degraded: false,
      });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      await screen.findAllByRole("listitem");

      // Click every checkbox to unset all.
      const checkboxes = screen
        .getAllByRole("checkbox")
        .filter((c) => c.getAttribute("data-state") === "checked");
      for (const cb of checkboxes) fireEvent.click(cb);

      await waitFor(() => {
        expect(screen.queryAllByRole("listitem")).toHaveLength(0);
        expect(screen.getByText("audit.empty")).toBeInTheDocument();
      });
    });
  });

  describe("date-range filter", () => {
    it("filters events to the date range (inclusive)", async () => {
      listMock.mockResolvedValue({
        events: [
          {
            kind: "vaultOpened",
            timestamp: "2026-05-15T12:00:00.000Z",
          },
          {
            kind: "vaultOpened",
            timestamp: "2026-05-20T12:00:00.000Z",
          },
          {
            kind: "vaultOpened",
            timestamp: "2026-05-25T12:00:00.000Z",
          },
        ],
        degraded: false,
      });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      await screen.findAllByRole("listitem");

      const from = screen.getByLabelText("audit.filter.from");
      const to = screen.getByLabelText("audit.filter.to");
      fireEvent.change(from, { target: { value: "2026-05-18" } });
      fireEvent.change(to, { target: { value: "2026-05-22" } });

      await waitFor(() => {
        const remaining = screen.getAllByRole("listitem");
        expect(remaining.length).toBe(1);
      });
    });

    it("shows a localized error and does not refetch when from > to", async () => {
      listMock.mockResolvedValue({
        events: [
          {
            kind: "vaultOpened",
            timestamp: "2026-05-15T12:00:00.000Z",
          },
        ],
        degraded: false,
      });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      await screen.findAllByRole("listitem");
      const callsBefore = listMock.mock.calls.length;

      fireEvent.change(screen.getByLabelText("audit.filter.from"), {
        target: { value: "2026-05-22" },
      });
      fireEvent.change(screen.getByLabelText("audit.filter.to"), {
        target: { value: "2026-05-18" },
      });

      const alert = await screen.findByRole("alert");
      expect(alert.textContent).toContain("audit.filter.invalidRange");
      // No further audit.list calls — the query is disabled when the
      // range is invalid.
      expect(listMock.mock.calls.length).toBe(callsBefore);
    });
  });

  describe("degraded indicator from get_audit_status", () => {
    it("renders the warning when getStatus reports degraded even if response degraded is false", async () => {
      getStatusMock.mockResolvedValue({ enabled: true, degraded: true });
      listMock.mockResolvedValue({ events: [], degraded: false });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      expect(
        await screen.findByText("audit.degradedWarning")
      ).toBeInTheDocument();
    });
  });

  describe("virtualization", () => {
    it("switches to a virtualized scroll container above the threshold", async () => {
      // Build > 200 events to cross the virtualization threshold.
      const events = Array.from({ length: 300 }, (_, i) => ({
        kind: "vaultOpened" as const,
        timestamp: new Date(2026, 4, 1, 12, i, 0).toISOString(),
      }));
      listMock.mockResolvedValue({ events, degraded: false });

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId={TEST_DB_ID} />
        </Wrapper>
      );

      expect(
        await screen.findByTestId("audit-virtual-scroll")
      ).toBeInTheDocument();
    });

    // Note: a structural "rows are direct <li> children of the <ul>"
    // test is intentionally omitted. JSDOM has no layout, so
    // @tanstack/react-virtual emits zero virtual items here — the
    // assertion would silently pass even for the regressed `ul > div > li`
    // markup. The code review is the verifier; the runtime invariant is
    // enforced by the `<AuditRow>` component itself rendering an `<li>`
    // (the virtualizer places its positioning style on that `<li>`).
  });

  describe("vault picker", () => {
    it("renders one picker option per recent database (filename + path)", async () => {
      listMock.mockResolvedValue({ events: [], degraded: false });
      getRecentDatabasesMock.mockResolvedValue([
        {
          path: "/Users/alice/Vaults/work.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-15T12:00:00.000Z",
        },
        {
          path: "/Users/alice/Vaults/personal.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-14T12:00:00.000Z",
        },
      ]);

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId="/Users/alice/Vaults/work.kdbx" />
        </Wrapper>
      );

      // The picker exposes a `data-vault-path` for each option so the
      // filename + path display can be verified without depending on the
      // specific dropdown component.
      const opts = await screen.findAllByRole("option");
      expect(opts.length).toBe(2);
      const paths = opts.map((o) => o.getAttribute("data-vault-path"));
      expect(paths).toContain("/Users/alice/Vaults/work.kdbx");
      expect(paths).toContain("/Users/alice/Vaults/personal.kdbx");

      const workOption = opts.find(
        (o) =>
          o.getAttribute("data-vault-path") === "/Users/alice/Vaults/work.kdbx"
      );
      expect(workOption?.textContent).toContain("work.kdbx");
      // Path is also visible so two Vaults with the same filename remain
      // distinguishable to the user.
      expect(workOption?.textContent).toContain(
        "/Users/alice/Vaults/work.kdbx"
      );
    });

    it("defaults to the open Vault and queries audit.list for it", async () => {
      listMock.mockResolvedValue({ events: [], degraded: false });
      getRecentDatabasesMock.mockResolvedValue([
        {
          path: "/Users/alice/Vaults/work.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-15T12:00:00.000Z",
        },
        {
          path: "/Users/alice/Vaults/personal.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-14T12:00:00.000Z",
        },
      ]);

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId="/Users/alice/Vaults/work.kdbx" />
        </Wrapper>
      );

      await waitFor(() => {
        expect(listMock).toHaveBeenCalledWith("/Users/alice/Vaults/work.kdbx");
      });
    });

    it("loads the audit log for the picked Vault when the user changes selection", async () => {
      listMock.mockResolvedValue({ events: [], degraded: false });
      getRecentDatabasesMock.mockResolvedValue([
        {
          path: "/Users/alice/Vaults/work.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-15T12:00:00.000Z",
        },
        {
          path: "/Users/alice/Vaults/personal.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-14T12:00:00.000Z",
        },
      ]);

      const Wrapper = createWrapper();
      render(
        <Wrapper>
          <AuditLogSection dbId="/Users/alice/Vaults/work.kdbx" />
        </Wrapper>
      );

      // Wait until the picker is mounted before changing it.
      await screen.findAllByRole("option");
      const picker = screen.getByLabelText(
        "audit.picker.label"
      ) as HTMLSelectElement;
      fireEvent.change(picker, {
        target: { value: "/Users/alice/Vaults/personal.kdbx" },
      });

      await waitFor(() => {
        expect(listMock).toHaveBeenCalledWith(
          "/Users/alice/Vaults/personal.kdbx"
        );
      });
    });

    it("falls back to UUID prefixes for entries when the picked Vault is not the currently-open one", async () => {
      listMock.mockResolvedValue({
        events: [
          {
            kind: "entryPasswordRevealed",
            timestamp: "2026-05-16T10:00:00.000Z",
            entryId: "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7",
          },
        ],
        degraded: false,
      });
      getRecentDatabasesMock.mockResolvedValue([
        {
          path: "/Users/alice/Vaults/work.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-15T12:00:00.000Z",
        },
        {
          path: "/Users/alice/Vaults/personal.kdbx",
          keyfilePath: null,
          lastOpened: "2026-05-14T12:00:00.000Z",
        },
      ]);

      const dbId = "/Users/alice/Vaults/work.kdbx";
      const Wrapper = createWrapper((qc) => {
        // Cache holds the work vault's entries but the user is looking at
        // personal's audit log. The on-disk audit log lives outside the
        // unlocked Vault scope, so entry titles must not bleed across.
        qc.setQueryData(queryKeys.entries.list(dbId, null), [
          { id: "8f1c2e3a-4b5d-6e7f-8091-a2b3c4d5e6f7", title: "GitHub" },
        ]);
      });
      render(
        <Wrapper>
          <AuditLogSection dbId={dbId} />
        </Wrapper>
      );

      await screen.findAllByRole("option");
      const picker = screen.getByLabelText(
        "audit.picker.label"
      ) as HTMLSelectElement;
      fireEvent.change(picker, {
        target: { value: "/Users/alice/Vaults/personal.kdbx" },
      });

      const row = await screen.findByRole("listitem");
      expect(row.textContent).toContain("8f1c2e3a");
      expect(row.textContent).not.toContain("GitHub");
    });
  });
});
