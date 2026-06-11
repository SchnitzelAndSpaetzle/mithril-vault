// SPDX-License-Identifier: MIT

import { describe, expect, it } from "vitest";
import {
  classifyAttachment,
  PREVIEW_MAX_BYTES,
} from "@/lib/attachment-preview";

const KB = 1024;

describe("classifyAttachment", () => {
  it("classifies a small PNG as a previewable image", () => {
    expect(
      classifyAttachment({
        filename: "logo.png",
        mimeType: "image/png",
        size: 10 * KB,
      })
    ).toEqual({ kind: "image" });
  });
});

describe("classifyAttachment — images", () => {
  it.each<string>([
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/bmp",
  ])("treats %s under the cap as a previewable image", (mimeType) => {
    expect(
      classifyAttachment({
        filename: "pic",
        mimeType,
        size: 1 * KB,
      })
    ).toEqual({ kind: "image" });
  });
});

describe("classifyAttachment — explicitly non-previewable types", () => {
  it.each<[string, string]>([
    // SVG is browser-renderable but executes script in its own context, so
    // it is treated as Download-only by the issue spec.
    ["icon.svg", "image/svg+xml"],
    ["report.pdf", "application/pdf"],
    ["archive.zip", "application/zip"],
    ["doc.docx", "application/octet-stream"],
    ["blob.bin", "application/octet-stream"],
  ])(
    "classifies %s with %s as none (no preview button)",
    (filename, mimeType) => {
      expect(classifyAttachment({ filename, mimeType, size: 1 * KB })).toEqual({
        kind: "none",
      });
    }
  );
});

describe("classifyAttachment — text", () => {
  it("classifies a small .txt file as previewable text", () => {
    expect(
      classifyAttachment({
        filename: "notes.txt",
        mimeType: "application/octet-stream",
        size: 50,
      })
    ).toEqual({ kind: "text" });
  });

  it.each<string>([
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
  ])("treats .%s under the cap as previewable text", (ext) => {
    expect(
      classifyAttachment({
        filename: `file.${ext}`,
        mimeType: "application/octet-stream",
        size: 1 * KB,
      })
    ).toEqual({ kind: "text" });
  });

  it("matches the extension case-insensitively", () => {
    // Real-world filenames carry mixed case (e.g. README.MD on Windows).
    expect(
      classifyAttachment({
        filename: "README.MD",
        mimeType: "application/octet-stream",
        size: 1 * KB,
      })
    ).toEqual({ kind: "text" });
  });

  it("falls back to none when there is no extension at all", () => {
    expect(
      classifyAttachment({
        filename: "Makefile",
        mimeType: "application/octet-stream",
        size: 1 * KB,
      })
    ).toEqual({ kind: "none" });
  });

  it("falls back to none for an unknown text-ish extension", () => {
    // The allowlist is exhaustive by design — adding new extensions is a
    // conscious decision, not a heuristic that lights up on rs/py/sh.
    expect(
      classifyAttachment({
        filename: "main.rs",
        mimeType: "text/plain",
        size: 1 * KB,
      })
    ).toEqual({ kind: "none" });
  });
});

describe("classifyAttachment — size cap", () => {
  it("classifies a previewable image above 2 MB as too-large", () => {
    expect(
      classifyAttachment({
        filename: "huge.png",
        mimeType: "image/png",
        size: PREVIEW_MAX_BYTES + 1,
      })
    ).toEqual({ kind: "too-large" });
  });

  it("classifies a previewable text file above 2 MB as too-large", () => {
    expect(
      classifyAttachment({
        filename: "huge.log",
        mimeType: "application/octet-stream",
        size: PREVIEW_MAX_BYTES + 1,
      })
    ).toEqual({ kind: "too-large" });
  });

  it("treats a file exactly at the cap as previewable, not too-large", () => {
    expect(
      classifyAttachment({
        filename: "edge.png",
        mimeType: "image/png",
        size: PREVIEW_MAX_BYTES,
      })
    ).toEqual({ kind: "image" });
  });

  it("does not flag non-previewable types as too-large even when huge", () => {
    // A 5 MB PDF would skip preview anyway; the size cap is a refinement of
    // the previewable kinds, not a separate gate.
    expect(
      classifyAttachment({
        filename: "scan.pdf",
        mimeType: "application/pdf",
        size: 5 * 1024 * 1024,
      })
    ).toEqual({ kind: "none" });
  });
});

describe("PREVIEW_MAX_BYTES", () => {
  it("caps preview at ~2 MB", () => {
    expect(PREVIEW_MAX_BYTES).toBe(2 * 1024 * 1024);
  });
});
