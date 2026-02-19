import { type KeyboardEvent, useState } from "react";
import { X } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";

interface TagInputProps {
  value: string[];
  onChange: (tags: string[]) => void;
  disabled?: boolean;
  suggestions?: string[];
}

export function TagInput({
  value,
  onChange,
  disabled,
  suggestions = [],
}: TagInputProps) {
  const [input, setInput] = useState("");
  const [isFocused, setIsFocused] = useState(false);
  const [activeSuggestionIndex, setActiveSuggestionIndex] = useState(-1);

  const normalizedInput = input.trim().toLowerCase();
  const selectedTags = new Set(value.map((tag) => tag.toLowerCase()));

  const matchingSuggestions = suggestions.filter((suggestion) => {
    const normalizedSuggestion = suggestion.trim().toLowerCase();
    if (
      normalizedSuggestion.length === 0 ||
      selectedTags.has(normalizedSuggestion)
    ) {
      return false;
    }
    return (
      normalizedInput.length > 0 &&
      normalizedSuggestion.includes(normalizedInput)
    );
  });

  const showSuggestions =
    isFocused && !disabled && matchingSuggestions.length > 0;

  function addTag(tag: string) {
    const trimmed = tag.trim();
    if (trimmed && !value.includes(trimmed)) {
      onChange([...value, trimmed]);
    }
    setInput("");
    setActiveSuggestionIndex(-1);
  }

  function removeTag(tag: string) {
    onChange(value.filter((t) => t !== tag));
  }

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown" && showSuggestions) {
      e.preventDefault();
      setActiveSuggestionIndex((prev) =>
        prev < matchingSuggestions.length - 1 ? prev + 1 : 0
      );
      return;
    }

    if (e.key === "ArrowUp" && showSuggestions) {
      e.preventDefault();
      setActiveSuggestionIndex((prev) =>
        prev > 0 ? prev - 1 : matchingSuggestions.length - 1
      );
      return;
    }

    if (e.key === "Enter") {
      e.preventDefault();
      if (showSuggestions) {
        const selectedSuggestion =
          activeSuggestionIndex >= 0
            ? matchingSuggestions[activeSuggestionIndex]
            : matchingSuggestions[0];
        if (selectedSuggestion) {
          addTag(selectedSuggestion);
          return;
        }
      }
      addTag(input);
    } else if (e.key === "Backspace" && input === "" && value.length > 0) {
      const lastTag = value[value.length - 1];
      if (lastTag) {
        removeTag(lastTag);
      }
    }
  }

  return (
    <div className="relative">
      <div className="flex min-h-9 flex-wrap items-center gap-1.5 rounded-md border px-3 py-1.5 shadow-xs">
        {value.map((tag) => (
          <Badge key={tag} variant="secondary" className="gap-1">
            {tag}
            <button
              type="button"
              aria-label={`Remove tag ${tag}`}
              onClick={() => removeTag(tag)}
              disabled={disabled}
              className="rounded-full hover:bg-muted-foreground/20"
            >
              <X className="size-3" />
            </button>
          </Badge>
        ))}
        <Input
          value={input}
          onChange={(e) => {
            setInput(e.target.value);
            setActiveSuggestionIndex(-1);
          }}
          onKeyDown={handleKeyDown}
          onFocus={() => {
            setIsFocused(true);
            setActiveSuggestionIndex(-1);
          }}
          onBlur={() => {
            setIsFocused(false);
            setActiveSuggestionIndex(-1);
            addTag(input);
          }}
          placeholder={value.length === 0 ? "Add tags..." : ""}
          disabled={disabled}
          autoComplete="off"
          className="h-7 min-w-20 flex-1 border-0 bg-transparent px-0 shadow-none focus-visible:ring-0"
        />
      </div>

      {showSuggestions && (
        <div
          role="listbox"
          className="bg-popover text-popover-foreground absolute z-50 mt-1 max-h-44 w-full overflow-y-auto rounded-md border shadow-md"
        >
          {matchingSuggestions.map((suggestion, index) => (
            <button
              key={suggestion}
              type="button"
              role="option"
              aria-selected={index === activeSuggestionIndex}
              className="hover:bg-accent hover:text-accent-foreground data-[active=true]:bg-accent data-[active=true]:text-accent-foreground block w-full px-3 py-2 text-left text-sm"
              data-active={index === activeSuggestionIndex}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => addTag(suggestion)}
            >
              {suggestion}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
