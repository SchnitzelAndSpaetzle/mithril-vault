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
  iconId: z.number().int().nullable().optional(),
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

export const CustomIconDataSchema = z.object({
  mimeType: z.string().min(1),
  data: z.string(),
});
export type CustomIconData = z.infer<typeof CustomIconDataSchema>;

export const CustomIconMapSchema = z.record(z.string(), CustomIconDataSchema);
export type CustomIconMap = z.infer<typeof CustomIconMapSchema>;

export const FaviconFetchOutcomeSchema = z.enum([
  "updated",
  "unchanged",
  "notFound",
]);
export type FaviconFetchOutcome = z.infer<typeof FaviconFetchOutcomeSchema>;

export const PasswordGeneratorOptionsSchema = z.object({
  length: z.number().int().min(1).max(128),
  uppercase: z.boolean(),
  lowercase: z.boolean(),
  numbers: z.boolean(),
  symbols: z.boolean(),
  excludeAmbiguous: z.boolean(),
  excludeChars: z.string().optional(),
  minNumbers: z.number().int().min(0).optional(),
  minSymbols: z.number().int().min(0).optional(),
});
export type PasswordGeneratorOptions = z.infer<
  typeof PasswordGeneratorOptionsSchema
>;

export const PassphraseGeneratorOptionsSchema = z.object({
  wordCount: z.number().int().min(1).max(20),
  separator: z.string(),
  capitalize: z.boolean(),
  includeNumber: z.boolean(),
});
export type PassphraseGeneratorOptions = z.infer<
  typeof PassphraseGeneratorOptionsSchema
>;

export const GeneratedPasswordSchema = z.object({
  password: z.string(),
  entropyBits: z.number(),
});
export type GeneratedPassword = z.infer<typeof GeneratedPasswordSchema>;

export const GeneratedPassphraseSchema = z.object({
  passphrase: z.string(),
  entropyBits: z.number(),
});
export type GeneratedPassphrase = z.infer<typeof GeneratedPassphraseSchema>;

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
  autoLockTimeout: z.number().int().nonnegative(),
  clipboardClearTimeout: z.number().int().positive(),
  clearClipboardOnLock: z.boolean(),
  showClipboardCountdown: z.boolean(),
  showPasswordByDefault: z.boolean(),
  minimizeToTray: z.boolean(),
  startMinimized: z.boolean(),
  preventScreenCapture: z.boolean(),
  autoDownloadFavicons: z.boolean(),
  allowThirdPartyFaviconFallbacks: z.boolean(),
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

export const BackupSettingsSchema = z.object({
  enabled: z.boolean(),
  maxVersions: z.number().int().min(1).max(500),
  // Optional absolute-path override for snapshot storage. Serde on the
  // backend emits `null` when unset (the field exists but is None) and
  // omits it when the directory key is missing entirely; accept both.
  directory: z.string().nullable().optional(),
  // Opt-in open-side snapshot (#193). Defaults to false so existing installs
  // do not start taking extra snapshots until the user enables it.
  onOpen: z.boolean().default(false),
});
export type BackupSettings = z.infer<typeof BackupSettingsSchema>;
export const BACKUP_MAX_VERSIONS_PRESETS = [5, 10, 25, 50, 100] as const;
export const DEFAULT_BACKUP_MAX_VERSIONS = 10;

/// Audit-log preferences. `enabled` is the master gate (off => the backend
/// `AuditService::record` becomes a no-op, existing log file untouched).
/// `retentionDays` is bounded `1..=365` on the backend boundary; the UI
/// must mirror the same range so an invalid value never leaves the form.
export const AppPreferencesAuditSchema = z.object({
  enabled: z.boolean(),
  retentionDays: z.number().int().min(1).max(365),
});
export type AuditPreferences = z.infer<typeof AppPreferencesAuditSchema>;
export const DEFAULT_AUDIT_RETENTION_DAYS = 90;

export const AppPreferencesSchema = z.object({
  general: GeneralSettingsSchema,
  security: SecuritySettingsSchema,
  appearance: AppearanceSettingsSchema,
  browserIntegration: BrowserIntegrationSettingsSchema,
  advanced: AdvancedSettingsSchema,
  backups: BackupSettingsSchema,
  audit: AppPreferencesAuditSchema,
});
export type AppPreferences = z.infer<typeof AppPreferencesSchema>;

export type GroupEntryCounts = Record<string, number>;

/// Settings → Backups list row. Snapshot kind comes from filename pattern
/// detection on the backend; auto vs manual is what drives the row badge.
export const BackupKindSchema = z.enum(["auto", "manual"]);
export type BackupKind = z.infer<typeof BackupKindSchema>;

export const BackupListEntrySchema = z.object({
  path: z.string().min(1),
  timestamp: z.string().min(1),
  sizeBytes: z.number().int().nonnegative(),
  kind: BackupKindSchema,
});
export type BackupListEntry = z.infer<typeof BackupListEntrySchema>;

/// Result of a manual backup snapshot — the path of the file just written.
/// Used by the Settings → "Create backup now" toast.
export const BackupInfoSchema = z.object({
  path: z.string().min(1),
});
export type BackupInfo = z.infer<typeof BackupInfoSchema>;

/// Audit log event — security-relevant action recorded on this device.
/// Each kind plugs into the same row shape so the panel does not need
/// per-kind layouts; optional fields (`attemptCount`, `reason`, `entryId`)
/// carry the kind-specific payload.
export const AuditEventKindSchema = z.enum([
  "vaultUnlockFailed",
  "vaultOpened",
  "vaultLocked",
  "entryPasswordRevealed",
  "entryPasswordCopied",
  "entryProtectedFieldRevealed",
  "preferencesSecurityChanged",
  "auditCleared",
]);
export type AuditEventKind = z.infer<typeof AuditEventKindSchema>;

/// Why a Vault transitioned from unlocked to locked. Mirrors the backend
/// `services::audit::format::Reason` enum on the camelCase wire.
export const AuditReasonSchema = z.enum([
  "manual",
  "autoLock",
  "appQuit",
  "screenLock",
]);
export type AuditReason = z.infer<typeof AuditReasonSchema>;

/// Allowlisted App Preference leaves whose flips surface as
/// `preferencesSecurityChanged` events. Pinning this as a union (not a
/// free string) ties the wire identifier to the i18n key registry, so a
/// new allowlist entry on the backend fails the frontend type-check
/// until its label is added under `audit.settingName.*`.
export const SecuritySettingChangeNameSchema = z.enum([
  "security.clipboardClearTimeout",
  "security.preventScreenCapture",
  "security.autoDownloadFavicons",
  "security.allowThirdPartyFaviconFallbacks",
  "security.autoLockTimeout",
  "audit.enabled",
  "audit.retentionDays",
]);
export type SecuritySettingChangeName = z.infer<
  typeof SecuritySettingChangeNameSchema
>;

export const AuditEventSchema = z.object({
  kind: AuditEventKindSchema,
  timestamp: z.string().min(1),
  attemptCount: z.number().int().positive().nullable().optional(),
  reason: AuditReasonSchema.nullable().optional(),
  entryId: z.string().min(1).nullable().optional(),
  /// Dot-pathed App Preference leaf for `preferencesSecurityChanged`
  /// events (e.g. `security.preventScreenCapture`). Old/new values are
  /// deliberately absent from the wire — the on-disk log records THAT a
  /// flip happened, not what it flipped to.
  settingName: SecuritySettingChangeNameSchema.nullable().optional(),
});
export type AuditEvent = z.infer<typeof AuditEventSchema>;

export const AuditFilterSchema = z.object({
  kinds: z.array(AuditEventKindSchema).optional(),
});
export type AuditFilter = z.infer<typeof AuditFilterSchema>;

/// Response from the `get_audit_events` IPC. `degraded` is the
/// session-wide flag set by the backend whenever an audit record or read
/// has failed internally. The UI uses it to surface a "some history may
/// be missing" banner so a soft failure is never visually identical to
/// "no events yet".
export const AuditEventsResponseSchema = z.object({
  events: z.array(AuditEventSchema),
  degraded: z.boolean(),
});
export type AuditEventsResponse = z.infer<typeof AuditEventsResponseSchema>;

/// Snapshot of the audit subsystem's runtime state for the Settings panel
/// header. `enabled` mirrors the master gate (`audit.enabled` preference);
/// `degraded` is the session-wide flag set by the backend whenever any
/// audit record/read has failed internally. The header indicator only
/// clears on app restart because `degraded` lives in process memory.
export const AuditStatusSchema = z.object({
  enabled: z.boolean(),
  degraded: z.boolean(),
});
export type AuditStatus = z.infer<typeof AuditStatusSchema>;

/// Namespaced enum of Password Health findings. Wire shape mirrors the
/// backend `FindingKindDto` — additions land here when new check kinds
/// ship in follow-up slices.
export const FindingKindSchema = z.enum(["password.expired"]);
export type FindingKind = z.infer<typeof FindingKindSchema>;

export const FindingSchema = z.object({
  entryId: z.string().min(1),
  kind: FindingKindSchema,
});
export type Finding = z.infer<typeof FindingSchema>;

export const HealthTotalsSchema = z.object({
  critical: z.number().int().nonnegative(),
  high: z.number().int().nonnegative(),
  healthy: z.number().int().nonnegative(),
  total: z.number().int().nonnegative(),
});
export type HealthTotals = z.infer<typeof HealthTotalsSchema>;

export const ReuseGroupSchema = z.object({
  passwordHash: z.string().min(1),
  entryIds: z.array(z.string().min(1)),
});
export type ReuseGroup = z.infer<typeof ReuseGroupSchema>;

/// Per-Vault Password Health snapshot. `score` is `null` on an empty
/// Vault (no in-scope Entries) and a `0..=100` integer otherwise.
/// `reuseGroups` is on the wire from day one but is empty in the
/// expired-only tracer slice.
export const PasswordHealthReportSchema = z.object({
  score: z.number().int().min(0).max(100).nullable(),
  findings: z.array(FindingSchema),
  totals: HealthTotalsSchema,
  reuseGroups: z.array(ReuseGroupSchema),
});
export type PasswordHealthReport = z.infer<typeof PasswordHealthReportSchema>;

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
