// SPDX-License-Identifier: MIT

export const queryKeys = {
  database: {
    all: ["database"] as const,
    info: () => [...queryKeys.database.all, "info"] as const,
  },
  settings: {
    all: ["settings"] as const,
    recentDatabases: () =>
      [...queryKeys.settings.all, "recentDatabases"] as const,
  },
  entries: {
    all: ["entries"] as const,
    list: (groupId?: string | null) =>
      [...queryKeys.entries.all, "list", groupId ?? null] as const,
    detail: (id: string) => [...queryKeys.entries.all, "detail", id] as const,
  },
  groups: {
    all: ["groups"] as const,
    list: (parentId?: string | null) =>
      [...queryKeys.groups.all, "list", parentId ?? null] as const,
    detail: (id: string) => [...queryKeys.groups.all, "detail", id] as const,
  },
} as const;
