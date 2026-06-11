// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Filter } from "lucide-react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useTransition } from "react";
import { useActiveDatabase } from "@/hooks/use-active-database";
import { cn } from "@/lib/utils";

/// Entry-list filter menu. Sits beside the sort dropdown in the
/// header. Currently exposes a single "Has attachments" toggle that
/// drives the `hasAttachments` URL search param; the trigger is
/// accented while any filter is active so the user can tell at a
/// glance that the list is narrowed.
export default function FilterDropdown() {
  const { t } = useTranslation();
  const search = useSearch({ strict: false });
  const navigate = useNavigate();
  const { dbId } = useActiveDatabase();
  const [, startTransition] = useTransition();

  const hasAttachments = search.hasAttachments === true;
  const anyFilterActive = hasAttachments;

  const handleHasAttachmentsChange = (checked: boolean) => {
    if (!dbId) return;
    startTransition(() => {
      void navigate({
        to: "/dashboard/index/$dbId",
        params: { dbId },
        search: (prev) => ({
          ...prev,
          hasAttachments: checked ? true : undefined,
        }),
      });
    });
  };

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon-sm"
          aria-label={t("entries.filter.filterEntries")}
          className={cn(anyFilterActive && "border-primary text-primary")}
        >
          <Filter />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel>{t("entries.filter.title")}</DropdownMenuLabel>
        <DropdownMenuCheckboxItem
          checked={hasAttachments}
          onCheckedChange={handleHasAttachmentsChange}
        >
          {t("entries.filter.hasAttachments")}
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
