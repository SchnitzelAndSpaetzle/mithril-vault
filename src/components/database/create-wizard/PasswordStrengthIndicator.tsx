import { useDeferredValue, useEffect, useRef, useState } from "react";
import { zxcvbnAsync, zxcvbnOptions, type ZxcvbnResult } from "@zxcvbn-ts/core";
import * as zxcvbnCommonPackage from "@zxcvbn-ts/language-common";
import * as zxcvbnEnPackage from "@zxcvbn-ts/language-en";
import { cn } from "@/lib/utils";

// Configure zxcvbn with dictionaries
const options = {
  dictionary: {
    ...zxcvbnCommonPackage.dictionary,
    ...zxcvbnEnPackage.dictionary,
  },
  graphs: zxcvbnCommonPackage.adjacencyGraphs,
  translations: zxcvbnEnPackage.translations,
};
zxcvbnOptions.setOptions(options);

const STRENGTH_LABELS = ["Weak", "Fair", "Good", "Strong", "Very Strong"];
const STRENGTH_COLORS = [
  "bg-red-500",
  "bg-orange-500",
  "bg-yellow-500",
  "bg-lime-500",
  "bg-green-500",
];

function usePasswordStrength(password: string): ZxcvbnResult | null {
  const deferredPassword = useDeferredValue(password);
  const [result, setResult] = useState<ZxcvbnResult | null>(null);
  const prevPasswordRef = useRef<string>("");

  useEffect(() => {
    // Only run when password actually changes
    if (deferredPassword === prevPasswordRef.current) {
      return;
    }
    prevPasswordRef.current = deferredPassword;

    if (!deferredPassword) {
      // Use a microtask to avoid synchronous setState in effect
      queueMicrotask(() => setResult(null));
      return;
    }

    let cancelled = false;
    zxcvbnAsync(deferredPassword).then((res) => {
      if (!cancelled) {
        setResult(res);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [deferredPassword]);

  return result;
}

interface PasswordStrengthIndicatorProps {
  password: string;
  className?: string;
}

export function PasswordStrengthIndicator({
  password,
  className,
}: PasswordStrengthIndicatorProps) {
  const result = usePasswordStrength(password);

  if (!password || !result) {
    return null;
  }

  const score = result.score;
  const label = STRENGTH_LABELS[score];
  const color = STRENGTH_COLORS[score];
  const feedback = result.feedback.suggestions;

  return (
    <div className={cn("space-y-2", className)}>
      <div className="flex items-center gap-2">
        <div className="flex flex-1 gap-1">
          {[0, 1, 2, 3].map((index) => (
            <div
              key={index}
              className={cn(
                "h-1.5 flex-1 rounded-full transition-colors",
                index <= score ? color : "bg-muted"
              )}
            />
          ))}
        </div>
        <span
          className={cn("text-xs font-medium", {
            "text-red-500": score === 0,
            "text-orange-500": score === 1,
            "text-yellow-600": score === 2,
            "text-lime-600": score === 3,
            "text-green-600": score === 4,
          })}
        >
          {label}
        </span>
      </div>

      {score < 3 && feedback.length > 0 && (
        <p className="text-xs text-muted-foreground">{feedback[0]}</p>
      )}
    </div>
  );
}
