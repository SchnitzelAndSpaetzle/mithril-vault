// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import dayjs from "dayjs";

import EntryItemDetails from "./EntryItemDetails";
import type { Entry } from "@/lib/types";

const baseEntry: Entry = {
  id: "entry-1",
  groupId: "root",
  title: "GitHub",
  username: "octocat",
  url: null,
  notes: null,
  iconId: 0,
  customIconUuid: null,
  tags: [],
  customFields: {},
  customFieldMeta: [],
  createdAt: "2026-01-01T00:00:00Z",
  modifiedAt: "2026-01-02T00:00:00Z",
  accessedAt: "2026-01-02T00:00:00Z",
  expires: false,
  expiryTime: null,
  attachments: [],
};

// Mutable holder the mocked hook reads from. vi.hoisted runs before the
// vi.mock factories so they can close over it safely.
const state = vi.hoisted(() => ({
  entry: null as Entry | null,
  isTransitioning: false,
}));

vi.mock("@/hooks/use-entry-detail", () => ({
  useEntryDetail: () => ({
    entry: state.entry,
    isLoading: false,
    isError: false,
    password: null,
    isPasswordVisible: false,
    isPasswordLoading: false,
    isTransitioning: state.isTransitioning,
    revealPassword: vi.fn(),
    hidePassword: vi.fn(),
  }),
}));

vi.mock("@/hooks/use-custom-icons", () => ({
  useCustomIcons: () => ({ data: {} }),
}));

vi.mock("@/hooks/use-copy-to-clipboard", () => ({
  useCopyToClipboard: () => ({ copy: vi.fn(), isCopied: false }),
}));

vi.mock("@/hooks/use-clipboard-countdown", () => ({
  useClipboardCountdown: () => vi.fn(),
}));

vi.mock("@/hooks/use-clipboard-timeout", () => ({
  useClipboardTimeout: () => 30,
}));

const exportAttachment = vi.hoisted(() => vi.fn());
const deleteAttachment = vi.hoisted(() => vi.fn());
const addAttachments = vi.hoisted(() => vi.fn());
const databaseSave = vi.hoisted(() => vi.fn());
const save = vi.hoisted(() => vi.fn());
const ask = vi.hoisted(() => vi.fn());
const toastSuccess = vi.hoisted(() => vi.fn());
const toastError = vi.hoisted(() => vi.fn());

vi.mock("@/lib/tauri", () => ({
  clipboard: { copyPassword: vi.fn(), copyProtectedField: vi.fn() },
  entries: {
    getProtectedCustomField: vi.fn(),
    exportAttachment,
    deleteAttachment,
    addAttachments,
  },
  database: { save: databaseSave },
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

vi.mock("@tauri-apps/plugin-dialog", () => ({ save, ask }));

vi.mock("sonner", () => ({
  toast: { success: toastSuccess, error: toastError },
}));

function renderDetails(override: Partial<Entry>) {
  state.entry = { ...baseEntry, ...override };
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <EntryItemDetails entryId="entry-1" dbId="db-1" />
    </QueryClientProvider>
  );
}

// A single previewable-but-not-previewed attachment, the fixture the
// download and delete flows exercise their happy/error paths against.
const PDF_ATTACHMENT = {
  filename: "report.pdf",
  size: 2048,
  mimeType: "application/pdf",
} as const;

function renderWithPdf() {
  renderDetails({ attachments: [PDF_ATTACHMENT] });
}

function clickAttachmentAction(name: string) {
  fireEvent.click(screen.getByRole("button", { name }));
}

describe("EntryItemDetails expired indicator", () => {
  beforeEach(() => {
    state.entry = null;
  });

  it("strikes through the title and shows an Expired badge when expired", () => {
    renderDetails({ expires: true, expiryTime: "2000-01-01T00:00:00Z" });

    const title = screen.getByRole("heading", { name: "GitHub" });
    expect(title.className).toContain("line-through");
    expect(title.className).toContain("text-muted-foreground");
    expect(screen.getByText("entries.detail.expired")).toBeInTheDocument();
  });

  it("shows the absolute expiry date in local time alongside the metadata", () => {
    const expiryTime = "2000-06-15T12:00:00Z";
    renderDetails({ expires: true, expiryTime });

    expect(screen.getByText("entries.detail.expires")).toBeInTheDocument();
    // Rendered in the viewer's local time via dayjs — assert the local
    // calendar year appears so the row reflects the actual instant.
    const localYear = String(dayjs(expiryTime).year());
    expect(
      screen.getByText((text) => text.includes(localYear))
    ).toBeInTheDocument();
  });

  it("shows no badge or strikethrough for a non-expired Entry", () => {
    renderDetails({ expires: false });

    const title = screen.getByRole("heading", { name: "GitHub" });
    expect(title.className).not.toContain("line-through");
    expect(
      screen.queryByText("entries.detail.expired")
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText("entries.detail.expires")
    ).not.toBeInTheDocument();
  });
});

describe("EntryItemDetails attachments section", () => {
  beforeEach(() => {
    state.entry = null;
    state.isTransitioning = false;
    exportAttachment.mockReset();
    save.mockReset();
    toastSuccess.mockReset();
    toastError.mockReset();
  });

  it("renders a Download action for each attachment row", () => {
    renderDetails({
      attachments: [
        { filename: "a.txt", size: 1, mimeType: "text/plain" },
        { filename: "b.png", size: 2, mimeType: "image/png" },
      ],
    });

    const buttons = screen.getAllByRole("button", {
      name: "entries.detail.downloadAttachment",
    });
    expect(buttons).toHaveLength(2);
  });

  it("renders one row per attachment with its filename and human-readable size", () => {
    renderWithPdf();

    const row = screen.getByRole("listitem");
    expect(row).toHaveTextContent("report.pdf");
    // 2048 bytes formatted as decimal KB (2.048 → "2 KB").
    expect(row).toHaveTextContent("2 KB");
  });

  it("downloads via a save dialog with the filename pre-filled, then exports the bytes", async () => {
    save.mockResolvedValue("/home/user/Downloads/report.pdf");
    exportAttachment.mockResolvedValue(undefined);

    renderWithPdf();
    clickAttachmentAction("entries.detail.downloadAttachment");

    await waitFor(() => {
      expect(save).toHaveBeenCalledWith({ defaultPath: "report.pdf" });
    });
    expect(exportAttachment).toHaveBeenCalledWith(
      "db-1",
      "entry-1",
      "report.pdf",
      "/home/user/Downloads/report.pdf"
    );
    expect(toastSuccess).toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("disables Download and ignores clicks while the entry is transitioning", () => {
    state.isTransitioning = true;
    renderWithPdf();

    const button = screen.getByRole("button", {
      name: "entries.detail.downloadAttachment",
    });
    expect(button).toBeDisabled();

    fireEvent.click(button);
    expect(save).not.toHaveBeenCalled();
    expect(exportAttachment).not.toHaveBeenCalled();
  });

  it("does nothing when the save dialog is cancelled", async () => {
    save.mockResolvedValue(null);

    renderWithPdf();
    clickAttachmentAction("entries.detail.downloadAttachment");

    await waitFor(() => {
      expect(save).toHaveBeenCalled();
    });
    expect(exportAttachment).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("surfaces an error toast when the export fails", async () => {
    save.mockResolvedValue("/home/user/Downloads/report.pdf");
    exportAttachment.mockRejectedValue(new Error("disk full"));

    renderWithPdf();
    clickAttachmentAction("entries.detail.downloadAttachment");

    await waitFor(() => {
      expect(toastError).toHaveBeenCalled();
    });
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it("shows an empty state and no rows when there are no attachments", () => {
    renderDetails({ attachments: [] });

    expect(
      screen.getByText("entries.detail.noAttachments")
    ).toBeInTheDocument();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });

  it("sorts attachment rows by filename, case-insensitively", () => {
    renderDetails({
      attachments: [
        { filename: "Zebra.txt", size: 10, mimeType: "text/plain" },
        { filename: "apple.png", size: 20, mimeType: "image/png" },
        { filename: "Banana.gif", size: 30, mimeType: "image/gif" },
      ],
    });

    const filenames = screen
      .getAllByRole("listitem")
      .map((li) => li.textContent ?? "");

    expect(filenames[0]).toContain("apple.png");
    expect(filenames[1]).toContain("Banana.gif");
    expect(filenames[2]).toContain("Zebra.txt");
  });
});

describe("EntryItemDetails attachment delete", () => {
  beforeEach(() => {
    state.entry = null;
    state.isTransitioning = false;
    deleteAttachment.mockReset();
    databaseSave.mockReset();
    ask.mockReset();
    toastSuccess.mockReset();
    toastError.mockReset();
  });

  it("renders a Delete action for each attachment row", () => {
    renderDetails({
      attachments: [
        { filename: "a.txt", size: 1, mimeType: "text/plain" },
        { filename: "b.png", size: 2, mimeType: "image/png" },
      ],
    });

    const buttons = screen.getAllByRole("button", {
      name: "entries.detail.deleteAttachment",
    });
    expect(buttons).toHaveLength(2);
  });

  it("confirms, then deletes the attachment and persists on confirm", async () => {
    ask.mockResolvedValue(true);
    deleteAttachment.mockResolvedValue(undefined);
    databaseSave.mockResolvedValue(undefined);

    renderWithPdf();
    clickAttachmentAction("entries.detail.deleteAttachment");

    await waitFor(() => {
      expect(ask).toHaveBeenCalled();
    });
    expect(deleteAttachment).toHaveBeenCalledWith(
      "db-1",
      "entry-1",
      "report.pdf"
    );
    expect(databaseSave).toHaveBeenCalledWith("db-1");
    expect(toastSuccess).toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("leaves the attachment intact when the confirmation is cancelled", async () => {
    ask.mockResolvedValue(false);

    renderWithPdf();
    clickAttachmentAction("entries.detail.deleteAttachment");

    await waitFor(() => {
      expect(ask).toHaveBeenCalled();
    });
    expect(deleteAttachment).not.toHaveBeenCalled();
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it("disables Delete and ignores clicks while the entry is transitioning", () => {
    state.isTransitioning = true;
    renderWithPdf();

    const button = screen.getByRole("button", {
      name: "entries.detail.deleteAttachment",
    });
    expect(button).toBeDisabled();

    fireEvent.click(button);
    expect(ask).not.toHaveBeenCalled();
    expect(deleteAttachment).not.toHaveBeenCalled();
  });

  it("surfaces an error toast when the delete fails", async () => {
    ask.mockResolvedValue(true);
    deleteAttachment.mockRejectedValue(new Error("locked"));

    renderWithPdf();
    clickAttachmentAction("entries.detail.deleteAttachment");

    await waitFor(() => {
      expect(toastError).toHaveBeenCalled();
    });
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it("still completes the delete flow when persisting to disk fails", async () => {
    // The reference is already gone from the in-memory Vault once
    // deleteAttachment resolves, so a save failure must not abort the
    // success path — otherwise the row lingers and later actions target an
    // attachment that no longer exists. The save error is surfaced on its
    // own; the delete still reports success and the entry queries refresh.
    ask.mockResolvedValue(true);
    deleteAttachment.mockResolvedValue(undefined);
    databaseSave.mockRejectedValue(new Error("disk full"));

    renderWithPdf();
    clickAttachmentAction("entries.detail.deleteAttachment");

    // The delete success path still runs despite the save rejection.
    await waitFor(() => {
      expect(toastSuccess).toHaveBeenCalled();
    });
    expect(deleteAttachment).toHaveBeenCalledWith(
      "db-1",
      "entry-1",
      "report.pdf"
    );
    expect(databaseSave).toHaveBeenCalledWith("db-1");
  });
});

describe("EntryItemDetails attachment add", () => {
  beforeEach(() => {
    state.entry = null;
    state.isTransitioning = false;
    addAttachments.mockReset();
    databaseSave.mockReset();
    toastSuccess.mockReset();
    toastError.mockReset();
  });

  it("invokes the Rust-side picker command with no path, then persists", async () => {
    // The dialog now lives in Rust: the frontend hands the command only the
    // db/entry ids — never a path — so a fabricated path can't reach the read
    // (ADR-0004 trust boundary). On a non-empty outcome it persists once and
    // reports success.
    addAttachments.mockResolvedValue({
      added: ["codes.txt", "scan.pdf"],
      failed: [],
    });
    databaseSave.mockResolvedValue(undefined);

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(databaseSave).toHaveBeenCalledWith("db-1");
    });
    // The command receives ids only — no caller-supplied path argument.
    expect(addAttachments).toHaveBeenCalledWith("db-1", "entry-1");
    expect(toastSuccess).toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("does nothing when the picker comes back empty (cancelled)", async () => {
    addAttachments.mockResolvedValue({ added: [], failed: [] });

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(addAttachments).toHaveBeenCalled();
    });
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("disables Add and ignores clicks while the entry is transitioning", () => {
    state.isTransitioning = true;
    renderDetails({ attachments: [] });

    const button = screen.getByRole("button", {
      name: "entries.detail.addAttachment",
    });
    expect(button).toBeDisabled();

    fireEvent.click(button);
    expect(addAttachments).not.toHaveBeenCalled();
  });

  it("surfaces a batch error toast when the command itself rejects", async () => {
    addAttachments.mockRejectedValue(new Error("vault locked"));

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(toastError).toHaveBeenCalled();
    });
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it("raises one error toast per failed file rather than one for the batch", async () => {
    // Each per-file failure must get its own toast so the user can tell which
    // files failed (the string names the file + the backend reason); a single
    // collapsed toast would hide that.
    addAttachments.mockResolvedValue({
      added: [],
      failed: [
        { sourceName: "huge-a.bin", reason: "too large" },
        { sourceName: "huge-b.bin", reason: "too large" },
      ],
    });

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(toastError).toHaveBeenCalledTimes(2);
    });
    // A wholly-failed batch leaves no unsaved state and reports no success.
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
  });

  it("persists the survivors when some files in the batch failed", async () => {
    // A mixed outcome: the survivors still persist and report success, with
    // one error toast for the failure and one success toast for what landed.
    addAttachments.mockResolvedValue({
      added: ["ok-a.txt", "ok-b.txt"],
      failed: [{ sourceName: "huge.bin", reason: "too large" }],
    });
    databaseSave.mockResolvedValue(undefined);

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(databaseSave).toHaveBeenCalledWith("db-1");
    });
    expect(toastError).toHaveBeenCalledTimes(1);
    expect(toastSuccess).toHaveBeenCalledTimes(1);
  });
});
