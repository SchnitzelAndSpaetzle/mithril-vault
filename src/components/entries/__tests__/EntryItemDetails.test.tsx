// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import EntryItemDetails from "../EntryItemDetails";
import { useEntryDetail } from "@/hooks/use-entry-detail";
import { clipboard, entries as entriesApi } from "@/lib/tauri";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Entry } from "@/lib/types";

const mockEntry: Entry = {
  id: "entry-1",
  groupId: "group-1",
  title: "Test Entry",
  username: "user@example.com",
  url: "https://example.com",
  notes: "Some notes here",
  iconId: 0,
  customIconUuid: null,
  tags: ["work", "dev"],
  customFields: { "Custom Key": "custom value" },
  customFieldMeta: [{ key: "Custom Key", isProtected: false }],
  createdAt: "2024-02-17T15:56:34Z",
  modifiedAt: "2024-02-17T15:58:43Z",
  accessedAt: "2024-02-17T15:58:43Z",
};

vi.mock("@/hooks/use-entry-detail", () => ({
  useEntryDetail: vi.fn(() => ({
    entry: mockEntry,
    isLoading: false,
    isError: false,
    password: null,
    isPasswordVisible: false,
    isPasswordLoading: false,
    isTransitioning: false,
    revealPassword: vi.fn(),
    hidePassword: vi.fn(),
  })),
}));

vi.mock("@/hooks/use-custom-icons", () => ({
  useCustomIcons: vi.fn(() => ({ data: {} })),
}));

vi.mock("@/hooks/use-clipboard-timeout", () => ({
  useClipboardTimeout: vi.fn(() => 45),
}));

vi.mock("@/lib/tauri", () => ({
  clipboard: { copyPassword: vi.fn() },
  entries: { getProtectedCustomField: vi.fn() },
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

function makeHookResult(
  overrides: Partial<ReturnType<typeof useEntryDetail>> = {}
) {
  return {
    entry: mockEntry,
    isLoading: false,
    isError: false,
    password: null,
    isPasswordVisible: false,
    isPasswordLoading: false,
    isTransitioning: false,
    revealPassword: vi.fn(),
    hidePassword: vi.fn(),
    ...overrides,
  };
}

describe("EntryItemDetails", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useEntryDetail).mockReturnValue(makeHookResult());
  });

  it("renders entry title", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("Test Entry")).toBeInTheDocument();
  });

  it("renders username", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("user@example.com")).toBeInTheDocument();
  });

  it("renders password as masked by default", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("••••••••")).toBeInTheDocument();
  });

  it("renders URL", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("https://example.com")).toBeInTheDocument();
  });

  it("renders tags", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("work")).toBeInTheDocument();
    expect(screen.getByText("dev")).toBeInTheDocument();
  });

  it("renders notes", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("Some notes here")).toBeInTheDocument();
  });

  it("renders custom fields", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("Custom Key")).toBeInTheDocument();
    expect(screen.getByText("custom value")).toBeInTheDocument();
  });

  it("renders metadata dates", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    expect(screen.getByText("Created")).toBeInTheDocument();
    expect(screen.getByText("Modified")).toBeInTheDocument();
  });

  it("shows skeleton when loading", async () => {
    vi.mocked(useEntryDetail).mockReturnValueOnce({
      ...makeHookResult(),
      entry: null,
      isLoading: true,
    });

    const { container } = render(
      <EntryItemDetails entryId="entry-1" dbId="db-1" />
    );
    const skeletons = container.querySelectorAll('[data-slot="skeleton"]');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it("copies password via backend clipboard command", async () => {
    vi.mocked(clipboard.copyPassword).mockResolvedValueOnce(undefined);
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    const passwordText = screen.getByText("••••••••");
    await act(async () => {
      fireEvent.click(passwordText.closest("button") as HTMLButtonElement);
    });
    expect(clipboard.copyPassword).toHaveBeenCalledWith("db-1", "entry-1", 45);
  });

  it("calls reveal and hide handlers from useEntryDetail", () => {
    const revealPassword = vi.fn();
    const hidePassword = vi.fn();

    vi.mocked(useEntryDetail)
      .mockReturnValueOnce(
        makeHookResult({
          revealPassword,
          hidePassword,
          isPasswordVisible: false,
          password: null,
        })
      )
      .mockReturnValueOnce(
        makeHookResult({
          revealPassword,
          hidePassword,
          isPasswordVisible: true,
          password: "super-secret",
        })
      );

    const { rerender } = render(
      <EntryItemDetails entryId="entry-1" dbId="db-1" />
    );
    fireEvent.click(screen.getByRole("button", { name: "reveal password" }));
    expect(revealPassword).toHaveBeenCalledTimes(1);

    rerender(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    fireEvent.click(screen.getByRole("button", { name: "hide password" }));
    expect(hidePassword).toHaveBeenCalledTimes(1);
  });

  it("opens URL through tauri opener", () => {
    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    fireEvent.click(screen.getByRole("button", { name: "open url" }));
    expect(openUrl).toHaveBeenCalledWith("https://example.com");
  });

  it("reveals protected custom field value through backend API", async () => {
    const protectedEntry: Entry = {
      ...mockEntry,
      customFields: {},
      customFieldMeta: [{ key: "API Token", isProtected: true }],
    };
    vi.mocked(useEntryDetail).mockReturnValueOnce(
      makeHookResult({ entry: protectedEntry })
    );
    vi.mocked(entriesApi.getProtectedCustomField).mockResolvedValueOnce({
      key: "API Token",
      value: "token-123",
    });

    render(<EntryItemDetails entryId="entry-1" dbId="db-1" />);
    fireEvent.click(screen.getByRole("button", { name: "reveal API Token" }));

    await waitFor(() => {
      expect(entriesApi.getProtectedCustomField).toHaveBeenCalledWith(
        "db-1",
        "entry-1",
        "API Token"
      );
      expect(screen.getByText("token-123")).toBeInTheDocument();
    });
  });

  it("disables sensitive actions while transitioning between entries", () => {
    vi.mocked(useEntryDetail).mockReturnValueOnce({
      ...makeHookResult(),
      isTransitioning: true,
    });

    render(<EntryItemDetails entryId="entry-2" dbId="db-1" />);
    expect(
      screen.getByRole("button", { name: "reveal password" })
    ).toBeDisabled();
    expect(screen.getByRole("button", { name: "open url" })).toBeDisabled();
  });
});
