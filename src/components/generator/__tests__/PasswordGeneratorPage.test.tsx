// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PasswordGenerator } from "@/components/generator/PasswordGenerator";
import { clipboard, generator } from "@/lib/tauri";

vi.mock("@/components/ui/password-strength-indicator", () => ({
  PasswordStrengthIndicator: () => <div>Password strength</div>,
}));

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

describe("PasswordGenerator", () => {
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
    render(<PasswordGenerator />);

    expect(
      screen.getByRole("tab", { name: "passwordGenerator.passwordTab" })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("tab", { name: "passwordGenerator.passphraseTab" })
    ).toBeInTheDocument();
  });

  it("shows generated password and entropy on password tab", async () => {
    render(<PasswordGenerator />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("test-password-123")).toBeInTheDocument();
    });

    expect(
      screen.getByText("passwordGenerator.entropyBits")
    ).toBeInTheDocument();
  });

  it("renders passphrase tab trigger", () => {
    render(<PasswordGenerator />);

    const passphraseTab = screen.getByRole("tab", {
      name: "passwordGenerator.passphraseTab",
    });
    expect(passphraseTab).toBeInTheDocument();
    expect(passphraseTab).not.toHaveAttribute("data-state", "active");
  });

  it("copies password when copy button is clicked", async () => {
    render(<PasswordGenerator />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("test-password-123")).toBeInTheDocument();
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

  it("does not render use button when onUsePassword is not provided", () => {
    render(<PasswordGenerator />);

    expect(
      screen.queryByRole("button", { name: "passwordGenerator.usePassword" })
    ).not.toBeInTheDocument();
  });

  it("renders use button when onUsePassword is provided", async () => {
    render(<PasswordGenerator onUsePassword={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("test-password-123")).toBeInTheDocument();
    });

    expect(
      screen.getByRole("button", { name: "passwordGenerator.usePassword" })
    ).toBeInTheDocument();
  });

  it("calls onUsePassword with current password when use button is clicked", async () => {
    const onUsePassword = vi.fn();
    render(<PasswordGenerator onUsePassword={onUsePassword} />);

    await waitFor(() => {
      expect(screen.getByDisplayValue("test-password-123")).toBeInTheDocument();
    });

    fireEvent.click(
      screen.getByRole("button", { name: "passwordGenerator.usePassword" })
    );

    expect(onUsePassword).toHaveBeenCalledWith("test-password-123");
  });
});
