// SPDX-License-Identifier: MIT

import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PasswordStrengthIndicator } from "@/components/ui/password-strength-indicator";

const mockZxcvbnAsync = vi.fn();

vi.mock("@zxcvbn-ts/core", () => ({
  zxcvbnAsync: (password: string) => mockZxcvbnAsync(password),
  zxcvbnOptions: { setOptions: vi.fn() },
}));

vi.mock("@zxcvbn-ts/language-common", () => ({
  dictionary: {},
  adjacencyGraphs: {},
}));

vi.mock("@zxcvbn-ts/language-en", () => ({
  dictionary: {},
  translations: {},
}));

function makeFeedbackWithScore(
  score: 0 | 1 | 2 | 3 | 4,
  suggestions: string[] = []
) {
  return { score, feedback: { suggestions, warning: "" } };
}

describe("PasswordStrengthIndicator", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockZxcvbnAsync.mockImplementation(async (password: string) => {
      const defaultScores: Record<string, 0 | 1 | 2 | 3 | 4> = {
        ab: 0,
        abcdefg: 1,
        abcdefghijk: 2,
        "Password1!": 3,
        "Correct-Horse-Batt1!": 4,
      };
      return makeFeedbackWithScore(defaultScores[password] ?? 0);
    });
  });

  it("renders nothing when password is empty", () => {
    const { container } = render(<PasswordStrengthIndicator password="" />);
    expect(container.firstChild).toBeNull();
  });

  // --- Very Weak (<28 bits): "ab" = 9.4 bits ---

  it("shows 'Very Weak' for low-entropy password", () => {
    render(<PasswordStrengthIndicator password="ab" />);
    expect(screen.getByText("passwordStrength.veryWeak")).toBeInTheDocument();
  });

  it("Very Weak: 1 active red bar, 4 muted", () => {
    render(<PasswordStrengthIndicator password="ab" />);
    const bars = Array.from(screen.getByRole("meter").children);
    expect(bars).toHaveLength(5);
    expect(bars[0]).toHaveClass("bg-red-500");
    expect(bars[1]).toHaveClass("bg-muted");
    expect(bars[4]).toHaveClass("bg-muted");
  });

  it("Very Weak: shows zxcvbn feedback when available", async () => {
    mockZxcvbnAsync.mockResolvedValue(
      makeFeedbackWithScore(0, ["Use a longer password"])
    );
    render(<PasswordStrengthIndicator password="ab" />);

    await waitFor(() => {
      expect(screen.getByText("Use a longer password")).toBeInTheDocument();
    });
  });

  // --- Weak (28-35 bits): "abcdefg" = 32.9 bits ---

  it("shows 'Weak' for entropy 28-35", () => {
    render(<PasswordStrengthIndicator password="abcdefg" />);
    expect(screen.getByText("passwordStrength.weak")).toBeInTheDocument();
  });

  it("Weak: 2 active orange bars, 3 muted", () => {
    render(<PasswordStrengthIndicator password="abcdefg" />);
    const bars = Array.from(screen.getByRole("meter").children);
    expect(bars[0]).toHaveClass("bg-orange-500");
    expect(bars[1]).toHaveClass("bg-orange-500");
    expect(bars[2]).toHaveClass("bg-muted");
  });

  it("Weak: shows zxcvbn feedback when available", async () => {
    mockZxcvbnAsync.mockResolvedValue(
      makeFeedbackWithScore(1, ["Avoid common words"])
    );
    render(<PasswordStrengthIndicator password="abcdefg" />);

    await waitFor(() => {
      expect(screen.getByText("Avoid common words")).toBeInTheDocument();
    });
  });

  // --- Fair (36-59 bits): "abcdefghijk" = 51.7 bits ---

  it("shows 'Fair' for entropy 36-59", () => {
    render(<PasswordStrengthIndicator password="abcdefghijk" />);
    expect(screen.getByText("passwordStrength.fair")).toBeInTheDocument();
  });

  it("Fair: shows feedback (boundary: level <= 2)", async () => {
    mockZxcvbnAsync.mockResolvedValue(
      makeFeedbackWithScore(2, ["Add more symbols"])
    );
    render(<PasswordStrengthIndicator password="abcdefghijk" />);

    await waitFor(() => {
      expect(screen.getByText("Add more symbols")).toBeInTheDocument();
    });
  });

  // --- Strong (60-127 bits): "Password1!" = 65.7 bits ---

  it("shows 'Strong' for entropy 60-127", () => {
    render(<PasswordStrengthIndicator password="Password1!" />);
    expect(screen.getByText("passwordStrength.strong")).toBeInTheDocument();
  });

  it("Strong: 4 active green bars, 1 muted", () => {
    render(<PasswordStrengthIndicator password="Password1!" />);
    const bars = Array.from(screen.getByRole("meter").children);
    expect(bars[0]).toHaveClass("bg-green-500");
    expect(bars[3]).toHaveClass("bg-green-500");
    expect(bars[4]).toHaveClass("bg-muted");
  });

  it("Strong: does NOT show feedback", async () => {
    mockZxcvbnAsync.mockResolvedValue(makeFeedbackWithScore(3, ["Some tip"]));
    render(<PasswordStrengthIndicator password="Password1!" />);

    await waitFor(() => {
      expect(mockZxcvbnAsync).toHaveBeenCalled();
    });

    expect(screen.queryByText("Some tip")).not.toBeInTheDocument();
  });

  // --- Excellent (128+ bits): "Correct-Horse-Batt1!" = ~138 bits ---

  it("shows 'Excellent' for entropy >= 128", () => {
    render(<PasswordStrengthIndicator password="Correct-Horse-Batt1!" />);
    expect(screen.getByText("passwordStrength.excellent")).toBeInTheDocument();
  });

  it("Excellent: all 5 bars have rainbow class", () => {
    render(<PasswordStrengthIndicator password="Correct-Horse-Batt1!" />);
    const bars = Array.from(screen.getByRole("meter").children);
    bars.forEach((bar) => {
      expect(bar).toHaveClass("strength-bar-rainbow");
    });
  });

  it("Excellent: bars have staggered animation-delay for wave effect", () => {
    render(<PasswordStrengthIndicator password="Correct-Horse-Batt1!" />);
    const bars = Array.from(
      screen.getByRole("meter").children
    ) as HTMLElement[];
    expect(bars[0]?.style.animationDelay).toBe("0s");
    expect(bars[1]?.style.animationDelay).toBe("-0.4s");
    expect(bars[4]?.style.animationDelay).toBe("-1.6s");
  });

  it("Excellent: label has rainbow class", () => {
    render(<PasswordStrengthIndicator password="Correct-Horse-Batt1!" />);
    expect(screen.getByText("passwordStrength.excellent")).toHaveClass(
      "strength-label-rainbow"
    );
  });

  it("Excellent: does NOT show feedback", async () => {
    mockZxcvbnAsync.mockResolvedValue(
      makeFeedbackWithScore(4, ["This should not show"])
    );
    render(<PasswordStrengthIndicator password="Correct-Horse-Batt1!" />);

    await waitFor(() => {
      expect(mockZxcvbnAsync).toHaveBeenCalled();
    });

    expect(screen.queryByText("This should not show")).not.toBeInTheDocument();
  });

  // --- entropyBits override ---

  it("uses entropyBits prop instead of character-level calculation when provided", () => {
    // "Correct-Horse-Batt1!" would normally be Excellent (~138 bits from characters)
    // but with entropyBits=57 it should be Fair
    render(
      <PasswordStrengthIndicator
        password="Correct-Horse-Batt1!"
        entropyBits={57}
      />
    );
    expect(screen.getByText("passwordStrength.fair")).toBeInTheDocument();
  });

  it("entropyBits override: shows correct bar colors for overridden level", () => {
    render(
      <PasswordStrengthIndicator
        password="Correct-Horse-Batt1!"
        entropyBits={30}
      />
    );
    const bars = Array.from(screen.getByRole("meter").children);
    expect(bars[0]).toHaveClass("bg-orange-500");
    expect(bars[1]).toHaveClass("bg-orange-500");
    expect(bars[2]).toHaveClass("bg-muted");
  });

  it("uses typed-password scoring when entropyBits is not provided", () => {
    // Typed passwords use zxcvbn score (with conservative fallback while pending).
    render(<PasswordStrengthIndicator password="Correct-Horse-Batt1!" />);
    expect(screen.getByText("passwordStrength.excellent")).toBeInTheDocument();
  });

  it("pending typed-password fallback does not overrate repetitive patterns", () => {
    mockZxcvbnAsync.mockReturnValue(
      new Promise<ReturnType<typeof makeFeedbackWithScore>>(() => {
        // Keep pending so the synchronous fallback is asserted.
      })
    );

    render(<PasswordStrengthIndicator password="Aa1!Aa1!Aa1!Aa1!" />);
    expect(
      screen.queryByText("passwordStrength.strong")
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText("passwordStrength.excellent")
    ).not.toBeInTheDocument();
  });

  // --- General behavior ---

  it("does not render feedback when suggestions are empty", async () => {
    mockZxcvbnAsync.mockResolvedValue(makeFeedbackWithScore(0, []));
    render(<PasswordStrengthIndicator password="ab" />);

    await waitFor(() => {
      expect(mockZxcvbnAsync).toHaveBeenCalled();
    });

    const paragraphs = screen
      .getByText("passwordStrength.veryWeak")
      .closest(".space-y-2")
      ?.querySelectorAll("p");
    expect(paragraphs).toHaveLength(0);
  });

  it("forwards className to the root element", () => {
    render(<PasswordStrengthIndicator password="ab" className="mt-4" />);
    const label = screen.getByText("passwordStrength.veryWeak");
    expect(label.closest(".mt-4")).toBeInTheDocument();
  });

  it("meter has correct aria attributes", () => {
    render(<PasswordStrengthIndicator password="abcdefghijk" />);
    const meter = screen.getByRole("meter");
    expect(meter).toHaveAttribute("aria-valuenow", "2");
    expect(meter).toHaveAttribute("aria-valuemin", "0");
    expect(meter).toHaveAttribute("aria-valuemax", "4");
  });

  it("non-rainbow bars do not have animation-delay", () => {
    render(<PasswordStrengthIndicator password="ab" />);
    const bars = Array.from(
      screen.getByRole("meter").children
    ) as HTMLElement[];
    bars.forEach((bar) => {
      expect(bar.style.animationDelay).toBe("");
    });
  });

  it("ignores stale zxcvbn feedback when password changes rapidly", async () => {
    let resolveFirst!: (v: ReturnType<typeof makeFeedbackWithScore>) => void;
    const firstPromise = new Promise<ReturnType<typeof makeFeedbackWithScore>>(
      (res) => {
        resolveFirst = res;
      }
    );

    mockZxcvbnAsync
      .mockReturnValueOnce(firstPromise)
      .mockResolvedValue(makeFeedbackWithScore(1, []));

    const { rerender } = render(<PasswordStrengthIndicator password="ab" />);
    rerender(<PasswordStrengthIndicator password="abcdefg" />);

    await act(async () => {
      resolveFirst(makeFeedbackWithScore(0, ["Stale suggestion"]));
    });

    expect(screen.getByText("passwordStrength.weak")).toBeInTheDocument();
    expect(screen.queryByText("Stale suggestion")).not.toBeInTheDocument();
  });
});
