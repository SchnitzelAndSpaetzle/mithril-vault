// SPDX-License-Identifier: MIT
//
// "Merge from file…" action in the database switcher: picking and merging
// happen entirely in the backend (ADR-0004 provenance — the renderer never
// supplies a path); the component's job is to trigger the command and
// render the Merge Summary toast.

import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import { DatabaseSwitcher } from "../database-switcher";
import { SidebarProvider } from "@/components/ui/sidebar.tsx";
import type { MergeSummary } from "@/lib/types";

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  mergeFromFile: vi.fn(),
  lock: vi.fn(),
  clipboardClear: vi.fn(),
  dialogOpen: vi.fn(),
  toastSuccess: vi.fn(),
  toastWarning: vi.fn(),
  toastError: vi.fn(),
  updateTabInfo: vi.fn(),
}));

vi.mock("@tanstack/react-router", () => ({
  useNavigate: () => mocks.navigate,
}));

vi.mock("@/hooks/use-active-database", () => ({
  useActiveDatabase: () => ({
    tab: {
      id: "tab-1",
      path: "/mock/vault.kdbx",
      info: { name: "My Vault" },
    },
    dbId: "/mock/vault.kdbx",
  }),
}));

vi.mock("@/hooks/use-app-preferences", () => ({
  useAppPreferences: () => ({ preferences: undefined }),
}));

vi.mock("@/hooks/use-recent-databases.ts", () => ({
  useRecentDatabases: () => ({ recentDatabases: [], isLoading: false }),
}));

vi.mock("@/stores/database-tabs", () => ({
  useDatabaseTabs: (selector: (state: unknown) => unknown) =>
    selector({ updateTabInfo: mocks.updateTabInfo }),
}));

vi.mock("@/lib/tauri.ts", () => ({
  database: { mergeFromFile: mocks.mergeFromFile, lock: mocks.lock },
  clipboard: { clear: mocks.clipboardClear },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.dialogOpen }));

vi.mock("sonner", () => ({
  toast: {
    success: mocks.toastSuccess,
    warning: mocks.toastWarning,
    error: mocks.toastError,
  },
}));

// Radix's DropdownMenu uses pointer-capture APIs jsdom doesn't implement.
beforeAll(() => {
  Element.prototype.hasPointerCapture = vi.fn(() => false);
  Element.prototype.releasePointerCapture = vi.fn();
  Element.prototype.scrollIntoView = vi.fn();
});

beforeEach(() => {
  vi.clearAllMocks();
});

function renderSwitcher() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SidebarProvider>
        <DatabaseSwitcher />
      </SidebarProvider>
    </QueryClientProvider>
  );
}

function openDropdownAndClickMerge() {
  const trigger = screen.getByText("My Vault");
  fireEvent.pointerDown(
    trigger,
    new MouseEvent("pointerdown", { bubbles: true, button: 0 })
  );
  fireEvent.click(trigger);
  fireEvent.click(screen.getByText("databaseSwitcher.mergeFromFile"));
}

function summary(overrides: Partial<MergeSummary> = {}): MergeSummary {
  return {
    entriesAdded: 0,
    entriesUpdated: 0,
    entriesDeleted: 0,
    conflicts: [],
    securityPostureChanges: [],
    ...overrides,
  };
}

describe("DatabaseSwitcher merge from file", () => {
  it("merges and reports combined counts and the first conflict", async () => {
    mocks.mergeFromFile.mockResolvedValue(
      summary({
        entriesAdded: 2,
        entriesUpdated: 3,
        conflicts: [{ entryId: "e-1", title: "Netflix" }],
      })
    );

    renderSwitcher();
    openDropdownAndClickMerge();

    await waitFor(() => {
      expect(mocks.mergeFromFile).toHaveBeenCalledWith("/mock/vault.kdbx");
    });
    expect(mocks.toastSuccess).toHaveBeenCalledWith(
      "database.merge.toast.added, database.merge.toast.updated, " +
        "database.merge.toast.conflicts — database.merge.toast.conflictDetail"
    );
    expect(mocks.toastWarning).not.toHaveBeenCalled();
  });

  it("reports no changes when the merge was a no-op", async () => {
    mocks.mergeFromFile.mockResolvedValue(summary());

    renderSwitcher();
    openDropdownAndClickMerge();

    await waitFor(() => {
      expect(mocks.toastSuccess).toHaveBeenCalledWith(
        "database.merge.toast.noChanges"
      );
    });
  });

  it("shows nothing when the user cancels the file pick", async () => {
    mocks.mergeFromFile.mockResolvedValue(null);

    renderSwitcher();
    openDropdownAndClickMerge();

    await waitFor(() => {
      expect(mocks.mergeFromFile).toHaveBeenCalled();
    });
    expect(mocks.toastSuccess).not.toHaveBeenCalled();
    expect(mocks.toastError).not.toHaveBeenCalled();
  });

  it("warns separately when security posture differs", async () => {
    mocks.mergeFromFile.mockResolvedValue(
      summary({ entriesAdded: 1, securityPostureChanges: ["kdf"] })
    );

    renderSwitcher();
    openDropdownAndClickMerge();

    await waitFor(() => {
      expect(mocks.toastWarning).toHaveBeenCalledWith(
        "database.merge.toast.securityPosture"
      );
    });
  });

  it("surfaces merge failures as an error toast", async () => {
    mocks.mergeFromFile.mockRejectedValue(new Error("InvalidPassword"));

    renderSwitcher();
    openDropdownAndClickMerge();

    await waitFor(() => {
      expect(mocks.toastError).toHaveBeenCalledWith(
        "database.merge.toast.failed"
      );
    });
  });
});
