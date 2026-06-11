// SPDX-License-Identifier: MIT

/**
 * Largest Attachment, in bytes, that the in-app preview will render. Files
 * above this skip rendering and show a "too large to preview — download to
 * view" message instead. Chosen so that base64-encoding a payload over the
 * IPC boundary and holding the resulting `data:` URL in a single DOM node
 * stays comfortably cheap on every supported desktop platform.
 */
export const PREVIEW_MAX_BYTES = 2 * 1024 * 1024;

/**
 * MIME types the preview renders as inline images via a `data:` URL. SVG is
 * deliberately excluded — it is technically previewable in a browser but
 * executes script in its own context, so it is treated as Download-only.
 */
const IMAGE_MIME_TYPES = new Set<string>([
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
  "image/bmp",
]);

/**
 * Filename extensions (lower-case, no leading dot) the preview renders as
 * raw monospace text. Matched on extension rather than MIME because real
 * KDBX attachments often arrive with `application/octet-stream` even when
 * the file is plainly textual.
 */
const TEXT_EXTENSIONS = new Set<string>([
  "txt",
  "md",
  "csv",
  "json",
  "xml",
  "yaml",
  "yml",
  "ini",
  "log",
  "conf",
]);

function extensionOf(filename: string): string {
  const dot = filename.lastIndexOf(".");
  if (dot < 0 || dot === filename.length - 1) return "";
  return filename.slice(dot + 1).toLowerCase();
}

/**
 * Result of classifying an Attachment for preview. A discriminated union so
 * the caller can pattern-match without checking flags:
 *
 * - `image` / `text`: previewable and within the size cap — open the modal
 *   and render via the matching path.
 * - `too-large`: the type would be previewable, but the bytes exceed
 *   [`PREVIEW_MAX_BYTES`]. The Preview button is still offered, but the
 *   modal shows a "too large to preview — download to view" message.
 * - `none`: the Attachment is not previewable (SVG, PDF, archives, office
 *   docs, opaque binaries). The Preview button is hidden entirely.
 */
export type AttachmentPreviewKind =
  | { kind: "image" }
  | { kind: "text" }
  | { kind: "too-large" }
  | { kind: "none" };

/**
 * Classify an Attachment for in-app preview from its metadata. Pure: a given
 * `(filename, mimeType, size)` always yields the same kind.
 */
export function classifyAttachment(meta: {
  filename: string;
  mimeType: string;
  size: number;
}): AttachmentPreviewKind {
  const previewable: "image" | "text" | null = IMAGE_MIME_TYPES.has(
    meta.mimeType
  )
    ? "image"
    : TEXT_EXTENSIONS.has(extensionOf(meta.filename))
      ? "text"
      : null;
  if (previewable === null) return { kind: "none" };
  if (meta.size > PREVIEW_MAX_BYTES) return { kind: "too-large" };
  return { kind: previewable };
}
