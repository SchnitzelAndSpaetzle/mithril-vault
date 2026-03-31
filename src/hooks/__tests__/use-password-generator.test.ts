// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import {
  usePassphraseGenerator,
  usePasswordGenerator,
} from "@/hooks/use-password-generator";
import { generator } from "@/lib/tauri";

vi.mock("@/lib/tauri", () => ({
  generator: {
    generate: vi.fn(),
    generatePassphrase: vi.fn(),
  },
}));

describe("usePasswordGenerator", () => {
  it("calls generator.generate and exposes password + entropyBits", async () => {
    vi.mocked(generator.generate).mockResolvedValue({
      password: "Abc123!@#",
      entropyBits: 95.2,
    });

    const { result } = renderHook(() =>
      usePasswordGenerator(
        {
          length: 20,
          uppercase: true,
          lowercase: true,
          numbers: true,
          symbols: true,
          excludeAmbiguous: false,
        },
        true
      )
    );

    await waitFor(() => {
      expect(result.current.password).toBe("Abc123!@#");
    });

    expect(result.current.entropyBits).toBe(95.2);
    expect(result.current.error).toBeNull();
    expect(generator.generate).toHaveBeenCalled();
  });

  it("does not generate when disabled", () => {
    vi.mocked(generator.generate).mockClear();

    renderHook(() =>
      usePasswordGenerator(
        {
          length: 20,
          uppercase: true,
          lowercase: true,
          numbers: true,
          symbols: true,
          excludeAmbiguous: false,
        },
        false
      )
    );

    expect(generator.generate).not.toHaveBeenCalled();
  });

  it("propagates error state", async () => {
    vi.mocked(generator.generate).mockRejectedValue(
      new Error("generation failed")
    );

    const { result } = renderHook(() =>
      usePasswordGenerator(
        {
          length: 20,
          uppercase: true,
          lowercase: true,
          numbers: true,
          symbols: true,
          excludeAmbiguous: false,
        },
        true
      )
    );

    await waitFor(() => {
      expect(result.current.error).toBe("generation failed");
    });

    expect(result.current.password).toBe("");
    expect(result.current.entropyBits).toBe(0);
  });
});

describe("usePassphraseGenerator", () => {
  it("calls generator.generatePassphrase and exposes passphrase + entropyBits", async () => {
    vi.mocked(generator.generatePassphrase).mockResolvedValue({
      passphrase: "correct-horse-battery-staple",
      entropyBits: 51.7,
    });

    const { result } = renderHook(() =>
      usePassphraseGenerator(
        {
          wordCount: 4,
          separator: "-",
          capitalize: false,
          includeNumber: false,
        },
        true
      )
    );

    await waitFor(() => {
      expect(result.current.passphrase).toBe("correct-horse-battery-staple");
    });

    expect(result.current.entropyBits).toBe(51.7);
    expect(result.current.error).toBeNull();
    expect(generator.generatePassphrase).toHaveBeenCalled();
  });
});
