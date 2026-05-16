// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";

const listMock = vi.fn();

vi.mock("@/lib/tauri", () => ({
  audit: {
    list: (...args: unknown[]) => listMock(...args),
  },
}));

import { AuditLogSection } from "@/components/settings/sections/AuditLogSection";

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
