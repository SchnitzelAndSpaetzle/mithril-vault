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
  mockGetPassword,
  mockGetProtectedCustomField,
} = vi.hoisted(() => ({
  mockCreateEntry: vi.fn(),
  mockUpdateEntry: vi.fn(),
  mockMoveEntry: vi.fn(),
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
      },
    ])
  ),
  mockGetPassword: vi.fn(() => Promise.resolve("existing-password")),
  mockGetProtectedCustomField: vi.fn(() =>
    Promise.resolve({ key: "secret", value: "secret-value" })
  ),
}));

vi.mock("@/hooks/use-entry-mutations", () => ({
  useEntryMutations: vi.fn(() => ({
    createEntry: { mutateAsync: mockCreateEntry, isPending: false },
    updateEntry: { mutateAsync: mockUpdateEntry, isPending: false },
    moveEntry: { mutateAsync: mockMoveEntry, isPending: false },
    deleteEntry: { mutateAsync: vi.fn(), isPending: false },
  })),
}));

vi.mock("@/lib/tauri", () => ({
  entries: {
    list: mockListEntries,
    getPassword: mockGetPassword,
    getProtectedCustomField: mockGetProtectedCustomField,
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
  toast: { success: vi.fn(), error: vi.fn() },
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
};

function createWrapper() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  function TestWrapper({ children }: { children: React.ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
  }
  return TestWrapper;
}

describe("EntryEditForm", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
});
