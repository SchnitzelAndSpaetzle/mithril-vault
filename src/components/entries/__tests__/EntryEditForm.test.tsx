// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  act,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { EntryEditForm } from "../EntryEditForm";
import type { Entry } from "@/lib/types";
import React from "react";

const {
  mockCreateEntry,
  mockUpdateEntry,
  mockMoveEntry,
  mockListEntries,
  mockGetEntry,
  mockGetPassword,
  mockGetProtectedCustomField,
  mockDatabaseSave,
  mockGetCustomIcons,
  mockFetchFavicon,
  mockClearCustomIcon,
  mockSetCustomIcon,
  mockToast,
  mockAutoDownloadFavicons,
} = vi.hoisted(() => ({
  mockCreateEntry: vi.fn(),
  mockUpdateEntry: vi.fn(),
  mockMoveEntry: vi.fn(),
  mockGetEntry: vi.fn(),
  mockListEntries: vi.fn(() =>
    Promise.resolve([
      {
        id: "entry-a",
        groupId: "group-1",
        title: "Entry A",
        username: "alice@example.com",
        url: null,
        notes: null,
        iconId: 0,
        customIconUuid: null,
        tags: [],
        customFields: {},
        customFieldMeta: [],
        createdAt: "2024-02-17T15:56:34Z",
        modifiedAt: "2024-02-17T15:58:43Z",
        accessedAt: "2024-02-17T15:58:43Z",
        expires: false,
      },
      {
        id: "entry-b",
        groupId: "group-1",
        title: "Entry B",
        username: "bob@example.com",
        url: null,
        notes: null,
        iconId: 0,
        customIconUuid: null,
        tags: [],
        customFields: {},
        customFieldMeta: [],
        createdAt: "2024-02-17T15:56:34Z",
        modifiedAt: "2024-02-17T15:58:43Z",
        accessedAt: "2024-02-17T15:58:43Z",
        expires: false,
      },
    ])
  ),
  mockGetPassword: vi.fn(() => Promise.resolve("existing-password")),
  mockGetProtectedCustomField: vi.fn(() =>
    Promise.resolve({ key: "secret", value: "secret-value" })
  ),
  mockDatabaseSave: vi.fn(() => Promise.resolve()),
  mockGetCustomIcons: vi.fn(() => Promise.resolve({})),
  mockFetchFavicon: vi.fn<
    (...args: unknown[]) => Promise<"updated" | "unchanged" | "notFound">
  >(() => Promise.resolve("notFound")),
  mockClearCustomIcon: vi.fn(() => Promise.resolve(false)),
  mockSetCustomIcon: vi.fn(() => Promise.resolve(true)),
  mockAutoDownloadFavicons: { value: false },
  mockToast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("@/hooks/use-entry-mutations", () => ({
  useEntryMutations: vi.fn(() => ({
    createEntry: { mutateAsync: mockCreateEntry, isPending: false },
    updateEntry: { mutateAsync: mockUpdateEntry, isPending: false },
    moveEntry: { mutateAsync: mockMoveEntry, isPending: false },
    deleteEntry: { mutateAsync: vi.fn(), isPending: false },
  })),
}));

vi.mock("@/hooks/use-app-preferences", () => ({
  useAppPreferences: () => ({
    preferences: {
      security: {
        autoDownloadFavicons: mockAutoDownloadFavicons.value,
      },
    },
  }),
}));

vi.mock("@/lib/tauri", () => ({
  database: {
    save: mockDatabaseSave,
    getCustomIcons: mockGetCustomIcons,
  },
  entries: {
    list: mockListEntries,
    get: mockGetEntry,
    getPassword: mockGetPassword,
    getProtectedCustomField: mockGetProtectedCustomField,
    fetchFavicon: mockFetchFavicon,
    clearCustomIcon: mockClearCustomIcon,
    setCustomIcon: mockSetCustomIcon,
  },
  generator: {
    generate: vi.fn(() =>
      Promise.resolve({ password: "generated-pw-123", entropyBits: 128 })
    ),
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: vi.fn(() => Promise.resolve(true)),
}));

vi.mock("sonner", () => ({
  toast: mockToast,
}));

// Mock PasswordStrengthIndicator to avoid zxcvbn setup
vi.mock("@/components/ui/password-strength-indicator", () => ({
  PasswordStrengthIndicator: () => null,
}));

const mockEntry: Entry = {
  id: "entry-1",
  groupId: "group-1",
  title: "Test Entry",
  username: "user@example.com",
  url: "https://example.com",
  notes: "Some notes",
  iconId: 0,
  customIconUuid: null,
  tags: ["work"],
  customFields: { "Custom Key": "custom value" },
  customFieldMeta: [{ key: "Custom Key", isProtected: false }],
  createdAt: "2024-02-17T15:56:34Z",
  modifiedAt: "2024-02-17T15:58:43Z",
  accessedAt: "2024-02-17T15:58:43Z",
  expires: false,
  attachments: [],
};

const mockEntryTwo: Entry = {
  id: "entry-2",
  groupId: "group-1",
  title: "Second Entry",
  username: "second@example.com",
  url: "https://second.example.com",
  notes: "Second notes",
  iconId: 1,
  customIconUuid: null,
  tags: ["personal"],
  customFields: { "API Key": "public" },
  customFieldMeta: [{ key: "API Key", isProtected: false }],
  createdAt: "2024-02-17T15:56:34Z",
  modifiedAt: "2024-02-17T15:58:43Z",
  accessedAt: "2024-02-17T15:58:43Z",
  expires: false,
  attachments: [],
};

const mockEntryWithCustomIcon: Entry = {
  ...mockEntry,
  id: "entry-3",
  customIconUuid: "abc123",
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  function TestWrapper({ children }: Readonly<{ children: React.ReactNode }>) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  }
  return TestWrapper;
}

describe("EntryEditForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAutoDownloadFavicons.value = false;
  });

  it("renders empty form in create mode", () => {
    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    expect(
      screen.getByPlaceholderText("entries.form.titlePlaceholder")
    ).toHaveValue("");
    expect(
      screen.getByPlaceholderText("entries.form.usernamePlaceholder")
    ).toHaveValue("");
    expect(screen.getByText("entries.form.createEntry")).toBeInTheDocument();
  });

  it("renders pre-filled form in edit mode", async () => {
    render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    // Wait for secrets to load
    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("entries.form.titlePlaceholder")
      ).toHaveValue("Test Entry");
    });

    expect(
      screen.getByPlaceholderText("entries.form.usernamePlaceholder")
    ).toHaveValue("user@example.com");
    expect(screen.getByText("entries.form.saveChanges")).toBeInTheDocument();
  });

  it("shows title validation error on empty submit", async () => {
    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.click(screen.getByText("entries.form.createEntry"));
    });

    await waitFor(() => {
      expect(screen.getByText("Title is required.")).toBeInTheDocument();
    });
  });

  it("calls createEntry on submit in create mode", async () => {
    const onSave = vi.fn();
    const mockResult = { ...mockEntry, id: "new-entry-1" };
    mockCreateEntry.mockResolvedValueOnce(mockResult);

    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={onSave}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        {
          target: { value: "New Entry" },
        }
      );
    });

    await act(async () => {
      fireEvent.click(screen.getByText("entries.form.createEntry"));
    });

    await waitFor(() => {
      expect(mockCreateEntry).toHaveBeenCalledWith(
        expect.objectContaining({
          dbId: "db-1",
          groupId: "group-1",
          data: expect.objectContaining({
            title: "New Entry",
            username: "",
            password: "generated-pw-123",
          }),
        })
      );
      expect(onSave).toHaveBeenCalledWith(mockResult);
    });
  });

  it("calls updateEntry on submit in edit mode", async () => {
    const onSave = vi.fn();
    const mockResult = { ...mockEntry, title: "Updated Title" };
    mockUpdateEntry.mockResolvedValueOnce(mockResult);

    render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={onSave}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    // Wait for secrets to load
    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("entries.form.titlePlaceholder")
      ).toHaveValue("Test Entry");
    });

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        {
          target: { value: "Updated Title" },
        }
      );
    });

    await act(async () => {
      fireEvent.click(screen.getByText("entries.form.saveChanges"));
    });

    await waitFor(() => {
      expect(mockUpdateEntry).toHaveBeenCalledWith(
        expect.objectContaining({
          dbId: "db-1",
          id: "entry-1",
          data: expect.objectContaining({
            title: "Updated Title",
          }),
        })
      );
      expect(onSave).toHaveBeenCalledWith(mockResult);
    });
  });

  it("blocks save and supports retry when protected values fail to load", async () => {
    mockGetPassword.mockRejectedValue(new Error("secret load failed"));

    render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByText("entries.form.protectedValuesError")
      ).toBeInTheDocument();
    });

    expect(
      screen.getByRole("button", { name: "entries.form.saveChanges" })
    ).toBeDisabled();

    // Reset to succeed on retry
    mockGetPassword.mockResolvedValue("existing-password");

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "common.retry" }));
    });

    await waitFor(() => {
      expect(
        screen.queryByText("entries.form.protectedValuesError")
      ).not.toBeInTheDocument();
      expect(
        screen.getByRole("button", { name: "entries.form.saveChanges" })
      ).not.toBeDisabled();
    });
  });

  it("calls onCancel immediately when form is not dirty", async () => {
    const onCancel = vi.fn();

    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={onCancel}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.click(screen.getByText("common.cancel"));
    });

    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("cancel button calls onCancel", async () => {
    // Note: Testing the unsaved changes confirmation dialog (form.formState.isDirty)
    // is not reliable in jsdom because react-hook-form's Controller onChange
    // doesn't detect dirty state from synthetic DOM events. This is tested
    // manually in real browser instead. Here we verify the cancel flow works.
    const onCancel = vi.fn();

    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={onCancel}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.click(screen.getByText("common.cancel"));
    });

    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("toggles password visibility", async () => {
    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    const passwordInput = screen.getByPlaceholderText(
      "entries.form.passwordPlaceholder"
    );
    expect(passwordInput).toHaveAttribute("type", "password");

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "entries.form.showPassword" })
      );
    });

    expect(passwordInput).toHaveAttribute("type", "text");
  });

  it("renders and applies username suggestions from dropdown", async () => {
    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("entries.form.usernamePlaceholder")
      ).toBeInTheDocument();
    });

    const usernameInput = screen.getByPlaceholderText(
      "entries.form.usernamePlaceholder"
    );
    await act(async () => {
      fireEvent.focus(usernameInput);
      fireEvent.change(usernameInput, { target: { value: "ali" } });
    });

    expect(
      screen.getByRole("option", { name: "alice@example.com" })
    ).toBeInTheDocument();

    await act(async () => {
      fireEvent.keyDown(usernameInput, { key: "ArrowDown" });
      fireEvent.keyDown(usernameInput, { key: "Enter" });
    });

    expect(usernameInput).toHaveValue("alice@example.com");
  });

  it("resets non-secret fields when switching edited entry", async () => {
    const { rerender } = render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("entries.form.titlePlaceholder")
      ).toHaveValue("Test Entry");
    });

    rerender(
      <EntryEditForm
        entry={mockEntryTwo}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("entries.form.titlePlaceholder")
      ).toHaveValue("Second Entry");
      expect(
        screen.getByPlaceholderText("entries.form.usernamePlaceholder")
      ).toHaveValue("second@example.com");
      expect(
        screen.getByPlaceholderText("entries.form.urlPlaceholder")
      ).toHaveValue("https://second.example.com");
    });
  });

  it("keeps create form values when selected group changes", async () => {
    const { rerender } = render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        {
          target: { value: "Draft title" },
        }
      );
    });

    rerender(
      <EntryEditForm
        dbId="db-1"
        groupId="group-2"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />
    );

    expect(
      screen.getByPlaceholderText("entries.form.titlePlaceholder")
    ).toHaveValue("Draft title");
  });

  it("emits dirty state when form changes", async () => {
    const onDirtyChange = vi.fn();
    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
        onDirtyChange={onDirtyChange}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        {
          target: { value: "Dirty title" },
        }
      );
    });

    await waitFor(() => {
      expect(onDirtyChange).toHaveBeenCalledWith(true);
    });
  });

  it("fetches favicon from URL via manual action and syncs form icon state", async () => {
    mockFetchFavicon.mockResolvedValueOnce("updated");
    mockGetEntry.mockResolvedValueOnce({
      ...mockEntry,
      customIconUuid: "fetched-uuid",
    });

    render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "entries.form.fetchFavicon" })
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "entries.form.fetchFavicon" })
      );
    });

    await waitFor(() => {
      expect(mockFetchFavicon).toHaveBeenCalledWith("db-1", "entry-1", true);
      expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
      expect(mockGetEntry).toHaveBeenCalledWith("db-1", "entry-1");
    });
  });

  it("does not save the database when refetch returns unchanged", async () => {
    mockFetchFavicon.mockResolvedValueOnce("unchanged");

    render(
      <EntryEditForm
        entry={mockEntryWithCustomIcon}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "entries.form.refreshFavicon" })
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "entries.form.refreshFavicon" })
      );
    });

    await waitFor(() => {
      expect(mockFetchFavicon).toHaveBeenCalledWith("db-1", "entry-3", true);
    });
    expect(mockDatabaseSave).not.toHaveBeenCalled();
  });

  it("disables manual favicon fetch while URL edits are unsaved", async () => {
    render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "entries.form.fetchFavicon" })
      ).not.toBeDisabled();
    });

    fireEvent.change(
      screen.getByPlaceholderText("entries.form.urlPlaceholder"),
      {
        target: { value: "https://changed.example.com" },
      }
    );

    expect(
      screen.getByRole("button", { name: "entries.form.fetchFavicon" })
    ).toBeDisabled();
  });

  it("clears custom icon via manual action", async () => {
    mockClearCustomIcon.mockResolvedValueOnce(true);

    render(
      <EntryEditForm
        entry={mockEntryWithCustomIcon}
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "entries.form.clearCustomIcon" })
      ).not.toBeDisabled();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "entries.form.clearCustomIcon" })
      );
    });

    await waitFor(() => {
      expect(mockClearCustomIcon).toHaveBeenCalledWith("db-1", "entry-3");
      expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
    });
  });

  it("applies a picked custom icon when creating an entry", async () => {
    const created = { ...mockEntry, id: "entry-new", customIconUuid: null };
    mockCreateEntry.mockResolvedValueOnce(created);
    mockGetCustomIcons.mockResolvedValueOnce({
      "icon-uuid-1": { mimeType: "image/png", data: "AAA=" },
    });
    mockSetCustomIcon.mockResolvedValueOnce(true);

    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    // Wait for useCustomIcons to populate, then open the picker and click
    // the custom-icon tile.
    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "entries.form.chooseIcon" })
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "entries.form.chooseIcon" })
      );
    });

    await waitFor(() => {
      expect(
        screen.getByRole("button", {
          name: "iconPicker.customIconLabel",
        })
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "iconPicker.customIconLabel" })
      );
    });

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        { target: { value: "Brand new" } }
      );
      fireEvent.click(screen.getByText("entries.form.createEntry"));
    });

    await waitFor(() => {
      expect(mockCreateEntry).toHaveBeenCalled();
      expect(mockSetCustomIcon).toHaveBeenCalledWith(
        "db-1",
        "entry-new",
        "icon-uuid-1"
      );
      expect(mockDatabaseSave).toHaveBeenCalledWith("db-1");
    });
  });

  it("reports a successful create even when the custom-icon assignment fails", async () => {
    const created = { ...mockEntry, id: "entry-new", customIconUuid: null };
    mockCreateEntry.mockResolvedValueOnce(created);
    mockGetCustomIcons.mockResolvedValueOnce({
      "icon-uuid-1": { mimeType: "image/png", data: "AAA=" },
    });
    mockSetCustomIcon.mockRejectedValueOnce(new Error("uuid not found"));
    const onSave = vi.fn();

    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={onSave}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "entries.form.chooseIcon" })
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "entries.form.chooseIcon" })
      );
    });

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "iconPicker.customIconLabel" })
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(
        screen.getByRole("button", { name: "iconPicker.customIconLabel" })
      );
    });

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        { target: { value: "Brand new" } }
      );
      fireEvent.click(screen.getByText("entries.form.createEntry"));
    });

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith(created);
    });
    expect(mockToast.success).toHaveBeenCalledWith("entries.toast.created");
    expect(mockToast.error).toHaveBeenCalledWith(
      "entries.toast.customIconAssignFailed"
    );
    expect(mockToast.error).not.toHaveBeenCalledWith(
      "entries.toast.createFailed"
    );
  });

  it("auto-fetches favicon after creating via Save and create another", async () => {
    mockAutoDownloadFavicons.value = true;
    const created = {
      ...mockEntry,
      id: "entry-new",
      url: "https://newsite.example.com",
    };
    mockCreateEntry.mockResolvedValueOnce(created);
    mockFetchFavicon.mockResolvedValueOnce("updated");

    render(
      <EntryEditForm
        dbId="db-1"
        groupId="group-1"
        onSave={vi.fn()}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        { target: { value: "Brand new" } }
      );
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.urlPlaceholder"),
        { target: { value: "https://newsite.example.com" } }
      );
      fireEvent.click(screen.getByText("entries.form.saveAndNew"));
    });

    await waitFor(() => {
      expect(mockCreateEntry).toHaveBeenCalled();
      expect(mockFetchFavicon).toHaveBeenCalledWith("db-1", "entry-new", false);
    });
  });

  it("auto-fetches favicon after save when enabled", async () => {
    mockAutoDownloadFavicons.value = true;
    const onSave = vi.fn();
    const mockResult = { ...mockEntry, id: "entry-1", title: "Updated Title" };
    mockUpdateEntry.mockResolvedValueOnce(mockResult);
    mockFetchFavicon.mockResolvedValueOnce("unchanged");

    render(
      <EntryEditForm
        entry={mockEntry}
        dbId="db-1"
        groupId="group-1"
        onSave={onSave}
        onCancel={vi.fn()}
      />,
      { wrapper: createWrapper() }
    );

    await waitFor(() => {
      expect(
        screen.getByPlaceholderText("entries.form.titlePlaceholder")
      ).toHaveValue("Test Entry");
    });

    await act(async () => {
      fireEvent.change(
        screen.getByPlaceholderText("entries.form.titlePlaceholder"),
        {
          target: { value: "Updated Title" },
        }
      );
      fireEvent.click(screen.getByText("entries.form.saveChanges"));
    });

    await waitFor(() => {
      expect(onSave).toHaveBeenCalledWith(mockResult);
      expect(mockFetchFavicon).toHaveBeenCalledWith("db-1", "entry-1", false);
    });
  });

  describe("expiry", () => {
    const mockEntryWithExpiry: Entry = {
      ...mockEntry,
      id: "entry-expiry",
      expires: true,
      expiryTime: "2027-06-15T10:30:00.000Z",
    };

    it("renders the expiry checkbox after the password field, off by default", () => {
      render(
        <EntryEditForm
          dbId="db-1"
          groupId="group-1"
          onSave={vi.fn()}
          onCancel={vi.fn()}
        />,
        { wrapper: createWrapper() }
      );

      const checkbox = screen.getByRole("checkbox", {
        name: "entries.form.expiry.label",
      });
      expect(checkbox).toBeInTheDocument();
      expect(checkbox).toHaveAttribute("data-state", "unchecked");

      const password = screen.getByPlaceholderText(
        "entries.form.passwordPlaceholder"
      );
      expect(
        password.compareDocumentPosition(checkbox) &
          Node.DOCUMENT_POSITION_FOLLOWING
      ).toBeTruthy();
    });

    it("hides the preset dropdown and picker until the checkbox is ticked", () => {
      const { container } = render(
        <EntryEditForm
          dbId="db-1"
          groupId="group-1"
          onSave={vi.fn()}
          onCancel={vi.fn()}
        />,
        { wrapper: createWrapper() }
      );

      expect(
        container.querySelector('[data-slot="date-time-picker-trigger"]')
      ).not.toBeInTheDocument();

      fireEvent.click(
        screen.getByRole("checkbox", { name: "entries.form.expiry.label" })
      );

      const trigger = container.querySelector(
        '[data-slot="date-time-picker-trigger"]'
      );
      expect(trigger).toBeInTheDocument();
      // First tick pre-selects "1 year", so the picker shows a value rather
      // than its empty placeholder.
      expect(trigger).not.toHaveTextContent(
        "entries.form.expiry.pickPlaceholder"
      );
    });

    it("does not send expiry on create when the checkbox is off", async () => {
      const onSave = vi.fn();
      mockCreateEntry.mockResolvedValueOnce({ ...mockEntry, id: "new-1" });

      render(
        <EntryEditForm
          dbId="db-1"
          groupId="group-1"
          onSave={onSave}
          onCancel={vi.fn()}
        />,
        { wrapper: createWrapper() }
      );

      await act(async () => {
        fireEvent.change(
          screen.getByPlaceholderText("entries.form.titlePlaceholder"),
          { target: { value: "No Expiry" } }
        );
      });
      await act(async () => {
        fireEvent.click(screen.getByText("entries.form.createEntry"));
      });

      await waitFor(() => {
        expect(mockCreateEntry).toHaveBeenCalledWith(
          expect.objectContaining({
            data: expect.objectContaining({
              expires: false,
              expiryTime: undefined,
            }),
          })
        );
      });
    });

    it("loads an existing entry's expiry and persists it on update", async () => {
      const onSave = vi.fn();
      mockUpdateEntry.mockResolvedValueOnce(mockEntryWithExpiry);

      render(
        <EntryEditForm
          entry={mockEntryWithExpiry}
          dbId="db-1"
          groupId="group-1"
          onSave={onSave}
          onCancel={vi.fn()}
        />,
        { wrapper: createWrapper() }
      );

      await waitFor(() => {
        expect(
          screen.getByPlaceholderText("entries.form.titlePlaceholder")
        ).toHaveValue("Test Entry");
      });

      expect(
        screen.getByRole("checkbox", { name: "entries.form.expiry.label" })
      ).toHaveAttribute("data-state", "checked");

      await act(async () => {
        fireEvent.click(screen.getByText("entries.form.saveChanges"));
      });

      await waitFor(() => {
        expect(mockUpdateEntry).toHaveBeenCalledWith(
          expect.objectContaining({
            id: "entry-expiry",
            data: expect.objectContaining({
              expires: true,
              expiryTime: "2027-06-15T10:30:00.000Z",
            }),
          })
        );
      });
    });
  });
});
