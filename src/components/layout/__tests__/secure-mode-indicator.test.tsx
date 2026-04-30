// SPDX-License-Identifier: MIT

import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { SecureModeIndicator } from "@/components/layout/secure-mode-indicator";

const mockUseWindowProtection = vi.fn();

vi.mock("@/hooks/use-window-protection", () => ({
  useWindowProtection: () => mockUseWindowProtection(),
}));

describe("SecureModeIndicator", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders nothing when protection is disabled", () => {
    mockUseWindowProtection.mockReturnValue({
      enabled: false,
      isSupported: true,
    });

    const { container } = render(<SecureModeIndicator />);

    expect(container).toBeEmptyDOMElement();
  });

  it("renders indicator with active tooltip when enabled and supported", () => {
    mockUseWindowProtection.mockReturnValue({
      enabled: true,
      isSupported: true,
    });

    render(<SecureModeIndicator />);

    const indicator = screen.getByRole("status");
    expect(indicator).toHaveAttribute(
      "aria-label",
      "secureMode.indicator.activeTooltip"
    );
  });

  it("renders indicator with not-supported tooltip when enabled but unsupported", () => {
    mockUseWindowProtection.mockReturnValue({
      enabled: true,
      isSupported: false,
    });

    render(<SecureModeIndicator />);

    const indicator = screen.getByRole("status");
    expect(indicator).toHaveAttribute(
      "aria-label",
      "secureMode.indicator.notSupportedTooltip"
    );
  });
});
