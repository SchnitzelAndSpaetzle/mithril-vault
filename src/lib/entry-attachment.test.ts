// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import { formatAttachmentSize } from "@/lib/entry-attachment";

describe("formatAttachmentSize", () => {
  it("renders sub-kilobyte sizes as whole bytes", () => {
    expect(formatAttachmentSize(0)).toBe("0 B");
    expect(formatAttachmentSize(11)).toBe("11 B");
    expect(formatAttachmentSize(999)).toBe("999 B");
  });

  it("switches to decimal kilobytes at 1000 bytes", () => {
    // Decimal (1000) units, matching macOS Finder and native save dialogs.
    expect(formatAttachmentSize(1000)).toBe("1 KB");
    expect(formatAttachmentSize(1500)).toBe("1.5 KB");
  });

  it("keeps at most one decimal place and drops a trailing .0", () => {
    expect(formatAttachmentSize(2048)).toBe("2 KB");
  });

  it("scales up through megabytes and gigabytes", () => {
    expect(formatAttachmentSize(1_500_000)).toBe("1.5 MB");
    expect(formatAttachmentSize(1_500_000_000)).toBe("1.5 GB");
  });
});
