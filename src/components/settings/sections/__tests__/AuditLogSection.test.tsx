// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { queryKeys } from "@/lib/query-keys";

const listMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  audit: {
    list: (...args: unknown[]) => listMock(...args),
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
