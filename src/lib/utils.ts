// SPDX-License-Identifier: MIT

export { debounce, throttle, cloneDeep, isEqual, pick, omit } from "lodash-es";
export { default as dayjs } from "dayjs";

export function cn(...classes: (string | undefined | null | false)[]): string {
  return classes.filter(Boolean).join(" ");
}

export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 3) + "...";
}
