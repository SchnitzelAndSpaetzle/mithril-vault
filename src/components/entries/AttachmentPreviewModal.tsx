// SPDX-License-Identifier: MIT

import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { AttachmentMeta } from "@/lib/types";
import { classifyAttachment } from "@/lib/attachment-preview";

interface AttachmentPreviewModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  attachment: AttachmentMeta;
  fetchBytes: (filename: string) => Promise<Uint8Array>;
}

function bytesToBase64(bytes: Uint8Array): string {
  // chunked binary string → btoa, the standard browser path. Chunked to keep
  // String.fromCharCode under its argument-limit on multi-MB payloads.
  const CHUNK = 0x8000;
  let binary = "";
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(
      ...bytes.subarray(i, Math.min(i + CHUNK, bytes.length))
    );
  }
  return btoa(binary);
}

export function AttachmentPreviewModal({
  open,
  onOpenChange,
  attachment,
  fetchBytes,
}: Readonly<AttachmentPreviewModalProps>) {
  const { t } = useTranslation();
  const kind = classifyAttachment(attachment).kind;
  const [bytes, setBytes] = useState<Uint8Array | null>(null);

  useEffect(() => {
    // Skip the byte fetch for kinds the modal will not render (too-large
    // shows a message; none is rendered as nothing). Saves an IPC round-trip
    // and keeps the in-memory copy of the bytes small.
    if (!open || kind === "too-large" || kind === "none") return;
    let cancelled = false;
    void fetchBytes(attachment.filename).then((result) => {
      if (!cancelled) setBytes(result);
    });
    return () => {
      cancelled = true;
    };
  }, [open, kind, attachment.filename, fetchBytes]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[80vh] overflow-auto">
        <DialogHeader>
          <DialogTitle>{attachment.filename}</DialogTitle>
        </DialogHeader>
        {kind === "too-large" ? (
          <p className="text-sm text-muted-foreground">
            {t("entries.detail.attachmentPreviewTooLarge")}
          </p>
        ) : kind === "image" && bytes ? (
          <img
            alt={attachment.filename}
            src={`data:${attachment.mimeType};base64,${bytesToBase64(bytes)}`}
          />
        ) : kind === "text" && bytes ? (
          <pre className="whitespace-pre-wrap break-words font-mono text-sm">
            {new TextDecoder("utf-8").decode(bytes)}
          </pre>
        ) : (
          <p>{t("entries.detail.attachmentPreviewLoading")}</p>
        )}
      </DialogContent>
    </Dialog>
  );
}
