// SPDX-License-Identifier: MIT

import { useDeferredValue, useEffect, useState } from "react";
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
const HAS_TRIPLE_REPEAT = /(.)\1{2,}/;
const IS_SINGLE_CHAR_ONLY = /^(.)\1+$/;

export function calculateEntropy(password: string): number {
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

function estimateTypedPasswordStrength(password: string): 0 | 1 | 2 | 3 | 4 {
  if (!password) {
    return 0;
  }

  const length = password.length;
  const characterClasses =
    Number(HAS_LOWERCASE.test(password)) +
    Number(HAS_UPPERCASE.test(password)) +
    Number(HAS_DIGITS.test(password)) +
    Number(HAS_SYMBOLS.test(password));
  const uniqueRatio = new Set(password).size / length;

  let score = 0;

  if (length >= 2) score += 1;
  if (length >= 7) score += 1;
  if (length >= 11) score += 1;
  if (length >= 16) score += 1;
  if (length >= 21) score += 1;

  if (characterClasses >= 2) score += 1;
  if (characterClasses >= 3) score += 1;
  if (characterClasses === 4) score += 1;

  if (HAS_TRIPLE_REPEAT.test(password)) score -= 2;
  if (IS_SINGLE_CHAR_ONLY.test(password)) score -= 2;

  if (uniqueRatio < 0.35) score -= 3;
  else if (uniqueRatio < 0.5) score -= 2;
  else if (uniqueRatio < 0.6) score -= 1;

  if (score <= 1) return 0;
  if (score <= 2) return 1;
  if (score <= 4) return 2;
  if (score <= 6) return 3;
  return 4;
}

interface PasswordFeedback {
  score: 0 | 1 | 2 | 3 | 4 | null;
  suggestions: string[] | null;
}

interface ResolvedPasswordFeedback extends PasswordFeedback {
  password: string;
}

function normalizeScore(score: number): 0 | 1 | 2 | 3 | 4 {
  if (score <= 0) return 0;
  if (score >= 4) return 4;
  return score as 1 | 2 | 3;
}

function usePasswordFeedback(password: string): PasswordFeedback {
  const deferredPassword = useDeferredValue(password);
  const [resolvedFeedback, setResolvedFeedback] =
    useState<ResolvedPasswordFeedback>({
      password: "",
      score: null,
      suggestions: null,
    });

  useEffect(() => {
    if (!deferredPassword) {
      return;
    }

    let cancelled = false;
    zxcvbnAsync(deferredPassword).then((res) => {
      if (!cancelled) {
        setResolvedFeedback({
          password: deferredPassword,
          score: normalizeScore(res.score),
          suggestions: res.feedback.suggestions,
        });
      }
    });

    return () => {
      cancelled = true;
    };
  }, [deferredPassword]);

  if (!deferredPassword || resolvedFeedback.password !== deferredPassword) {
    return {
      score: null,
      suggestions: null,
    };
  }

  return {
    score: resolvedFeedback.score,
    suggestions: resolvedFeedback.suggestions,
  };
}

const RAINBOW_BAR_DELAY_SECONDS = -0.4;

interface PasswordStrengthIndicatorProps {
  password: string;
  entropyBits?: number | undefined;
  className?: string;
}

export function PasswordStrengthIndicator({
  password,
  entropyBits,
  className,
}: PasswordStrengthIndicatorProps) {
  const { t } = useTranslation();
  const { score, suggestions } = usePasswordFeedback(password);

  if (!password) {
    return null;
  }

  const level =
    entropyBits === undefined
      ? (score ?? estimateTypedPasswordStrength(password))
      : getStrengthLevel(entropyBits);
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
