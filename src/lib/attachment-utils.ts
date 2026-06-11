// SPDX-License-Identifier: MIT

import type { Entry } from "@/lib/types";

/// True when the Entry carries at least one attachment. Drives both
/// the has-attachments list filter and the per-row paperclip glyph.
export function entryHasAttachments(entry: Entry): boolean {
  return entry.attachments.length > 0;
}
