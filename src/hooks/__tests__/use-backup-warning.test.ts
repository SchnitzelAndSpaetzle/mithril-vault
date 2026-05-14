// SPDX-License-Identifier: MIT

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook } from "@testing-library/react";

const mockListen = vi.fn();
const mockToast = vi.fn();
const mockToastWarning = vi.fn();
const mockT = vi.fn((key: string, opts?: Record<string, unknown>) =>
  opts
    ? Object.entries(opts).reduce<string>(
        (acc, [k, v]) => acc.replace(`{{${k}}}`, String(v)),
        // Resolve against the production English source so the test
        // verifies what users actually see, not the i18n key.
        "Open-side backup failed for {{path}}: {{reason}}. The vault is still unlocked."
      )
    : key
);

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

vi.mock("sonner", () => ({
  toast: Object.assign((...args: unknown[]) => mockToast(...args), {
    warning: (...args: unknown[]) => mockToastWarning(...args),
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: mockT,
    i18n: { language: "en", changeLanguage: vi.fn() },
  }),
}));

import { useBackupWarning } from "../use-backup-warning";

type Listener = (event: { payload: { path: string; reason: string } }) => void;

describe("useBackupWarning", () => {
  beforeEach(() => {
    mockListen.mockReset();
    mockToast.mockReset();
    mockToastWarning.mockReset();
    mockListen.mockReturnValue(Promise.resolve(vi.fn()));
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("subscribes to the backup-warning event on mount", () => {
    renderHook(() => useBackupWarning());

    expect(mockListen).toHaveBeenCalledWith(
      "backup-warning",
      expect.any(Function)
    );
  });

  it("renders a non-blocking warning toast (not error/modal) with payload context", () => {
    // Acceptance criterion for #193: open-side backup failures must surface
    // as a non-blocking toast — never a modal and never an error dialog.
    renderHook(() => useBackupWarning());

    const [[, listener]] = mockListen.mock.calls as [[string, Listener]];
    listener({
      payload: {
        path: "/Volumes/Backup/vault.kdbx",
        reason: "Permission denied",
      },
    });

    expect(mockToastWarning).toHaveBeenCalledTimes(1);
    const [message] = mockToastWarning.mock.calls[0] as [string];
    expect(message).toContain("/Volumes/Backup/vault.kdbx");
    expect(message).toContain("Permission denied");
  });
});
