// SPDX-License-Identifier: MIT

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

const SORT_FIELD_LABELS: Record<EntrySortField, string> = {
  title: "Title",
  username: "Username",
  url: "URL",
  modifiedAt: "Modified",
  createdAt: "Created",
};

const DATE_FIELDS: EntrySortField[] = ["modifiedAt", "createdAt"];

function isDateField(field: EntrySortField): boolean {
  return DATE_FIELDS.includes(field);
}

export default function SortDropdown() {
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
        <Button variant="outline" size="icon-sm" aria-label="Sort entries">
          <SortIcon />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        <DropdownMenuLabel>Sort by</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={sortBy}
          onValueChange={handleSortByChange}
        >
          {Object.entries(SORT_FIELD_LABELS).map(([value, label]) => (
            <DropdownMenuRadioItem key={value} value={value}>
              {label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuLabel>Order</DropdownMenuLabel>
        <DropdownMenuRadioGroup
          value={sortOrder}
          onValueChange={handleSortOrderChange}
        >
          <DropdownMenuRadioItem value="asc">
            {isDateField(sortBy) ? "Oldest first" : "A \u2192 Z"}
          </DropdownMenuRadioItem>
          <DropdownMenuRadioItem value="desc">
            {isDateField(sortBy) ? "Newest first" : "Z \u2192 A"}
          </DropdownMenuRadioItem>
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
