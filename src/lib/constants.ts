// SPDX-License-Identifier: GPL-3.0-or-later

export const APP_NAME = "MithrilVault";

export const CLIPBOARD_CLEAR_TIMEOUT_MS = 30_000;

export const AUTO_LOCK_TIMEOUT_MS = 5 * 60 * 1000;

export const PASSWORD_GENERATOR_DEFAULTS = {
  length: 20,
  uppercase: true,
  lowercase: true,
  numbers: true,
  symbols: true,
  excludeAmbiguous: false,
} as const;

export const KDBX_FILE_EXTENSION = ".kdbx";
export const KEY_FILE_EXTENSIONS = [".key", ".keyx"] as const;
