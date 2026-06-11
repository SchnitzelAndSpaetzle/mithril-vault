// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { SettingsSection } from "@/components/settings/SettingsSection";
import { ATTACHMENT_BYTES_PER_MB, type AppPreferences } from "@/lib/types";

interface AttachmentsSettingsSectionProps {
  draft: AppPreferences;
  updateDraft: (updater: (previous: AppPreferences) => AppPreferences) => void;
}

// Lower bound on the MB inputs: 1 MB. The backend enforces the real invariant
// (hardCapBytes >= 1, softWarnBytes <= hardCapBytes); these attributes just keep
// the spinbutton from stepping into obviously useless values.
const MIN_MB = 1;

/// Converts a byte-valued threshold to the decimal MB shown in the input.
function bytesToMb(bytes: number): number {
  return Math.round(bytes / ATTACHMENT_BYTES_PER_MB);
}

export function AttachmentsSettingsSection({
  draft,
  updateDraft,
}: Readonly<AttachmentsSettingsSectionProps>) {
  const { t } = useTranslation();

  // Parses an MB input and writes it back to the draft as bytes via `apply`. A
  // blank or non-numeric value is ignored so a half-typed field never writes
  // NaN bytes into the draft.
  const onMbChange = (
    value: string,
    apply: (previous: AppPreferences, bytes: number) => AppPreferences
  ) => {
    const mb = Number.parseInt(value, 10);
    if (Number.isNaN(mb)) return;
    updateDraft((previous) => apply(previous, mb * ATTACHMENT_BYTES_PER_MB));
  };

  return (
    <SettingsSection
      id="attachments-settings"
      title={t("settings.attachments.title")}
      description={t("settings.attachments.description")}
    >
      <div className="flex flex-col gap-2 text-sm">
        <label htmlFor="attachment-soft-warn" className="flex flex-col gap-1">
          <span>{t("settings.attachments.softWarn.label")}</span>
          <span className="text-muted-foreground">
            {t("settings.attachments.softWarn.description")}
          </span>
        </label>
        <Input
          id="attachment-soft-warn"
          aria-label={t("settings.attachments.softWarn.label")}
          type="number"
          inputMode="numeric"
          min={MIN_MB}
          className="w-32"
          value={bytesToMb(draft.attachments.softWarnBytes)}
          onChange={(event) =>
            onMbChange(event.target.value, (previous, bytes) => ({
              ...previous,
              attachments: {
                ...previous.attachments,
                softWarnBytes: bytes,
              },
            }))
          }
        />
      </div>

      <div className="flex flex-col gap-2 text-sm">
        <label htmlFor="attachment-hard-cap" className="flex flex-col gap-1">
          <span>{t("settings.attachments.hardCap.label")}</span>
          <span className="text-muted-foreground">
            {t("settings.attachments.hardCap.description")}
          </span>
        </label>
        <Input
          id="attachment-hard-cap"
          aria-label={t("settings.attachments.hardCap.label")}
          type="number"
          inputMode="numeric"
          min={MIN_MB}
          className="w-32"
          value={bytesToMb(draft.attachments.hardCapBytes)}
          onChange={(event) =>
            onMbChange(event.target.value, (previous, bytes) => ({
              ...previous,
              attachments: {
                ...previous.attachments,
                hardCapBytes: bytes,
              },
            }))
          }
        />
      </div>
    </SettingsSection>
  );
}
