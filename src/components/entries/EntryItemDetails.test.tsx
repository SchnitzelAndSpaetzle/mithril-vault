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
const preparePickedAttachments = vi.hoisted(() => vi.fn());
const prepareDroppedAttachments = vi.hoisted(() => vi.fn());
const commitPreparedAttachments = vi.hoisted(() => vi.fn());
const getAttachmentBytes = vi.hoisted(() => vi.fn());
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
    preparePickedAttachments,
    prepareDroppedAttachments,
    commitPreparedAttachments,
    getAttachmentBytes,
  },
  database: { save: databaseSave },
}));

// A plan with no over-soft files: the add proceeds without a confirmation
// prompt. `items` is non-empty so the flow reaches commit (an empty plan is a
// no-op). Tests that don't care about the exact items reuse this.
function planWithoutConfirmation() {
  return {
    items: [{ sourceName: "file.txt", size: 10, status: "ok" as const }],
    requiresConfirmation: false,
  };
}

// A plan flagged as requiring confirmation (at least one over-soft file), used
// by the soft-warning tests.
function planRequiringConfirmation() {
  return {
    items: [
      { sourceName: "big.pdf", size: 8_000_000, status: "overSoft" as const },
    ],
    requiresConfirmation: true,
  };
}

// Mutable holder for the responsive breakpoint so each test can render the
// desktop (drop zone present) or mobile (drop zone hidden) layout.
const mobile = vi.hoisted(() => ({ isMobile: false }));
vi.mock("@/hooks/use-mobile", () => ({
  useIsMobile: () => mobile.isMobile,
}));

// Captures the handler the detail panel registers for the native
// `tauri://drag-drop` event so tests can fire synthetic drop events at it.
const dragDrop = vi.hoisted(() => ({
  handler: null as ((event: { payload: unknown }) => void) | null,
}));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: (handler: (event: { payload: unknown }) => void) => {
      dragDrop.handler = handler;
      return Promise.resolve(() => {
        dragDrop.handler = null;
      });
    },
  }),
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

describe("EntryItemDetails attachment preview", () => {
  beforeEach(() => {
    state.entry = null;
    state.isTransitioning = false;
    getAttachmentBytes.mockReset();
  });

  it("renders a Preview button for an image attachment", () => {
    renderDetails({
      attachments: [{ filename: "logo.png", size: 100, mimeType: "image/png" }],
    });

    expect(
      screen.getByRole("button", { name: "entries.detail.previewAttachment" })
    ).toBeInTheDocument();
  });

  it("hides the Preview button for non-previewable types (PDF)", () => {
    renderWithPdf();

    expect(
      screen.queryByRole("button", { name: "entries.detail.previewAttachment" })
    ).not.toBeInTheDocument();
    // Download remains available for non-previewable types — that's the
    // whole point of the gating.
    expect(
      screen.getByRole("button", { name: "entries.detail.downloadAttachment" })
    ).toBeInTheDocument();
  });

  it("hides the Preview button for SVG (browser-renderable but not previewable per spec)", () => {
    renderDetails({
      attachments: [
        { filename: "icon.svg", size: 200, mimeType: "image/svg+xml" },
      ],
    });

    expect(
      screen.queryByRole("button", { name: "entries.detail.previewAttachment" })
    ).not.toBeInTheDocument();
  });

  it("shows the Preview button for a text attachment (.txt)", () => {
    renderDetails({
      attachments: [
        {
          filename: "notes.txt",
          size: 50,
          mimeType: "application/octet-stream",
        },
      ],
    });

    expect(
      screen.getByRole("button", { name: "entries.detail.previewAttachment" })
    ).toBeInTheDocument();
  });

  it("opens the modal and fetches bytes when Preview is clicked on an image", async () => {
    getAttachmentBytes.mockResolvedValue(
      new Uint8Array([0x61, 0x62, 0x63]) // "abc" → base64 "YWJj"
    );

    renderDetails({
      attachments: [{ filename: "logo.png", size: 3, mimeType: "image/png" }],
    });

    fireEvent.click(
      screen.getByRole("button", { name: "entries.detail.previewAttachment" })
    );

    const img = await screen.findByRole("img", { name: "logo.png" });
    expect(img.getAttribute("src")).toBe("data:image/png;base64,YWJj");
    expect(getAttachmentBytes).toHaveBeenCalledWith(
      "db-1",
      "entry-1",
      "logo.png"
    );
  });

  it("opens the modal and fetches bytes when Preview is clicked on a text file", async () => {
    const contents = "hello world";
    getAttachmentBytes.mockResolvedValue(new TextEncoder().encode(contents));

    renderDetails({
      attachments: [
        {
          filename: "notes.txt",
          size: contents.length,
          mimeType: "application/octet-stream",
        },
      ],
    });

    fireEvent.click(
      screen.getByRole("button", { name: "entries.detail.previewAttachment" })
    );

    await waitFor(() => {
      expect(getAttachmentBytes).toHaveBeenCalledWith(
        "db-1",
        "entry-1",
        "notes.txt"
      );
    });
    expect(await screen.findByText(contents)).toBeInTheDocument();
  });

  it("shows the Preview button for a previewable file above the size cap (modal explains it)", () => {
    // Per the answered spec question: Preview button is still offered; the
    // modal carries the "too large to preview — download to view" message.
    const TWO_MB_PLUS_ONE = 2 * 1024 * 1024 + 1;
    renderDetails({
      attachments: [
        { filename: "huge.png", size: TWO_MB_PLUS_ONE, mimeType: "image/png" },
      ],
    });

    expect(
      screen.getByRole("button", { name: "entries.detail.previewAttachment" })
    ).toBeInTheDocument();
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
    preparePickedAttachments.mockReset();
    commitPreparedAttachments.mockReset();
    ask.mockReset();
    databaseSave.mockReset();
    toastSuccess.mockReset();
    toastError.mockReset();
  });

  it("prepares via the Rust-side picker, commits with no path, then persists", async () => {
    // The two-phase flow: prepare opens the dialog and buffers paths in Rust,
    // returning a size plan; commit stores them. The frontend hands the commit
    // only db/entry ids — never a path — so a fabricated path can't reach the
    // read (ADR-0004 trust boundary). With no over-soft file there is no prompt;
    // on a non-empty outcome it persists once and reports success.
    preparePickedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockResolvedValue({
      added: ["codes.txt", "scan.pdf"],
      failed: [],
    });
    databaseSave.mockResolvedValue(undefined);

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(databaseSave).toHaveBeenCalledWith("db-1");
    });
    // An under-soft batch is never prompted, and commit receives ids only.
    expect(ask).not.toHaveBeenCalled();
    expect(commitPreparedAttachments).toHaveBeenCalledWith("db-1", "entry-1");
    expect(toastSuccess).toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("ignores a second Add click while the first gesture is still in flight", async () => {
    // Both write paths buffer paths in one shared Rust-side buffer, so a
    // concurrent gesture started before the first commits would clobber the
    // first's paths. The in-flight guard must reject the second prepare until
    // the first flow finishes.
    let resolvePrepare!: (plan: {
      items: { sourceName: string; size: number; status: "ok" }[];
      requiresConfirmation: boolean;
    }) => void;
    preparePickedAttachments.mockReturnValue(
      new Promise((resolve) => {
        resolvePrepare = resolve;
      })
    );

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");
    // Second click lands while the first prepare is still pending.
    clickAttachmentAction("entries.detail.addAttachment");

    expect(preparePickedAttachments).toHaveBeenCalledTimes(1);

    // Let the first flow finish so the guard releases (and the test doesn't
    // leak a pending promise).
    resolvePrepare({ items: [], requiresConfirmation: false });
    await waitFor(() => {
      expect(preparePickedAttachments).toHaveBeenCalledTimes(1);
    });
  });

  it("does nothing when the picker comes back empty (cancelled)", async () => {
    // A cancelled dialog yields an empty plan — the flow never reaches commit.
    preparePickedAttachments.mockResolvedValue({
      items: [],
      requiresConfirmation: false,
    });

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(preparePickedAttachments).toHaveBeenCalled();
    });
    expect(commitPreparedAttachments).not.toHaveBeenCalled();
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
  });

  it("prompts for confirmation when a file is over the soft threshold and commits on confirm", async () => {
    // AC: adding a file above the soft threshold prompts; confirming proceeds.
    preparePickedAttachments.mockResolvedValue(planRequiringConfirmation());
    ask.mockResolvedValue(true);
    commitPreparedAttachments.mockResolvedValue({
      added: ["big.pdf"],
      failed: [],
    });
    databaseSave.mockResolvedValue(undefined);

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(commitPreparedAttachments).toHaveBeenCalledWith("db-1", "entry-1");
    });
    expect(ask).toHaveBeenCalledTimes(1);
    expect(databaseSave).toHaveBeenCalledWith("db-1");
    expect(toastSuccess).toHaveBeenCalled();
  });

  it("aborts the whole batch when the soft-warning is declined", async () => {
    // AC: cancelling aborts. Nothing is committed, persisted, or reported.
    preparePickedAttachments.mockResolvedValue(planRequiringConfirmation());
    ask.mockResolvedValue(false);

    renderDetails({ attachments: [] });
    clickAttachmentAction("entries.detail.addAttachment");

    await waitFor(() => {
      expect(ask).toHaveBeenCalledTimes(1);
    });
    expect(commitPreparedAttachments).not.toHaveBeenCalled();
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
    expect(preparePickedAttachments).not.toHaveBeenCalled();
  });

  it("surfaces a batch error toast when the commit itself rejects", async () => {
    preparePickedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockRejectedValue(new Error("vault locked"));

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
    preparePickedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockResolvedValue({
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
    preparePickedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockResolvedValue({
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

// The native event reports a physical position; the panel is mocked to a
// 100x100 box at the origin so a position inside (50,50) vs. outside (500,500)
// exercises the scoping hit-test (devicePixelRatio defaults to 1 in jsdom).
function mockPanelRect() {
  return vi
    .spyOn(HTMLElement.prototype, "getBoundingClientRect")
    .mockReturnValue({
      left: 0,
      top: 0,
      right: 100,
      bottom: 100,
      width: 100,
      height: 100,
      x: 0,
      y: 0,
      toJSON: () => ({}),
    });
}

async function fireDrop(position: { x: number; y: number }) {
  await act(async () => {
    dragDrop.handler?.({ payload: { type: "drop", position } });
  });
}

describe("EntryItemDetails attachment drop zone", () => {
  beforeEach(() => {
    state.entry = null;
    state.isTransitioning = false;
    mobile.isMobile = false;
    dragDrop.handler = null;
    prepareDroppedAttachments.mockReset();
    commitPreparedAttachments.mockReset();
    ask.mockReset();
    databaseSave.mockReset();
    toastSuccess.mockReset();
    toastError.mockReset();
  });

  it("shows the drop-zone affordance on desktop alongside the picker button", () => {
    mobile.isMobile = false;
    renderDetails({ attachments: [] });

    expect(screen.getByText("entries.detail.dropToAttach")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "entries.detail.addAttachment" })
    ).toBeInTheDocument();
  });

  it("hides the drop zone on mobile but keeps the picker button", () => {
    // Mobile has no OS file-drop, so the affordance is gone; the picker button
    // remains the only way to add an attachment there.
    mobile.isMobile = true;
    renderDetails({ attachments: [] });

    expect(
      screen.queryByText("entries.detail.dropToAttach")
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "entries.detail.addAttachment" })
    ).toBeInTheDocument();
  });

  it("commits a drop that lands inside the panel via the shared add path", async () => {
    // A drop on the selected Entry's panel prepares (peeks) the Rust-side buffer
    // for its size plan, then commits (the command takes only ids — no path) and
    // goes through the exact same tail as the picker: persist once, refresh,
    // success toast.
    prepareDroppedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockResolvedValue({
      added: ["dropped.txt"],
      failed: [],
    });
    databaseSave.mockResolvedValue(undefined);
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    expect(dragDrop.handler).toBeTypeOf("function");

    await fireDrop({ x: 50, y: 50 });

    await waitFor(() => {
      expect(commitPreparedAttachments).toHaveBeenCalledWith("db-1", "entry-1");
    });
    expect(databaseSave).toHaveBeenCalledWith("db-1");
    expect(toastSuccess).toHaveBeenCalled();
    expect(toastError).not.toHaveBeenCalled();
    rect.mockRestore();
  });

  it("aborts a dropped batch when the soft-warning is declined", async () => {
    // The drop path shares the picker's prepare→confirm→commit flow: an
    // over-soft file prompts, and declining aborts the whole batch.
    prepareDroppedAttachments.mockResolvedValue(planRequiringConfirmation());
    ask.mockResolvedValue(false);
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    await fireDrop({ x: 50, y: 50 });

    await waitFor(() => {
      expect(ask).toHaveBeenCalledTimes(1);
    });
    expect(commitPreparedAttachments).not.toHaveBeenCalled();
    expect(databaseSave).not.toHaveBeenCalled();
    rect.mockRestore();
  });

  it("does not persist when a dropped batch wholly fails", async () => {
    // Same no-phantom-save guard as the picker: a fully-failed drop reports its
    // per-file failures but never persists or reports success.
    prepareDroppedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockResolvedValue({
      added: [],
      failed: [{ sourceName: "huge.bin", reason: "too large" }],
    });
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    await fireDrop({ x: 50, y: 50 });

    await waitFor(() => {
      expect(toastError).toHaveBeenCalledTimes(1);
    });
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
    rect.mockRestore();
  });

  it("ignores a drop that lands outside the panel", async () => {
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    await fireDrop({ x: 500, y: 500 });

    expect(prepareDroppedAttachments).not.toHaveBeenCalled();
    expect(databaseSave).not.toHaveBeenCalled();
    rect.mockRestore();
  });

  it("ignores a drop while the entry is transitioning", async () => {
    // A drop landing mid-transition could target a stale entry, so it no-ops.
    state.isTransitioning = true;
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    await fireDrop({ x: 50, y: 50 });

    expect(prepareDroppedAttachments).not.toHaveBeenCalled();
    rect.mockRestore();
  });

  it("registers no drop handler when no entry is selected", () => {
    // With no entry the detail panel renders a skeleton, so nothing subscribes
    // to the window-global drop event — drops are ignored structurally.
    state.entry = null;
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={queryClient}>
        <EntryItemDetails entryId="entry-1" dbId="db-1" />
      </QueryClientProvider>
    );

    expect(dragDrop.handler).toBeNull();
  });

  it("highlights the drop zone while a file is dragged over it", () => {
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    act(() => {
      dragDrop.handler?.({
        payload: { type: "over", position: { x: 50, y: 50 } },
      });
    });

    expect(screen.getByText("entries.detail.dropToAttach").className).toContain(
      "border-primary"
    );
    rect.mockRestore();
  });

  it("surfaces a batch error toast when a dropped commit rejects", async () => {
    prepareDroppedAttachments.mockResolvedValue(planWithoutConfirmation());
    commitPreparedAttachments.mockRejectedValue(new Error("vault locked"));
    const rect = mockPanelRect();

    renderDetails({ attachments: [] });
    await fireDrop({ x: 50, y: 50 });

    await waitFor(() => {
      expect(toastError).toHaveBeenCalled();
    });
    expect(databaseSave).not.toHaveBeenCalled();
    expect(toastSuccess).not.toHaveBeenCalled();
    rect.mockRestore();
  });
});
