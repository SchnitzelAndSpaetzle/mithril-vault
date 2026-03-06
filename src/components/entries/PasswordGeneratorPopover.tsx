import {
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { Check, Copy, Dices } from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { PasswordStrengthIndicator } from "@/components/database/create-wizard/PasswordStrengthIndicator";
import { clipboard, generator } from "@/lib/tauri";
import type { PasswordGeneratorOptions } from "@/lib/types";
import { useClipboardTimeout } from "@/hooks/use-clipboard-timeout";

interface PasswordGeneratorPopoverProps {
  onUsePassword: (password: string) => void;
  children: ReactNode;
}

const DEFAULT_OPTIONS: PasswordGeneratorOptions = {
  length: 20,
  uppercase: true,
  lowercase: true,
  numbers: true,
  symbols: true,
  excludeAmbiguous: true,
  excludeChars: "",
};

export function PasswordGeneratorPopover({
  onUsePassword,
  children,
}: PasswordGeneratorPopoverProps) {
  const { t } = useTranslation();
  const clipboardClearTimeout = useClipboardTimeout();
  const [open, setOpen] = useState(false);
  const [options, setOptions] =
    useState<PasswordGeneratorOptions>(DEFAULT_OPTIONS);
  const [generatedPassword, setGeneratedPassword] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [generationError, setGenerationError] = useState<string | null>(null);
  const [isCopied, setIsCopied] = useState(false);
  const latestGenerationRequestRef = useRef(0);

  const generate = useCallback(async (opts: PasswordGeneratorOptions) => {
    const requestId = latestGenerationRequestRef.current + 1;
    latestGenerationRequestRef.current = requestId;

    setIsGenerating(true);
    try {
      const pw = await generator.generate(opts);
      if (requestId === latestGenerationRequestRef.current) {
        setGeneratedPassword(pw);
        setGenerationError(null);
      }
    } catch (error) {
      if (requestId === latestGenerationRequestRef.current) {
        setGeneratedPassword("");
        setGenerationError(
          error instanceof Error ? error.message : String(error)
        );
      }
    } finally {
      if (requestId === latestGenerationRequestRef.current) {
        setIsGenerating(false);
      }
    }
  }, []);

  useEffect(() => {
    if (open) {
      void generate(options);
    }
  }, [open, generate, options]);

  function updateOption<K extends keyof PasswordGeneratorOptions>(
    key: K,
    value: PasswordGeneratorOptions[K]
  ) {
    setOptions((prev) => ({ ...prev, [key]: value }));
  }

  function handleUse() {
    onUsePassword(generatedPassword);
    setOpen(false);
  }

  async function handleCopy() {
    if (!generatedPassword || isGenerating) {
      return;
    }

    await clipboard.copyText(generatedPassword, clipboardClearTimeout);
    setIsCopied(true);
    window.setTimeout(() => setIsCopied(false), 2000);
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent className="w-80 space-y-3" align="end">
        <div className="text-sm font-medium">
          {t("passwordGenerator.title")}
        </div>

        {/* Generated password display */}
        <div className="flex items-center gap-2 rounded-md border bg-muted/50 p-2">
          <code className="flex-1 truncate text-sm">
            {!generatedPassword && isGenerating
              ? t("passwordGenerator.generating")
              : generatedPassword}
          </code>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            aria-label={t("passwordGenerator.regenerate")}
            onClick={() => generate(options)}
            disabled={isGenerating}
          >
            <Dices className="size-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon-xs"
            aria-label={
              isCopied
                ? t("passwordGenerator.passwordCopied")
                : t("passwordGenerator.copyPassword")
            }
            onClick={() => void handleCopy()}
            disabled={isGenerating || !generatedPassword}
          >
            {isCopied ? (
              <Check className="size-3.5 text-green-500 transition-all duration-200" />
            ) : (
              <Copy className="size-3.5" />
            )}
          </Button>
        </div>

        <PasswordStrengthIndicator password={generatedPassword} />
        {generationError && (
          <p className="text-xs text-destructive">{generationError}</p>
        )}

        {/* Length control */}
        <div className="flex items-center gap-3">
          <Label className="text-xs shrink-0">
            {t("passwordGenerator.length")}
          </Label>
          <input
            type="range"
            min={4}
            max={128}
            value={options.length}
            onChange={(e) => updateOption("length", Number(e.target.value))}
            className="flex-1 accent-primary"
          />
          <Input
            type="number"
            min={4}
            max={128}
            value={options.length}
            onChange={(e) =>
              updateOption(
                "length",
                Math.min(128, Math.max(4, Number(e.target.value)))
              )
            }
            className="h-7 w-14 text-center text-xs"
          />
        </div>

        {/* Character type checkboxes */}
        <div className="grid grid-cols-2 gap-2">
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
                  (options[key as keyof typeof options] as boolean) ?? false
                }
                onCheckedChange={(checked) =>
                  updateOption(
                    key as keyof PasswordGeneratorOptions,
                    checked === true
                  )
                }
              />
              <Label htmlFor={`gen-${key}`} className="text-xs">
                {label}
              </Label>
            </div>
          ))}
        </div>

        <div className="flex items-center gap-2">
          <Checkbox
            id="gen-exclude-ambiguous"
            checked={options.excludeAmbiguous}
            onCheckedChange={(checked) =>
              updateOption("excludeAmbiguous", checked === true)
            }
          />
          <Label htmlFor="gen-exclude-ambiguous" className="text-xs">
            {t("passwordGenerator.excludeAmbiguous")}
          </Label>
        </div>

        {/* Use the password button */}
        <Button
          type="button"
          size="sm"
          className="w-full"
          onClick={handleUse}
          disabled={isGenerating || !generatedPassword}
        >
          {t("passwordGenerator.usePassword")}
        </Button>
      </PopoverContent>
    </Popover>
  );
}
