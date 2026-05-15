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
    listMock.mockResolvedValueOnce([]);

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
  });

  it("renders one row per vault.unlock_failed event with kind and attempt count", async () => {
    listMock.mockResolvedValueOnce([
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
    ]);

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

  it("renders the loadError state when the backend fails", async () => {
    listMock.mockRejectedValueOnce(new Error("boom"));

    const Wrapper = createWrapper();
    render(
      <Wrapper>
        <AuditLogSection dbId="/tmp/vault.kdbx" />
      </Wrapper>
    );

    expect(await screen.findByText("audit.loadError")).toBeInTheDocument();
  });
});
