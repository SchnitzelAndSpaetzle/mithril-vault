// SPDX-License-Identifier: MIT

import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

/** The effective default applied when `Meta.history_max_items` is absent. */
const DEFAULT_HISTORY_MAX_ITEMS = 10;

type HistoryMode = "limited" | "unlimited" | "disabled";

/**
 * Maps the raw `Meta.history_max_items` value to the control's mode: `null`
 * (absent) and positive values are "limited"; negative is "unlimited"; `0` is
 * "disabled".
 */
function modeOf(maxItems: number | null): HistoryMode {
  if (maxItems === null || maxItems > 0) return "limited";
  if (maxItems < 0) return "unlimited";
  return "disabled";
}

/**
 * The number shown in the "keep newest" field: the explicit positive value, or
 * the effective default when the field is absent / not currently a positive
 * limit.
 */
function limitOf(maxItems: number | null): number {
  return maxItems !== null && maxItems > 0
    ? maxItems
    : DEFAULT_HISTORY_MAX_ITEMS;
}

interface VaultHistorySettingsControlProps {
  /** Raw `Meta.history_max_items`: `null` = absent, `<0` unlimited, `0` disabled, `>0` keep N. */
  maxItems: number | null;
  /** Persists a new raw value. */
  onChange: (maxItems: number | null) => void;
  /** Disables the control (e.g. while saving or with no Vault open). */
  disabled?: boolean;
}

/**
 * Writable per-Vault Entry-History retention control (#326): keep newest N,
 * unlimited, or disabled. Maps the raw `Meta.history_max_items` value to a
 * three-way mode and back, calling `onChange` with the raw value the backend
 * stores. Presentational and controlled — the parent owns persistence.
 */
export function VaultHistorySettingsControl({
  maxItems,
  onChange,
  disabled = false,
}: Readonly<VaultHistorySettingsControlProps>) {
  const { t } = useTranslation();
  const groupId = useId();
  const mode = modeOf(maxItems);
  const limit = limitOf(maxItems);

  return (
    <fieldset className="grid gap-3" disabled={disabled}>
      <legend className="text-sm font-medium">
        {t("settings.database.history.title")}
      </legend>
      <p className="text-sm text-muted-foreground">
        {t("settings.database.history.description")}
      </p>

      <div className="flex items-center gap-2">
        <input
          type="radio"
          id={`${groupId}-limited`}
          name={groupId}
          checked={mode === "limited"}
          onChange={() => onChange(limit)}
        />
        <Label htmlFor={`${groupId}-limited`}>
          {t("settings.database.history.keepNewest")}
        </Label>
        <Input
          type="number"
          min={1}
          aria-label={t("settings.database.history.itemsLabel")}
          className="w-20"
          value={limit}
          disabled={disabled || mode !== "limited"}
          onChange={(event) => {
            const next = Number.parseInt(event.target.value, 10);
            if (Number.isFinite(next) && next >= 1) {
              onChange(next);
            }
          }}
        />
      </div>

      <div className="flex items-center gap-2">
        <input
          type="radio"
          id={`${groupId}-unlimited`}
          name={groupId}
          checked={mode === "unlimited"}
          onChange={() => onChange(-1)}
        />
        <Label htmlFor={`${groupId}-unlimited`}>
          {t("settings.database.history.unlimited")}
        </Label>
      </div>

      <div className="flex items-center gap-2">
        <input
          type="radio"
          id={`${groupId}-disabled`}
          name={groupId}
          checked={mode === "disabled"}
          onChange={() => onChange(0)}
        />
        <Label htmlFor={`${groupId}-disabled`}>
          {t("settings.database.history.disabled")}
        </Label>
      </div>
    </fieldset>
  );
}
