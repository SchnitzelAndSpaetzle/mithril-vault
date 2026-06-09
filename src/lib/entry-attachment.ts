// SPDX-License-Identifier: MIT

/** Decimal (1000-based) unit ladder, matching macOS Finder and save dialogs. */
const SIZE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/**
 * Format an Attachment's byte size for display. Pure: a given byte count always
 * yields the same string.
 *
 * Uses decimal units (1 KB = 1000 B) so sizes line up with what the OS file
 * dialog reports. Bytes render as whole numbers; larger units keep at most one
 * decimal place with a trailing `.0` trimmed (e.g. `2 KB`, not `2.0 KB`).
 */
export function formatAttachmentSize(bytes: number): string {
  let size = bytes;
  let unit = 0;
  while (size >= 1000 && unit < SIZE_UNITS.length - 1) {
    size /= 1000;
    unit += 1;
  }

  // Bytes are always whole; scaled units round to a single decimal.
  const value =
    unit === 0
      ? size
      : Number.parseFloat((Math.round(size * 10) / 10).toFixed(1));

  return `${value} ${SIZE_UNITS[unit]}`;
}
