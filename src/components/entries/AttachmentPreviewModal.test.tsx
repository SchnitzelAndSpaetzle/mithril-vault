// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import { AttachmentPreviewModal } from "./AttachmentPreviewModal";

const noop = () => undefined;

// Three bytes that, base64-encoded, produce a predictable suffix we can match
// on. UTF-8 they decode to "abc".
const SAMPLE_BYTES = new Uint8Array([0x61, 0x62, 0x63]);

function utf8(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

// Match the literal textContent against a <pre>, bypassing the library's
// default whitespace normalization so we can assert on multi-line input.
function findPreWithText(text: string) {
  return screen.findByText((_, el) => {
    return el?.tagName === "PRE" && el.textContent === text;
  });
}

describe("AttachmentPreviewModal — text", () => {
  it("renders the fetched bytes as decoded UTF-8 text in a <pre>", async () => {
    const contents = "line one\nline two — π";
    const fetchBytes = vi.fn().mockResolvedValue(utf8(contents));

    render(
      <AttachmentPreviewModal
        open={true}
        onOpenChange={noop}
        attachment={{
          filename: "notes.txt",
          mimeType: "application/octet-stream",
          size: utf8(contents).length,
        }}
        fetchBytes={fetchBytes}
      />
    );

    const pre = await findPreWithText(contents);
    expect(pre).toBeInTheDocument();
    expect(fetchBytes).toHaveBeenCalledWith("notes.txt");
  });

  it("preserves the literal text without rendering markdown or syntax", async () => {
    // Spec calls this out explicitly: raw monospace text, no markdown.
    const md = "# Heading\n**not bold**";
    const fetchBytes = vi.fn().mockResolvedValue(utf8(md));

    render(
      <AttachmentPreviewModal
        open={true}
        onOpenChange={noop}
        attachment={{
          filename: "README.md",
          mimeType: "application/octet-stream",
          size: utf8(md).length,
        }}
        fetchBytes={fetchBytes}
      />
    );

    // Render preserves literal characters verbatim — no <h1>, no <strong>.
    const pre = await findPreWithText(md);
    expect(pre.querySelector("h1")).toBeNull();
    expect(pre.querySelector("strong")).toBeNull();
  });
});

describe("AttachmentPreviewModal — too-large", () => {
  it("shows the too-large message and never fetches bytes", async () => {
    const fetchBytes = vi.fn();
    const TWO_MB_PLUS_ONE = 2 * 1024 * 1024 + 1;

    render(
      <AttachmentPreviewModal
        open={true}
        onOpenChange={noop}
        attachment={{
          filename: "huge.png",
          mimeType: "image/png",
          size: TWO_MB_PLUS_ONE,
        }}
        fetchBytes={fetchBytes}
      />
    );

    expect(
      await screen.findByText("entries.detail.attachmentPreviewTooLarge")
    ).toBeInTheDocument();
    // Avoid paying for the byte fetch we are about to refuse to render.
    expect(fetchBytes).not.toHaveBeenCalled();
  });
});

describe("AttachmentPreviewModal — image", () => {
  it("renders the fetched bytes as a base64 data URL using the attachment MIME type", async () => {
    const fetchBytes = vi.fn().mockResolvedValue(SAMPLE_BYTES);

    render(
      <AttachmentPreviewModal
        open={true}
        onOpenChange={noop}
        attachment={{ filename: "logo.png", mimeType: "image/png", size: 3 }}
        fetchBytes={fetchBytes}
      />
    );

    const img = await screen.findByRole("img", { name: "logo.png" });
    expect(img.getAttribute("src")).toBe("data:image/png;base64,YWJj");
    expect(fetchBytes).toHaveBeenCalledWith("logo.png");
  });
});
