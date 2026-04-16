// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PasswordGeneratorDialog } from "@/components/entries/PasswordGeneratorDialog";
import { clipboard, generator } from "@/lib/tauri";

vi.mock("@/components/ui/password-strength-indicator", () => ({
  PasswordStrengthIndicator: () => <div>Password strength</div>,
}));

vi.mock("@/hooks/use-clipboard-timeout", () => ({
  useClipboardTimeout: () => 12,
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

describe("PasswordGeneratorDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(generator.generate).mockResolvedValue({
      password: "generated-password",
      entropyBits: 128,
    });
    vi.mocked(generator.generatePassphrase).mockResolvedValue({
      passphrase: "correct-horse-battery-staple",
      entropyBits: 51.7,
    });
    vi.mocked(clipboard.copyText).mockResolvedValue(undefined);
  });

  it("opens dialog and shows generator when trigger is clicked", async () => {
    render(
      <PasswordGeneratorDialog onUsePassword={vi.fn()}>
        <button type="button">Open</button>
      </PasswordGeneratorDialog>
    );

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(
        screen.getByRole("tab", { name: "passwordGenerator.passwordTab" })
      ).toBeInTheDocument();
    });
  });

  it("calls onUsePassword and closes dialog when use button is clicked", async () => {
    const onUsePassword = vi.fn();
    render(
      <PasswordGeneratorDialog onUsePassword={onUsePassword}>
        <button type="button">Open</button>
      </PasswordGeneratorDialog>
    );

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(
        screen.getByDisplayValue("generated-password")
      ).toBeInTheDocument();
    });

    fireEvent.click(
      screen.getByRole("button", { name: "passwordGenerator.usePassword" })
    );

    expect(onUsePassword).toHaveBeenCalledWith("generated-password");
  });
});
