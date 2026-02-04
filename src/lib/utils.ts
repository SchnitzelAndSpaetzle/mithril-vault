import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function getFilenameFromPath(
  path: string | undefined,
  fallback = ""
): string {
  if (!path) return fallback;
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || fallback;
}
