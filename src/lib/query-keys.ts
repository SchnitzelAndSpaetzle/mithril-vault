// SPDX-License-Identifier: MIT

export const queryKeys = {
  database: {
    all: ["database"] as const,
    info: (dbId: string) => [...queryKeys.database.all, dbId, "info"] as const,
  },
  settings: {
    all: ["settings"] as const,
    recentDatabases: () =>
      [...queryKeys.settings.all, "recentDatabases"] as const,
  },
  entries: {
    all: ["entries"] as const,
    list: (dbId: string, groupId?: string | null) =>
      [...queryKeys.entries.all, dbId, "list", groupId ?? null] as const,
    detail: (dbId: string, id: string) =>
      [...queryKeys.entries.all, dbId, "detail", id] as const,
  },
  groups: {
    all: ["groups"] as const,
    list: (dbId: string, parentId?: string | null) =>
      [...queryKeys.groups.all, dbId, "list", parentId ?? null] as const,
    detail: (dbId: string, id: string) =>
      [...queryKeys.groups.all, dbId, "detail", id] as const,
  },
} as const;
