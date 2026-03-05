// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PasswordGeneratorPopover } from "@/components/entries/PasswordGeneratorPopover";
import { clipboard, generator } from "@/lib/tauri";

vi.mock(
  "@/components/database/create-wizard/PasswordStrengthIndicator",
  () => ({
    PasswordStrengthIndicator: () => <div>Password strength</div>,
  })
);

vi.mock("@/hooks/use-clipboard-timeout", () => ({
  useClipboardTimeout: () => 12,
}));

vi.mock("@/lib/tauri", () => ({
  clipboard: {
    copyText: vi.fn(),
  },
  generator: {
    generate: vi.fn(),
  },
}));

describe("PasswordGeneratorPopover", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(generator.generate).mockResolvedValue("generated-password");
    vi.mocked(clipboard.copyText).mockResolvedValue(undefined);
  });

  it("copies generated password using settings-driven clipboard timeout", async () => {
    render(
      <PasswordGeneratorPopover onUsePassword={vi.fn()}>
        <button type="button">Open</button>
      </PasswordGeneratorPopover>
    );

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(generator.generate).toHaveBeenCalled();
    });

    fireEvent.click(screen.getByRole("button", { name: "Copy password" }));

    await waitFor(() => {
      expect(clipboard.copyText).toHaveBeenCalledWith("generated-password", 12);
    });
  });

  it("shows generation error and prevents copy when generation fails", async () => {
    vi.mocked(generator.generate).mockRejectedValueOnce(
      new Error("gen failed")
    );

    render(
      <PasswordGeneratorPopover onUsePassword={vi.fn()}>
        <button type="button">Open</button>
      </PasswordGeneratorPopover>
    );

    fireEvent.click(screen.getByRole("button", { name: "Open" }));

    await waitFor(() => {
      expect(screen.getByText("gen failed")).toBeInTheDocument();
    });

    expect(
      screen.getByRole("button", { name: "Copy password" })
    ).toBeDisabled();
    expect(clipboard.copyText).not.toHaveBeenCalled();
  });
});
