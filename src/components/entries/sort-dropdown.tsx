// SPDX-License-Identifier: MIT

import { useTranslation } from "react-i18next";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { ArrowDownAZ, ArrowUpAZ } from "lucide-react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useTransition } from "react";
import { useActiveDatabase } from "@/hooks/use-active-database";
import type { EntrySortField, SortOrder } from "@/lib/types";

const SORT_FIELD_KEYS = {
  title: "entries.sort.title",
  username: "entries.sort.username",
  url: "entries.sort.url",
  modifiedAt: "entries.sort.modified",
  createdAt: "entries.sort.created",
} as const satisfies Record<EntrySortField, string>;

const DATE_FIELDS: EntrySortField[] = ["modifiedAt", "createdAt"];

function isDateField(field: EntrySortField): boolean {
  return DATE_FIELDS.includes(field);
}

export default function SortDropdown() {
  const { t } = useTranslation();
  const search = useSearch({ strict: false });
  const navigate = useNavigate();
  const { dbId } = useActiveDatabase();
  const [, startTransition] = useTransition();

  const sortBy = search.sortBy ?? "title";
  const sortOrder = search.sortOrder ?? "asc";

  const handleSortByChange = (value: string) => {
    if (!dbId) return;
    startTransition(() => {
      void navigate({
        to: "/dashboard/index/$dbId",
        params: { dbId },
        search: (prev) => ({ ...prev, sortBy: value as EntrySortField }),
      });
    });
  };

  const handleSortOrderChange = (value: string) => {
    if (!dbId) return;
    startTransition(() => {
      void navigate({
        to: "/dashboard/index/$dbId",
        params: { dbId },
        search: (prev) => ({ ...prev, sortOrder: value as SortOrder }),
      });
    });
  };

  const SortIcon = sortOrder === "asc" ? ArrowDownAZ : ArrowUpAZ;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          size="icon-sm"
          aria-label={t("entries.sort.sortEntries")}
        >
          <SortIcon />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel>{t("entries.sort.sortBy")}</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={sortBy}
          onValueChange={handleSortByChange}
        >
          {(
            Object.entries(SORT_FIELD_KEYS) as [
              EntrySortField,
              (typeof SORT_FIELD_KEYS)[EntrySortField],
            ][]
          ).map(([value, key]) => (
            <DropdownMenuRadioItem key={value} value={value}>
              {t(key)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>{t("entries.sort.order")}</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={sortOrder}
          onValueChange={handleSortOrderChange}
        >
          <DropdownMenuRadioItem value="asc">
            {isDateField(sortBy)
              ? t("entries.sort.oldestFirst")
              : t("entries.sort.aToZ")}
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="desc">
            {isDateField(sortBy)
              ? t("entries.sort.newestFirst")
              : t("entries.sort.zToA")}
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
