import type { FocusEvent, KeyboardEvent } from "react";
import { Controller, type Control } from "react-hook-form";
import { Input } from "@/components/ui/input";
import { Field, FieldLabel } from "@/components/ui/field";
import type { EntryFormValues } from "@/lib/formTypes";

interface EntryUsernameFieldProps {
  control: Control<EntryFormValues>;
  isPending: boolean;
  usernameSuggestions: string[];
  activeUsernameSuggestionIndex: number;
  showUsernameSuggestions: boolean;
  onFocus: () => void;
  onBlur: (event: FocusEvent<HTMLInputElement>, onBlur: () => void) => void;
  onKeyDown: (
    event: KeyboardEvent<HTMLInputElement>,
    onChange: (value: string) => void
  ) => void;
  onSelectSuggestion: (
    username: string,
    onChange: (value: string) => void
  ) => void;
}

export function EntryUsernameField({
  control,
  isPending,
  usernameSuggestions,
  activeUsernameSuggestionIndex,
  showUsernameSuggestions,
  onFocus,
  onBlur,
  onKeyDown,
  onSelectSuggestion,
}: EntryUsernameFieldProps) {
  return (
    <Field>
      <FieldLabel htmlFor="username">Username</FieldLabel>
      <Controller
        name="username"
        control={control}
        render={({ field }) => (
          <div className="relative">
            <Input
              {...field}
              id="username"
              placeholder="Username or email"
              autoComplete="username"
              disabled={isPending}
              onFocus={onFocus}
              onBlur={(event) => onBlur(event, field.onBlur)}
              onKeyDown={(event) => onKeyDown(event, field.onChange)}
            />

            {showUsernameSuggestions && (
              <div
                role="listbox"
                className="bg-popover text-popover-foreground absolute z-50 mt-1 max-h-44 w-full overflow-y-auto rounded-md border shadow-md"
              >
                {usernameSuggestions.map((username, index) => (
                  <button
                    key={username}
                    type="button"
                    role="option"
                    data-username-suggestion="true"
                    aria-selected={index === activeUsernameSuggestionIndex}
                    className="hover:bg-accent hover:text-accent-foreground data-[active=true]:bg-accent data-[active=true]:text-accent-foreground block w-full px-3 py-2 text-left text-sm"
                    data-active={index === activeUsernameSuggestionIndex}
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => onSelectSuggestion(username, field.onChange)}
                  >
                    {username}
                  </button>
                ))}
              </div>
            )}
          </div>
        )}
      />
    </Field>
  );
}
