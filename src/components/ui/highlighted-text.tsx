// SPDX-License-Identifier: MIT

import { memo } from "react";
import { highlightMatches } from "@/lib/search-utils";

interface HighlightedTextProps {
  text: string;
  query: string;
  className?: string;
}

export const HighlightedText = memo(function HighlightedText({
  text,
  query,
  className,
}: HighlightedTextProps) {
  const segments = highlightMatches(text, query);

  return (
    <span className={className}>
      {segments.map((segment, i) =>
        segment.highlighted ? (
          <mark key={i} className="bg-yellow-200 dark:bg-yellow-800 rounded-sm">
            {segment.text}
          </mark>
        ) : (
          <span key={i}>{segment.text}</span>
        )
      )}
    </span>
  );
});
