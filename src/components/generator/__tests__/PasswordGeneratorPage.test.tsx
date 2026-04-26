// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { PasswordGenerator } from "@/components/generator/PasswordGenerator";
import { clipboard, generator } from "@/lib/tauri";

vi.mock("@/components/ui/password-strength-indicator", async () => {
  const actual = await vi.importActual(
    "@/components/ui/password-strength-indicator"
  );
  return {
    ...(actual as object),
    PasswordStrengthIndicator: () => <div>Password strength</div>,
  };
});

vi.mock("@/hooks/use-clipboard-timeout", () => ({
  useClipboardTimeout: () => 30,
}));

vi.mock("@/hooks/use-clipboard-countdown", () => ({
  useClipboardCountdown: () => vi.fn(),
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
  async function activatePassphraseTab() {
    const passphraseTab = screen.getByRole("tab", {
      name: "passwordGenerator.passphraseTab",
    });
    fireEvent.mouseDown(passphraseTab, { button: 0 });
    await waitFor(() => {
      expect(passphraseTab).toHaveAttribute("data-state", "active");
    });
  }

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

  it("disables browser text assist on generated secret input", async () => {
    render(<PasswordGenerator />);

    const generatedInput = await screen.findByDisplayValue("test-password-123");

    expect(generatedInput).toHaveAttribute("autocomplete", "off");
    expect(generatedInput).toHaveAttribute("spellcheck", "false");
    expect(generatedInput).toHaveAttribute("autocorrect", "off");
    expect(generatedInput).toHaveAttribute("autocapitalize", "off");
  });

  it("uses manually edited password for copy and use actions", async () => {
    const onUsePassword = vi.fn();
    render(<PasswordGenerator onUsePassword={onUsePassword} />);

    const generatedInput = await screen.findByDisplayValue("test-password-123");
    fireEvent.change(generatedInput, { target: { value: "manual-secret" } });

    fireEvent.click(
      screen.getByRole("button", {
        name: "passwordGenerator.copyPassword",
      })
    );
    await waitFor(() => {
      expect(clipboard.copyText).toHaveBeenCalledWith("manual-secret", 30);
    });

    fireEvent.click(
      screen.getByRole("button", { name: "passwordGenerator.usePassword" })
    );
    expect(onUsePassword).toHaveBeenCalledWith("manual-secret");
  });

  it("resets edited password when regenerate is clicked", async () => {
    vi.mocked(generator.generate)
      .mockResolvedValueOnce({
        password: "test-password-123",
        entropyBits: 95.2,
      })
      .mockResolvedValueOnce({
        password: "fresh-password-456",
        entropyBits: 97.1,
      });

    render(<PasswordGenerator />);

    const generatedInput = await screen.findByDisplayValue("test-password-123");
    fireEvent.change(generatedInput, { target: { value: "manual-secret" } });

    fireEvent.click(
      screen.getByRole("button", { name: "passwordGenerator.regenerate" })
    );

    await waitFor(() => {
      expect(
        screen.getByDisplayValue("fresh-password-456")
      ).toBeInTheDocument();
    });
  });

  it("uses passphrase edits and regenerate flow when passphrase tab is active", async () => {
    const onUsePassword = vi.fn();
    vi.mocked(generator.generatePassphrase)
      .mockResolvedValueOnce({
        passphrase: "correct-horse-battery-staple",
        entropyBits: 51.7,
      })
      .mockResolvedValueOnce({
        passphrase: "delta-ocean-glass-42",
        entropyBits: 52.4,
      });

    render(<PasswordGenerator onUsePassword={onUsePassword} />);

    await activatePassphraseTab();

    const passphraseInput = await screen.findByDisplayValue(
      "correct-horse-battery-staple"
    );
    fireEvent.change(passphraseInput, {
      target: { value: "manual-passphrase" },
    });

    fireEvent.click(
      screen.getByRole("button", {
        name: "passwordGenerator.copyPassphrase",
      })
    );
    await waitFor(() => {
      expect(clipboard.copyText).toHaveBeenCalledWith("manual-passphrase", 30);
    });

    fireEvent.click(
      screen.getByRole("button", { name: "passwordGenerator.usePassphrase" })
    );
    expect(onUsePassword).toHaveBeenCalledWith("manual-passphrase");

    fireEvent.click(
      screen.getByRole("button", {
        name: "passwordGenerator.regeneratePassphrase",
      })
    );

    await waitFor(() => {
      expect(
        screen.getByDisplayValue("delta-ocean-glass-42")
      ).toBeInTheDocument();
    });
  });

  it("regenerates when password options change", async () => {
    render(<PasswordGenerator />);

    await screen.findByDisplayValue("test-password-123");

    fireEvent.change(screen.getByRole("slider"), { target: { value: "24" } });
    await waitFor(() => {
      expect(vi.mocked(generator.generate)).toHaveBeenLastCalledWith(
        expect.objectContaining({ length: 24 })
      );
    });

    fireEvent.click(
      screen.getByRole("checkbox", { name: "passwordGenerator.uppercase" })
    );
    await waitFor(() => {
      expect(vi.mocked(generator.generate)).toHaveBeenLastCalledWith(
        expect.objectContaining({ uppercase: false })
      );
    });
  });

  it("regenerates passphrase when passphrase options change", async () => {
    render(<PasswordGenerator />);

    await activatePassphraseTab();
    await screen.findByDisplayValue("correct-horse-battery-staple");

    fireEvent.change(screen.getByDisplayValue("-"), { target: { value: "_" } });
    await waitFor(() => {
      expect(vi.mocked(generator.generatePassphrase)).toHaveBeenLastCalledWith(
        expect.objectContaining({ separator: "_" })
      );
    });

    fireEvent.click(
      screen.getByRole("checkbox", { name: "passwordGenerator.includeNumber" })
    );
    await waitFor(() => {
      expect(vi.mocked(generator.generatePassphrase)).toHaveBeenLastCalledWith(
        expect.objectContaining({ includeNumber: false })
      );
    });
  });

  it("shows generator error message when password generation fails", async () => {
    vi.mocked(generator.generate).mockRejectedValueOnce(
      new Error("password generation failed")
    );

    render(<PasswordGenerator />);

    expect(
      await screen.findByText("password generation failed")
    ).toBeInTheDocument();
  });

  it("shows passphrase error message when passphrase generation fails", async () => {
    vi.mocked(generator.generatePassphrase).mockRejectedValueOnce(
      new Error("passphrase generation failed")
    );

    render(<PasswordGenerator />);
    await activatePassphraseTab();

    expect(
      await screen.findByText("passphrase generation failed")
    ).toBeInTheDocument();
  });

  it("shows generating label and disables actions while generating", async () => {
    vi.mocked(generator.generate).mockImplementation(
      () =>
        new Promise(() => {
          // Intentionally unresolved to assert loading state.
        })
    );

    render(<PasswordGenerator />);

    const input = await screen.findByDisplayValue(
      "passwordGenerator.generating"
    );
    expect(input).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "passwordGenerator.regenerate" })
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "passwordGenerator.copyPassword" })
    ).toBeDisabled();
  });

  it("disables copy/use when manual password is emptied", async () => {
    render(<PasswordGenerator onUsePassword={vi.fn()} />);

    const generatedInput = await screen.findByDisplayValue("test-password-123");
    fireEvent.change(generatedInput, { target: { value: "" } });

    expect(
      screen.getByRole("button", { name: "passwordGenerator.copyPassword" })
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "passwordGenerator.usePassword" })
    ).toBeDisabled();
  });

  it("clamps passphrase word count in number input and toggles capitalize", async () => {
    render(<PasswordGenerator />);
    await activatePassphraseTab();

    const panel = screen.getByRole("tabpanel");
    const wordCountInput = within(panel).getByRole("spinbutton");

    fireEvent.change(wordCountInput, { target: { value: "50" } });
    await waitFor(() => {
      expect(vi.mocked(generator.generatePassphrase)).toHaveBeenLastCalledWith(
        expect.objectContaining({ wordCount: 20 })
      );
    });

    fireEvent.change(wordCountInput, { target: { value: "1" } });
    await waitFor(() => {
      expect(vi.mocked(generator.generatePassphrase)).toHaveBeenLastCalledWith(
        expect.objectContaining({ wordCount: 3 })
      );
    });

    fireEvent.click(
      screen.getByRole("checkbox", {
        name: "passwordGenerator.capitalizeWords",
      })
    );
    await waitFor(() => {
      expect(vi.mocked(generator.generatePassphrase)).toHaveBeenLastCalledWith(
        expect.objectContaining({ capitalize: false })
      );
    });
  });
});
