// SPDX-License-Identifier: MIT

import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PasswordGeneratorPage } from "@/components/generator/PasswordGeneratorPage";
import { clipboard, generator } from "@/lib/tauri";

vi.mock(
  "@/components/database/create-wizard/PasswordStrengthIndicator",
  () => ({
    PasswordStrengthIndicator: () => <div>Password strength</div>,
  })
);

vi.mock("@/hooks/use-clipboard-timeout", () => ({
  useClipboardTimeout: () => 30,
}));

vi.mock("@/lib/tauri", () => ({
  clipboard: {
    copyText: vi.fn(),
  },
  generator: {
    generate: vi.fn(),
    generatePassphrase: vi.fn(),
  },
}));

describe("PasswordGeneratorPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(generator.generate).mockResolvedValue({
      password: "test-password-123",
      entropyBits: 95.2,
    });
    vi.mocked(generator.generatePassphrase).mockResolvedValue({
      passphrase: "correct-horse-battery-staple",
      entropyBits: 51.7,
    });
    vi.mocked(clipboard.copyText).mockResolvedValue(undefined);
  });

  it("renders both tabs", () => {
    render(<PasswordGeneratorPage />);

    expect(
      screen.getByRole("tab", { name: "passwordGenerator.passwordTab" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("tab", { name: "passwordGenerator.passphraseTab" })
    ).toBeInTheDocument();
  });

  it("shows generated password and entropy on password tab", async () => {
    render(<PasswordGeneratorPage />);

    await waitFor(() => {
      expect(screen.getByText("test-password-123")).toBeInTheDocument();
    });

    expect(
      screen.getByText("passwordGenerator.entropyBits")
    ).toBeInTheDocument();
  });

  it("renders passphrase tab trigger", () => {
    render(<PasswordGeneratorPage />);

    const passphraseTab = screen.getByRole("tab", {
      name: "passwordGenerator.passphraseTab",
    });
    expect(passphraseTab).toBeInTheDocument();
    expect(passphraseTab).not.toHaveAttribute("data-state", "active");
  });

  it("copies password when copy button is clicked", async () => {
    render(<PasswordGeneratorPage />);

    await waitFor(() => {
      expect(screen.getByText("test-password-123")).toBeInTheDocument();
    });

    fireEvent.click(
      screen.getByRole("button", {
        name: "passwordGenerator.copyPassword",
      })
    );

    await waitFor(() => {
      expect(clipboard.copyText).toHaveBeenCalledWith("test-password-123", 30);
    });
  });
});
