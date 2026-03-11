// SPDX-License-Identifier: MIT

import { z } from "zod/v4";

export const DatabaseInfoSchema = z.object({
  name: z.string(),
  path: z.string(),
  isModified: z.boolean(),
  isLocked: z.boolean(),
  rootGroupId: z.string(),
  version: z.string(),
});
export type DatabaseInfo = z.infer<typeof DatabaseInfoSchema>;

export const LockFileInfoSchema = z.object({
  pid: z.number(),
  application: z.string(),
  version: z.string(),
  openedAt: z.string(),
  hostname: z.string(),
});
export type LockFileInfo = z.infer<typeof LockFileInfoSchema>;

export const LockStatusSchema = z.discriminatedUnion("status", [
  z.object({ status: z.literal("available") }),
  z.object({ status: z.literal("lockedByCurrentProcess") }),
  z.object({
    status: z.literal("lockedByOtherProcess"),
    info: LockFileInfoSchema,
  }),
  z.object({ status: z.literal("staleLock"), info: LockFileInfoSchema }),
]);
export type LockStatus = z.infer<typeof LockStatusSchema>;

export const CustomFieldMetaSchema = z.object({
  key: z.string(),
  isProtected: z.boolean(),
});
export type CustomFieldMeta = z.infer<typeof CustomFieldMetaSchema>;

export const EntrySchema = z.object({
  id: z.string(),
  groupId: z.string(),
  title: z.string(),
  username: z.string(),
  url: z.string().nullable().optional(),
  notes: z.string().nullable().optional(),
  iconId: z.number().int().optional(),
  customIconUuid: z.string().nullable().optional(),
  tags: z.array(z.string()),
  customFields: z.record(z.string(), z.string()),
  customFieldMeta: z.array(CustomFieldMetaSchema),
  createdAt: z.string(),
  modifiedAt: z.string(),
  accessedAt: z.string(),
});
export type Entry = z.infer<typeof EntrySchema>;

export interface Group {
  id: string;
  parentId: string | null;
  name: string;
  icon: string | null;
  customIconUuid: string | null;
  children: Group[];
}

export const GroupSchema: z.ZodType<Group> = z.lazy(() =>
  z.object({
    id: z.string(),
    parentId: z.string().nullable(),
    name: z.string(),
    icon: z.string().nullable(),
    customIconUuid: z.string().nullable(),
    children: z.array(GroupSchema),
  })
);

export const CustomIconMapSchema = z.record(z.string(), z.string());
export type CustomIconMap = z.infer<typeof CustomIconMapSchema>;

export const PasswordGeneratorOptionsSchema = z.object({
  length: z.number().int().min(1).max(128),
  uppercase: z.boolean(),
  lowercase: z.boolean(),
  numbers: z.boolean(),
  symbols: z.boolean(),
  excludeAmbiguous: z.boolean(),
  excludeChars: z.string().optional(),
});
export type PasswordGeneratorOptions = z.infer<
  typeof PasswordGeneratorOptionsSchema
>;

export const CreateEntryDataSchema = z.object({
  title: z.string().min(1),
  username: z.string(),
  password: z.string(),
  url: z.string().optional(),
  notes: z.string().optional(),
  iconId: z.number().int().optional(),
  tags: z.array(z.string()).optional(),
  customFields: z.record(z.string(), z.string()).optional(),
  protectedCustomFields: z.record(z.string(), z.string()).optional(),
});
export type CreateEntryData = z.infer<typeof CreateEntryDataSchema>;

export const UpdateEntryDataSchema = z.object({
  title: z.string().min(1).optional(),
  username: z.string().optional(),
  password: z.string().optional(),
  url: z.string().optional(),
  notes: z.string().optional(),
  iconId: z.number().int().optional(),
  tags: z.array(z.string()).optional(),
  customFields: z.record(z.string(), z.string()).optional(),
  protectedCustomFields: z.record(z.string(), z.string()).optional(),
});
export type UpdateEntryData = z.infer<typeof UpdateEntryDataSchema>;

export const CustomFieldValueSchema = z.object({
  key: z.string(),
  value: z.string(),
});
export type CustomFieldValue = z.infer<typeof CustomFieldValueSchema>;

export const DatabaseCreationOptionsSchema = z.object({
  description: z.string().optional(),
  createDefaultGroups: z.boolean().optional(),
  kdfMemory: z.number().int().positive().optional(),
  kdfIterations: z.number().int().positive().optional(),
  kdfParallelism: z.number().int().positive().optional(),
});
export type DatabaseCreationOptions = z.infer<
  typeof DatabaseCreationOptionsSchema
>;

export const DatabaseHeaderInfoSchema = z.object({
  version: z.string(),
  isValidKdbx: z.boolean(),
  isSupported: z.boolean(),
  path: z.string(),
});
export type DatabaseHeaderInfo = z.infer<typeof DatabaseHeaderInfoSchema>;

export const OuterCipherSchema = z.enum(["aes256", "twofish", "chaCha20"]);
export type OuterCipher = z.infer<typeof OuterCipherSchema>;

export const InnerCipherSchema = z.enum(["plain", "salsa20", "chaCha20"]);
export type InnerCipher = z.infer<typeof InnerCipherSchema>;

export const CompressionSchema = z.enum(["none", "gZip"]);
export type Compression = z.infer<typeof CompressionSchema>;

export const KdfSettingsSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("aesKdf"),
    rounds: z.number().int().positive(),
  }),
  z.object({
    type: z.literal("argon2d"),
    memory: z.number().int().positive(),
    iterations: z.number().int().positive(),
    parallelism: z.number().int().positive(),
  }),
  z.object({
    type: z.literal("argon2id"),
    memory: z.number().int().positive(),
    iterations: z.number().int().positive(),
    parallelism: z.number().int().positive(),
  }),
]);
export type KdfSettings = z.infer<typeof KdfSettingsSchema>;

export const DatabaseConfigSchema = z.object({
  version: z.string(),
  outerCipher: OuterCipherSchema,
  innerCipher: InnerCipherSchema,
  compression: CompressionSchema,
  kdf: KdfSettingsSchema,
});
export type DatabaseConfig = z.infer<typeof DatabaseConfigSchema>;

export const RecentDatabaseSchema = z.object({
  path: z.string(),
  keyfilePath: z.string().nullable(),
  lastOpened: z.string(),
});
export type RecentDatabase = z.infer<typeof RecentDatabaseSchema>;

export const StartupBehaviorSchema = z.enum([
  "showUnlockScreen",
  "openLastDatabase",
  "openDefaultDatabase",
]);
export type StartupBehavior = z.infer<typeof StartupBehaviorSchema>;

export const ThemePreferenceSchema = z.enum(["system", "light", "dark"]);
export type ThemePreference = z.infer<typeof ThemePreferenceSchema>;

export const EntryListColumnsSchema = z.object({
  username: z.boolean(),
  url: z.boolean(),
  modifiedAt: z.boolean(),
  tags: z.boolean(),
});
export type EntryListColumns = z.infer<typeof EntryListColumnsSchema>;

export const GeneralSettingsSchema = z.object({
  language: z.string().min(1),
  startupBehavior: StartupBehaviorSchema,
  defaultDatabasePath: z.string().nullable(),
});
export type GeneralSettings = z.infer<typeof GeneralSettingsSchema>;

export const SecuritySettingsSchema = z.object({
  autoLockTimeout: z.number().int().positive(),
  clipboardClearTimeout: z.number().int().positive(),
  showPasswordByDefault: z.boolean(),
  minimizeToTray: z.boolean(),
  startMinimized: z.boolean(),
});
export type SecuritySettings = z.infer<typeof SecuritySettingsSchema>;

export const AppearanceSettingsSchema = z.object({
  theme: ThemePreferenceSchema,
  colorPreset: z.string().default("default"),
  fontSize: z.number().int().min(10).max(24),
  entryListColumns: EntryListColumnsSchema,
});
export type AppearanceSettings = z.infer<typeof AppearanceSettingsSchema>;

export const BrowserIntegrationSettingsSchema = z.object({
  enabled: z.boolean(),
  allowedSites: z.array(z.string().min(1)),
});
export type BrowserIntegrationSettings = z.infer<
  typeof BrowserIntegrationSettingsSchema
>;

export const AdvancedSettingsSchema = z.object({
  debugMode: z.boolean(),
  dataLocation: z.string(),
});
export type AdvancedSettings = z.infer<typeof AdvancedSettingsSchema>;

export const AppPreferencesSchema = z.object({
  general: GeneralSettingsSchema,
  security: SecuritySettingsSchema,
  appearance: AppearanceSettingsSchema,
  browserIntegration: BrowserIntegrationSettingsSchema,
  advanced: AdvancedSettingsSchema,
});
export type AppPreferences = z.infer<typeof AppPreferencesSchema>;

export const AppSettingsSchema = z.object({
  language: z.string().min(1),
  startupBehavior: StartupBehaviorSchema,
  defaultDatabasePath: z.string().nullable(),
  autoLockTimeout: z.number().int(),
  clipboardClearTimeout: z.number().int(),
  showPasswordByDefault: z.boolean(),
  minimizeToTray: z.boolean(),
  startMinimized: z.boolean(),
  theme: ThemePreferenceSchema,
  colorPreset: z.string().default("default"),
  fontSize: z.number().int().min(10).max(24),
  entryListShowUsername: z.boolean(),
  entryListShowUrl: z.boolean(),
  entryListShowModifiedAt: z.boolean(),
  entryListShowTags: z.boolean(),
  browserIntegrationEnabled: z.boolean(),
  browserAllowedSites: z.array(z.string()),
  debugMode: z.boolean(),
  recentDatabases: z.array(RecentDatabaseSchema),
});
export type AppSettings = z.infer<typeof AppSettingsSchema>;

export type GroupEntryCounts = Record<string, number>;

export const EntrySortFieldSchema = z.enum([
  "title",
  "username",
  "url",
  "modifiedAt",
  "createdAt",
]);
export type EntrySortField = z.infer<typeof EntrySortFieldSchema>;

export const SortOrderSchema = z.enum(["asc", "desc"]);
export type SortOrder = z.infer<typeof SortOrderSchema>;
