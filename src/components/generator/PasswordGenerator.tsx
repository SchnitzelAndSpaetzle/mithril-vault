// SPDX-License-Identifier: MIT

import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Copy, Dices } from "lucide-react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { PasswordStrengthIndicator } from "@/components/ui/password-strength-indicator";
import {
  usePassphraseGenerator,
  usePasswordGenerator,
} from "@/hooks/use-password-generator";
import { clipboard } from "@/lib/tauri";
import { useClipboardTimeout } from "@/hooks/use-clipboard-timeout";
import type {
  PassphraseGeneratorOptions,
  PasswordGeneratorOptions,
} from "@/lib/types";

const DEFAULT_PASSWORD_OPTIONS: PasswordGeneratorOptions = {
  length: 20,
  uppercase: true,
  lowercase: true,
  numbers: true,
  symbols: true,
  excludeAmbiguous: true,
  excludeChars: "",
  minNumbers: 0,
  minSymbols: 0,
};

const DEFAULT_PASSPHRASE_OPTIONS: PassphraseGeneratorOptions = {
  wordCount: 4,
  separator: "-",
  capitalize: true,
  includeNumber: true,
};

interface PasswordGeneratorProps {
  onUsePassword?: (value: string) => void;
}

export function PasswordGenerator({ onUsePassword }: PasswordGeneratorProps) {
  const { t } = useTranslation();
  const clipboardClearTimeout = useClipboardTimeout();
  const [activeTab, setActiveTab] = useState("password");

  const [passwordOptions, setPasswordOptions] =
    useState<PasswordGeneratorOptions>(DEFAULT_PASSWORD_OPTIONS);
  const [passphraseOptions, setPassphraseOptions] =
    useState<PassphraseGeneratorOptions>(DEFAULT_PASSPHRASE_OPTIONS);

  const stablePasswordOptions = useMemo(
    () => passwordOptions,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [JSON.stringify(passwordOptions)]
  );
  const stablePassphraseOptions = useMemo(
    () => passphraseOptions,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [JSON.stringify(passphraseOptions)]
  );

  const passwordGen = usePasswordGenerator(
    stablePasswordOptions,
    activeTab === "password"
  );
  const passphraseGen = usePassphraseGenerator(
    stablePassphraseOptions,
    activeTab === "passphrase"
  );

  const [isCopied, setIsCopied] = useState(false);
  const [customPassword, setCustomPassword] = useState<string | null>(null);
  const [customPassphrase, setCustomPassphrase] = useState<string | null>(null);

  const effectivePassword = customPassword ?? passwordGen.password;
  const effectivePassphrase = customPassphrase ?? passphraseGen.passphrase;
  const currentValue =
    activeTab === "password" ? effectivePassword : effectivePassphrase;

  async function handleCopy(text: string) {
    if (!text) return;
    await clipboard.copyText(text, clipboardClearTimeout);
    setIsCopied(true);
    window.setTimeout(() => setIsCopied(false), 2000);
  }

  function handleUse() {
    if (onUsePassword && currentValue) {
      onUsePassword(currentValue);
    }
  }

  function updatePasswordOption<K extends keyof PasswordGeneratorOptions>(
    key: K,
    value: PasswordGeneratorOptions[K]
  ) {
    setPasswordOptions((prev) => ({ ...prev, [key]: value }));
  }

  function updatePassphraseOption<K extends keyof PassphraseGeneratorOptions>(
    key: K,
    value: PassphraseGeneratorOptions[K]
  ) {
    setPassphraseOptions((prev) => ({ ...prev, [key]: value }));
  }

  return (
    <div className="w-full max-w-lg space-y-4">
      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList className="w-full">
          <TabsTrigger value="password">
            {t("passwordGenerator.passwordTab")}
          </TabsTrigger>
          <TabsTrigger value="passphrase">
            {t("passwordGenerator.passphraseTab")}
          </TabsTrigger>
        </TabsList>

        {/* Password Tab */}
        <TabsContent value="password" className="space-y-4">
          <GeneratedDisplay
            value={effectivePassword}
            isGenerating={passwordGen.isGenerating}
            isCopied={isCopied}
            onRegenerate={() => {
              setCustomPassword(null);
              passwordGen.regenerate();
            }}
            onChange={setCustomPassword}
            onCopy={() => void handleCopy(effectivePassword)}
            regenerateLabel={t("passwordGenerator.regenerate")}
            copyLabel={t("passwordGenerator.copyPassword")}
            copiedLabel={t("passwordGenerator.passwordCopied")}
            generatingLabel={t("passwordGenerator.generating")}
          />

          <PasswordStrengthIndicator
            password={effectivePassword}
            entropyBits={
              customPassword === null ? passwordGen.entropyBits : undefined
            }
          />

          {customPassword === null && passwordGen.entropyBits > 0 && (
            <p className="text-xs text-muted-foreground">
              {t("passwordGenerator.entropyBits", {
                bits: Math.round(passwordGen.entropyBits),
              })}
            </p>
          )}

          {passwordGen.error && (
            <p className="text-xs text-destructive">{passwordGen.error}</p>
          )}

          {/* Length slider */}
          <div className="flex items-center gap-3">
            <Label className="shrink-0 text-sm">
              {t("passwordGenerator.length")}
            </Label>
            <input
              type="range"
              min={4}
              max={128}
              value={passwordOptions.length}
              onChange={(e) =>
                updatePasswordOption("length", Number(e.target.value))
              }
              className="flex-1 accent-primary"
            />
            <Input
              type="number"
              min={4}
              max={128}
              value={passwordOptions.length}
              onChange={(e) =>
                updatePasswordOption(
                  "length",
                  Math.min(128, Math.max(4, Number(e.target.value)))
                )
              }
              className="h-8 w-16 text-center text-sm"
            />
          </div>

          {/* Character type checkboxes */}
          <div className="grid grid-cols-2 gap-3">
            {(
              [
                ["uppercase", t("passwordGenerator.uppercase")],
                ["lowercase", t("passwordGenerator.lowercase")],
                ["numbers", t("passwordGenerator.numbers")],
                ["symbols", t("passwordGenerator.symbols")],
              ] as const
            ).map(([key, label]) => (
              <div key={key} className="flex items-center gap-2">
                <Checkbox
                  id={`gen-${key}`}
                  checked={
                    (passwordOptions[
                      key as keyof typeof passwordOptions
                    ] as boolean) ?? false
                  }
                  onCheckedChange={(checked) =>
                    updatePasswordOption(
                      key as keyof PasswordGeneratorOptions,
                      checked === true
                    )
                  }
                />
                <Label htmlFor={`gen-${key}`} className="text-sm">
                  {label}
                </Label>
              </div>
            ))}
          </div>

          <div className="flex items-center gap-2">
            <Checkbox
              id="gen-exclude-ambiguous"
              checked={passwordOptions.excludeAmbiguous}
              onCheckedChange={(checked) =>
                updatePasswordOption("excludeAmbiguous", checked === true)
              }
            />
            <Label htmlFor="gen-exclude-ambiguous" className="text-sm">
              {t("passwordGenerator.excludeAmbiguous")}
            </Label>
          </div>

          {/* Exclude custom characters */}
          <div className="flex items-center gap-3">
            <Label className="shrink-0 text-sm">
              {t("passwordGenerator.excludeCustom")}
            </Label>
            <Input
              value={passwordOptions.excludeChars ?? ""}
              onChange={(e) =>
                updatePasswordOption("excludeChars", e.target.value)
              }
              placeholder="e.g. {}[]"
              className="h-8 text-sm"
            />
          </div>

          {/* Min numbers / min symbols */}
          <div className="grid grid-cols-2 gap-3">
            <div className="flex items-center gap-2">
              <Label className="shrink-0 text-sm">
                {t("passwordGenerator.minNumbers")}
              </Label>
              <Input
                type="number"
                min={0}
                max={passwordOptions.length}
                value={passwordOptions.minNumbers ?? 0}
                onChange={(e) =>
                  updatePasswordOption(
                    "minNumbers",
                    Math.max(0, Number(e.target.value))
                  )
                }
                disabled={!passwordOptions.numbers}
                className="h-8 w-16 text-center text-sm"
              />
            </div>
            <div className="flex items-center gap-2">
              <Label className="shrink-0 text-sm">
                {t("passwordGenerator.minSymbols")}
              </Label>
              <Input
                type="number"
                min={0}
                max={passwordOptions.length}
                value={passwordOptions.minSymbols ?? 0}
                onChange={(e) =>
                  updatePasswordOption(
                    "minSymbols",
                    Math.max(0, Number(e.target.value))
                  )
                }
                disabled={!passwordOptions.symbols}
                className="h-8 w-16 text-center text-sm"
              />
            </div>
          </div>
        </TabsContent>

        {/* Passphrase Tab */}
        <TabsContent value="passphrase" className="space-y-4">
          <GeneratedDisplay
            value={effectivePassphrase}
            isGenerating={passphraseGen.isGenerating}
            isCopied={isCopied}
            onRegenerate={() => {
              setCustomPassphrase(null);
              passphraseGen.regenerate();
            }}
            onChange={setCustomPassphrase}
            onCopy={() => void handleCopy(effectivePassphrase)}
            regenerateLabel={t("passwordGenerator.regeneratePassphrase")}
            copyLabel={t("passwordGenerator.copyPassphrase")}
            copiedLabel={t("passwordGenerator.passphraseCopied")}
            generatingLabel={t("passwordGenerator.generating")}
          />

          <PasswordStrengthIndicator
            password={effectivePassphrase}
            entropyBits={
              customPassphrase === null ? passphraseGen.entropyBits : undefined
            }
          />

          {customPassphrase === null && passphraseGen.entropyBits > 0 && (
            <p className="text-xs text-muted-foreground">
              {t("passwordGenerator.entropyBits", {
                bits: Math.round(passphraseGen.entropyBits),
              })}
            </p>
          )}

          {passphraseGen.error && (
            <p className="text-xs text-destructive">{passphraseGen.error}</p>
          )}

          {/* Word count slider */}
          <div className="flex items-center gap-3">
            <Label className="shrink-0 text-sm">
              {t("passwordGenerator.wordCount")}
            </Label>
            <input
              type="range"
              min={3}
              max={20}
              value={passphraseOptions.wordCount}
              onChange={(e) =>
                updatePassphraseOption("wordCount", Number(e.target.value))
              }
              className="flex-1 accent-primary"
            />
            <Input
              type="number"
              min={3}
              max={20}
              value={passphraseOptions.wordCount}
              onChange={(e) =>
                updatePassphraseOption(
                  "wordCount",
                  Math.min(20, Math.max(3, Number(e.target.value)))
                )
              }
              className="h-8 w-16 text-center text-sm"
            />
          </div>

          {/* Separator */}
          <div className="flex items-center gap-3">
            <Label className="shrink-0 text-sm">
              {t("passwordGenerator.separator")}
            </Label>
            <Input
              value={passphraseOptions.separator}
              onChange={(e) =>
                updatePassphraseOption("separator", e.target.value)
              }
              className="h-8 w-20 text-center text-sm"
            />
          </div>

          {/* Capitalize / Include number */}
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <Checkbox
                id="gen-capitalize"
                checked={passphraseOptions.capitalize}
                onCheckedChange={(checked) =>
                  updatePassphraseOption("capitalize", checked === true)
                }
              />
              <Label htmlFor="gen-capitalize" className="text-sm">
                {t("passwordGenerator.capitalizeWords")}
              </Label>
            </div>
            <div className="flex items-center gap-2">
              <Checkbox
                id="gen-include-number"
                checked={passphraseOptions.includeNumber}
                onCheckedChange={(checked) =>
                  updatePassphraseOption("includeNumber", checked === true)
                }
              />
              <Label htmlFor="gen-include-number" className="text-sm">
                {t("passwordGenerator.includeNumber")}
              </Label>
            </div>
          </div>
        </TabsContent>
      </Tabs>

      {onUsePassword && (
        <Button
          type="button"
          className="w-full"
          onClick={handleUse}
          disabled={
            (activeTab === "password" && passwordGen.isGenerating) ||
            (activeTab === "passphrase" && passphraseGen.isGenerating) ||
            !currentValue
          }
        >
          {activeTab === "password"
            ? t("passwordGenerator.usePassword")
            : t("passwordGenerator.usePassphrase")}
        </Button>
      )}
    </div>
  );
}

interface GeneratedDisplayProps {
  value: string;
  isGenerating: boolean;
  isCopied: boolean;
  onRegenerate: () => void;
  onChange: (value: string) => void;
  onCopy: () => void;
  regenerateLabel: string;
  copyLabel: string;
  copiedLabel: string;
  generatingLabel: string;
}

function GeneratedDisplay({
  value,
  isGenerating,
  isCopied,
  onRegenerate,
  onChange,
  onCopy,
  regenerateLabel,
  copyLabel,
  copiedLabel,
  generatingLabel,
}: GeneratedDisplayProps) {
  return (
    <div className="flex items-center gap-2 rounded-md border bg-muted/50 p-3">
      <input
        type="text"
        value={!value && isGenerating ? generatingLabel : value}
        onChange={(e) => onChange(e.target.value)}
        className="flex-1 break-all bg-transparent text-sm font-mono outline-none"
      />
      <Button
        type="button"
        variant="ghost"
        size="icon-xs"
        aria-label={regenerateLabel}
        onClick={onRegenerate}
        disabled={isGenerating}
      >
        <Dices className="size-4" />
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon-xs"
        aria-label={isCopied ? copiedLabel : copyLabel}
        onClick={onCopy}
        disabled={isGenerating || !value}
      >
        {isCopied ? (
          <Check className="size-4 text-green-500 transition-all duration-200" />
        ) : (
          <Copy className="size-4" />
        )}
      </Button>
    </div>
  );
}
