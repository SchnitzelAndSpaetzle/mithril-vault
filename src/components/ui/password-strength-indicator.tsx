// SPDX-License-Identifier: MIT

import { useDeferredValue, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { zxcvbnAsync, zxcvbnOptions } from "@zxcvbn-ts/core";
import * as zxcvbnCommonPackage from "@zxcvbn-ts/language-common";
import * as zxcvbnEnPackage from "@zxcvbn-ts/language-en";
import { cn } from "@/lib/utils";

const zxcvbnConfig = {
  dictionary: {
    ...zxcvbnCommonPackage.dictionary,
    ...zxcvbnEnPackage.dictionary,
  },
  graphs: zxcvbnCommonPackage.adjacencyGraphs,
  translations: zxcvbnEnPackage.translations,
};
zxcvbnOptions.setOptions(zxcvbnConfig);

const STRENGTH_BARS = [0, 1, 2, 3, 4] as const;

const STRENGTH_BAR_COLORS = [
  "bg-red-500",
  "bg-orange-500",
  "bg-yellow-500",
  "bg-green-500",
  "strength-bar-rainbow",
] as const;

const STRENGTH_LABEL_COLORS = [
  "text-red-500",
  "text-orange-500",
  "text-yellow-600",
  "text-green-600",
  "strength-label-rainbow",
] as const;

const STRENGTH_LABEL_KEYS = [
  "passwordStrength.veryWeak",
  "passwordStrength.weak",
  "passwordStrength.fair",
  "passwordStrength.strong",
  "passwordStrength.excellent",
] as const;

const HAS_LOWERCASE = /[a-z]/;
const HAS_UPPERCASE = /[A-Z]/;
const HAS_DIGITS = /[0-9]/;
const HAS_SYMBOLS = /[^a-zA-Z0-9]/;

function calculateEntropy(password: string): number {
  let charsetSize = 0;
  if (HAS_LOWERCASE.test(password)) charsetSize += 26;
  if (HAS_UPPERCASE.test(password)) charsetSize += 26;
  if (HAS_DIGITS.test(password)) charsetSize += 10;
  if (HAS_SYMBOLS.test(password)) charsetSize += 33;
  if (charsetSize === 0) return 0;
  return password.length * Math.log2(charsetSize);
}

function getStrengthLevel(entropy: number): 0 | 1 | 2 | 3 | 4 {
  if (entropy < 28) return 0;
  if (entropy < 36) return 1;
  if (entropy < 60) return 2;
  if (entropy < 128) return 3;
  return 4;
}

function usePasswordFeedback(password: string): string[] | null {
  const deferredPassword = useDeferredValue(password);
  const [suggestions, setSuggestions] = useState<string[] | null>(null);
  const prevPasswordRef = useRef<string>("");

  useEffect(() => {
    if (deferredPassword === prevPasswordRef.current) {
      return;
    }
    prevPasswordRef.current = deferredPassword;

    if (!deferredPassword) {
      queueMicrotask(() => setSuggestions(null));
      return;
    }

    let cancelled = false;
    zxcvbnAsync(deferredPassword).then((res) => {
      if (!cancelled) {
        setSuggestions(res.feedback.suggestions);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [deferredPassword]);

  return suggestions;
}

const RAINBOW_BAR_DELAY_SECONDS = -0.4;

interface PasswordStrengthIndicatorProps {
  password: string;
  entropyBits?: number;
  className?: string;
}

export function PasswordStrengthIndicator({
  password,
  entropyBits,
  className,
}: PasswordStrengthIndicatorProps) {
  const { t } = useTranslation();
  const suggestions = usePasswordFeedback(password);

  if (!password) {
    return null;
  }

  const entropy = entropyBits ?? calculateEntropy(password);
  const level = getStrengthLevel(entropy);
  const barColor = STRENGTH_BAR_COLORS[level];
  const labelColorClass = STRENGTH_LABEL_COLORS[level];
  const labelKey = STRENGTH_LABEL_KEYS[level];
  const showFeedback = level <= 2 && suggestions && suggestions.length > 0;

  return (
    <div className={cn("space-y-2", className)}>
      <div className="flex items-center gap-2">
        <div
          className="flex flex-1 gap-1"
          role="meter"
          aria-valuenow={level}
          aria-valuemin={0}
          aria-valuemax={4}
          aria-label={t(labelKey)}
        >
          {STRENGTH_BARS.map((index) => (
            <div
              key={index}
              className={cn(
                "h-1.5 flex-1 rounded-full transition-colors",
                index <= level ? barColor : "bg-muted"
              )}
              style={
                level === 4 && index <= level
                  ? { animationDelay: `${index * RAINBOW_BAR_DELAY_SECONDS}s` }
                  : undefined
              }
            />
          ))}
        </div>
        <span className={cn("text-xs font-medium", labelColorClass)}>
          {t(labelKey)}
        </span>
      </div>

      {showFeedback && (
        <p className="text-xs text-muted-foreground">{suggestions[0]}</p>
      )}
    </div>
  );
}
