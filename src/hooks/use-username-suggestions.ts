import {
  type FocusEvent,
  type KeyboardEvent,
  useCallback,
  useMemo,
  useState,
} from "react";
import { useEntries } from "@/hooks/use-entries";

interface UseUsernameSuggestionsOptions {
  dbId: string;
  watchedUsername: string;
  isPending: boolean;
}

export function useUsernameSuggestions({
  dbId,
  watchedUsername,
  isPending,
}: UseUsernameSuggestionsOptions) {
  const [isUsernameFocused, setIsUsernameFocused] = useState(false);
  const [activeUsernameSuggestionIndex, setActiveUsernameSuggestionIndex] =
    useState(-1);
  const { data: allEntries } = useEntries(dbId);

  const usernameSuggestionsAll = useMemo(() => {
    const usernames = new Set<string>();

    for (const existingEntry of allEntries ?? []) {
      const normalizedUsername = existingEntry.username.trim();
      if (normalizedUsername.length > 0) {
        usernames.add(normalizedUsername);
      }
    }

    return Array.from(usernames).sort((a, b) =>
      a.localeCompare(b, undefined, { sensitivity: "base" })
    );
  }, [allEntries]);

  const normalizedUsernameInput = watchedUsername.trim().toLowerCase();
  const usernameSuggestions = useMemo(() => {
    const selectedUsername = watchedUsername.trim().toLowerCase();
    return usernameSuggestionsAll.filter((username) => {
      const normalized = username.toLowerCase();
      if (normalized === selectedUsername) {
        return false;
      }

      return (
        normalizedUsernameInput.length > 0 &&
        normalized.includes(normalizedUsernameInput)
      );
    });
  }, [normalizedUsernameInput, usernameSuggestionsAll, watchedUsername]);

  const showUsernameSuggestions =
    isUsernameFocused && !isPending && usernameSuggestions.length > 0;

  const resetSuggestionState = useCallback(() => {
    setIsUsernameFocused(false);
    setActiveUsernameSuggestionIndex(-1);
  }, []);

  const handleFocus = useCallback(() => {
    setIsUsernameFocused(true);
    setActiveUsernameSuggestionIndex(-1);
  }, []);

  const applySuggestion = useCallback(
    (
      username: string,
      onChange: (value: string) => void,
      resetFocus = true
    ) => {
      onChange(username);
      if (resetFocus) {
        resetSuggestionState();
      } else {
        setActiveUsernameSuggestionIndex(-1);
      }
    },
    [resetSuggestionState]
  );

  const handleBlur = useCallback(
    (event: FocusEvent<HTMLInputElement>, onBlur: () => void) => {
      onBlur();
      if (
        event.relatedTarget instanceof HTMLElement &&
        event.relatedTarget.dataset["usernameSuggestion"] === "true"
      ) {
        return;
      }

      resetSuggestionState();
    },
    [resetSuggestionState]
  );

  const handleKeyDown = useCallback(
    (
      event: KeyboardEvent<HTMLInputElement>,
      onChange: (value: string) => void
    ) => {
      if (event.key === "ArrowDown" && showUsernameSuggestions) {
        event.preventDefault();
        setActiveUsernameSuggestionIndex((prev) =>
          prev < usernameSuggestions.length - 1 ? prev + 1 : 0
        );
        return;
      }

      if (event.key === "ArrowUp" && showUsernameSuggestions) {
        event.preventDefault();
        setActiveUsernameSuggestionIndex((prev) =>
          prev > 0 ? prev - 1 : usernameSuggestions.length - 1
        );
        return;
      }

      if (event.key === "Enter" && showUsernameSuggestions) {
        const selectedSuggestion =
          activeUsernameSuggestionIndex >= 0
            ? usernameSuggestions[activeUsernameSuggestionIndex]
            : usernameSuggestions[0];
        if (selectedSuggestion) {
          event.preventDefault();
          applySuggestion(selectedSuggestion, onChange);
        }
      }
    },
    [
      activeUsernameSuggestionIndex,
      applySuggestion,
      showUsernameSuggestions,
      usernameSuggestions,
    ]
  );

  return {
    usernameSuggestions,
    activeUsernameSuggestionIndex,
    showUsernameSuggestions,
    resetSuggestionState,
    handleFocus,
    handleBlur,
    handleKeyDown,
    applySuggestion,
  };
}
