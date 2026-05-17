// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { queryKeys } from "@/lib/query-keys";

const listMock = vi.fn();
const clearMock = vi.fn();
const entriesListMock = vi.fn().mockResolvedValue([]);
const askMock = vi.fn();
const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  audit: {
    list: (...args: unknown[]) => listMock(...args),
    clear: (...args: unknown[]) => clearMock(...args),
  },
  entries: {
    list: (...args: unknown[]) => entriesListMock(...args),
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
    entriesListMock.mockReset();
    entriesListMock.mockResolvedValue([]);
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    await waitFor(() => {
      expect(listMock).toHaveBeenCalledWith("/tmp/vault.kdbx");
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    const rows = await screen.findAllByRole("listitem");
    expect(rows.length).toBe(2);
    rows.forEach((row) => {
      expect(row.getAttribute("data-kind")).toBe("vaultUnlockFailed");
    });
    // i18n is mocked to echo keys; each row carries the kind label.
    const labels = screen.getAllByText("audit.kind.vaultUnlockFailed");
    expect(labels.length).toBe(2);
  });

  it("renders the degraded banner above the empty state when degraded is true", async () => {
    listMock.mockResolvedValueOnce({ events: [], degraded: true });

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    const rows = await screen.findAllByRole("listitem");
    expect(rows.length).toBe(5);

    // Each kind label is rendered via i18n; the mocked t() echoes keys.
    expect(screen.getAllByText("audit.kind.vaultOpened").length).toBe(1);
    expect(screen.getAllByText("audit.kind.vaultLocked").length).toBe(4);

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

    const dbId = "/tmp/vault.kdbx";
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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

    const dbId = "/tmp/vault.kdbx";
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

    const dbId = "/tmp/vault.kdbx";
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

    const dbId = "/tmp/vault.kdbx";
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

    const dbId = "/tmp/vault.kdbx";
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    // Wait for the initial load to settle.
    await screen.findAllByRole("listitem");

    fireEvent.click(
      await screen.findByRole("button", { name: "audit.clearButton" })
    );

    await waitFor(() => {
      expect(clearMock).toHaveBeenCalledWith("/tmp/vault.kdbx");
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
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
        <AuditLogSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    expect(await screen.findByText("audit.loadError")).toBeInTheDocument();
    // No empty-state copy when the load itself failed — distinct UX state.
    expect(screen.queryByText("audit.empty")).toBeNull();
  });
});
