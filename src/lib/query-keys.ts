// SPDX-License-Identifier: MIT

export const queryKeys = {
  database: {
    all: ["database"] as const,
    info: (dbId: string) => [...queryKeys.database.all, dbId, "info"] as const,
    config: (dbId: string) =>
      [...queryKeys.database.all, dbId, "config"] as const,
    customIcons: (dbId: string) =>
      [...queryKeys.database.all, dbId, "customIcons"] as const,
  },
  settings: {
    all: ["settings"] as const,
    preferences: () => [...queryKeys.settings.all, "preferences"] as const,
    recentDatabases: () =>
      [...queryKeys.settings.all, "recentDatabases"] as const,
  },
  entries: {
    all: ["entries"] as const,
    list: (dbId: string, groupId?: string | null) =>
      [...queryKeys.entries.all, dbId, "list", groupId ?? null] as const,
    detail: (dbId: string, id: string) =>
      [...queryKeys.entries.all, dbId, "detail", id] as const,
    tags: (dbId: string) => [...queryKeys.entries.all, dbId, "tags"] as const,
  },
  backups: {
    all: ["backups"] as const,
    list: (dbId: string) => [...queryKeys.backups.all, dbId, "list"] as const,
  },
  groups: {
    all: ["groups"] as const,
    list: (dbId: string, parentId?: string | null) =>
      [...queryKeys.groups.all, dbId, "list", parentId ?? null] as const,
    detail: (dbId: string, id: string) =>
      [...queryKeys.groups.all, dbId, "detail", id] as const,
    entryCounts: (dbId: string) =>
      [...queryKeys.groups.all, dbId, "entryCounts"] as const,
    recycleBinId: (dbId: string) =>
      [...queryKeys.groups.all, dbId, "recycleBinId"] as const,
  },
} as const;
