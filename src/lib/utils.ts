// SPDX-License-Identifier: GPL-3.0-or-later

import { z } from "zod/v4";
export { debounce, throttle, cloneDeep, isEqual, pick, omit } from "lodash-es";
export { default as dayjs } from "dayjs";

export function cn(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(" ");
}

const TruncateArgsSchema = z.tuple([z.string(), z.number().int().min(4)]);

export function truncate(str: string, maxLength: number): string {
  TruncateArgsSchema.parse([str, maxLength]);
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 3) + "...";
}
