// SPDX-License-Identifier: MIT

import { useCallback, useEffect, useRef, useState } from "react";
import { generator } from "@/lib/tauri";
import type {
  PassphraseGeneratorOptions,
  PasswordGeneratorOptions,
} from "@/lib/types";

export function usePasswordGenerator(
  options: PasswordGeneratorOptions,
  enabled: boolean
) {
  const [password, setPassword] = useState("");
  const [entropyBits, setEntropyBits] = useState(0);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const latestRequestRef = useRef(0);

  const regenerate = useCallback(async (opts: PasswordGeneratorOptions) => {
    const requestId = ++latestRequestRef.current;
    setIsGenerating(true);
    try {
      const result = await generator.generate(opts);
      if (requestId === latestRequestRef.current) {
        setPassword(result.password);
        setEntropyBits(result.entropyBits);
        setError(null);
      }
    } catch (err) {
      if (requestId === latestRequestRef.current) {
        setPassword("");
        setEntropyBits(0);
        setError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      if (requestId === latestRequestRef.current) {
        setIsGenerating(false);
      }
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      void regenerate(options);
    }
  }, [enabled, regenerate, options]);

  return {
    password,
    entropyBits,
    isGenerating,
    error,
    regenerate: () => regenerate(options),
  };
}

export function usePassphraseGenerator(
  options: PassphraseGeneratorOptions,
  enabled: boolean
) {
  const [passphrase, setPassphrase] = useState("");
  const [entropyBits, setEntropyBits] = useState(0);
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const latestRequestRef = useRef(0);

  const regenerate = useCallback(async (opts: PassphraseGeneratorOptions) => {
    const requestId = ++latestRequestRef.current;
    setIsGenerating(true);
    try {
      const result = await generator.generatePassphrase(opts);
      if (requestId === latestRequestRef.current) {
        setPassphrase(result.passphrase);
        setEntropyBits(result.entropyBits);
        setError(null);
      }
    } catch (err) {
      if (requestId === latestRequestRef.current) {
        setPassphrase("");
        setEntropyBits(0);
        setError(err instanceof Error ? err.message : String(err));
      }
    } finally {
      if (requestId === latestRequestRef.current) {
        setIsGenerating(false);
      }
    }
  }, []);

  useEffect(() => {
    if (enabled) {
      void regenerate(options);
    }
  }, [enabled, regenerate, options]);

  return {
    passphrase,
    entropyBits,
    isGenerating,
    error,
    regenerate: () => regenerate(options),
  };
}
