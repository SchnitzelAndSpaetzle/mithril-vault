import { Search, X } from "lucide-react";

import { Label } from "@/components/ui/label";
import { SidebarInput } from "@/components/ui/sidebar";

interface SearchFormProps extends Omit<
  React.ComponentProps<"form">,
  "onSubmit"
> {
  query: string;
  onQueryChange: (query: string) => void;
  onClear: () => void;
  onEscape?: () => void;
  inputId?: string;
  autoFocus?: boolean;
}

export function SearchForm({
  query,
  onQueryChange,
  onClear,
  onEscape,
  inputId = "global-search-input",
  autoFocus,
  ...props
}: SearchFormProps) {
  return (
    <form {...props} onSubmit={(e) => e.preventDefault()}>
      <div className="relative">
        <Label htmlFor={inputId} className="sr-only">
          Search
        </Label>
        <SidebarInput
          id={inputId}
          placeholder="Search entries... (Ctrl+K)"
          autoFocus={autoFocus}
          className="h-8 pl-7 pr-7"
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              e.currentTarget.blur();
              onEscape?.();
            }
          }}
        />
        <Search className="pointer-events-none absolute top-1/2 left-2 size-4 -translate-y-1/2 opacity-50 select-none" />
        {query && (
          <button
            type="button"
            className="absolute top-1/2 right-2 -translate-y-1/2 opacity-50 hover:opacity-100"
            onClick={onClear}
            aria-label="Clear search"
          >
            <X className="size-4" />
          </button>
        )}
      </div>
    </form>
  );
}
