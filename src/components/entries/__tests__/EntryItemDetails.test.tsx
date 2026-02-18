// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import EntryItemDetails from "../EntryItemDetails";
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
    revealPassword: vi.fn(),
    hidePassword: vi.fn(),
  })),
}));

vi.mock("@/hooks/use-custom-icons", () => ({
  useCustomIcons: vi.fn(() => ({ data: {} })),
}));

vi.mock("@/lib/tauri", () => ({
  clipboard: { copyPassword: vi.fn() },
  entries: { getProtectedCustomField: vi.fn() },
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

describe("EntryItemDetails", () => {
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
    const { useEntryDetail } = await import("@/hooks/use-entry-detail");
    vi.mocked(useEntryDetail).mockReturnValueOnce({
      entry: null,
      isLoading: true,
      isError: false,
      password: null,
      isPasswordVisible: false,
      isPasswordLoading: false,
      revealPassword: vi.fn(),
      hidePassword: vi.fn(),
    });

    const { container } = render(
      <EntryItemDetails entryId="entry-1" dbId="db-1" />
    );
    const skeletons = container.querySelectorAll('[data-slot="skeleton"]');
    expect(skeletons.length).toBeGreaterThan(0);
  });
});
