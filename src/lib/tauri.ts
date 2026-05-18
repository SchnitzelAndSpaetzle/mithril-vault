// SPDX-License-Identifier: MIT

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod/v4";
import type {
  AppPreferences,
  AuditEventsResponse,
  AuditFilter,
  AuditStatus,
  BackupInfo,
  BackupListEntry,
  CreateEntryData,
  CustomFieldValue,
  CustomIconMap,
  DatabaseConfig,
  DatabaseCreationOptions,
  DatabaseHeaderInfo,
  DatabaseInfo,
  Entry,
  FaviconFetchOutcome,
  GeneratedPassphrase,
  GeneratedPassword,
  Group,
  PassphraseGeneratorOptions,
  PasswordGeneratorOptions,
  RecentDatabase,
  UpdateEntryData,
} from "./types";
import {
  AppPreferencesSchema,
  AuditEventsResponseSchema,
  AuditStatusSchema,
  BackupInfoSchema,
  BackupListEntrySchema,
  CreateEntryDataSchema,
  CustomFieldValueSchema,
  CustomIconMapSchema,
  DatabaseConfigSchema,
  DatabaseCreationOptionsSchema,
  DatabaseHeaderInfoSchema,
  DatabaseInfoSchema,
  EntrySchema,
  FaviconFetchOutcomeSchema,
  GeneratedPassphraseSchema,
  GeneratedPasswordSchema,
  GroupSchema,
  PassphraseGeneratorOptionsSchema,
  PasswordGeneratorOptionsSchema,
  RecentDatabaseSchema,
  UpdateEntryDataSchema,
} from "./types";

export const KeepassIdSchema = z.guid();

const PathPasswordSchema = z.object({
  path: z.string().min(1),
  password: z.string(),
});

const PathKeyfileSchema = z.object({
  path: z.string().min(1),
  keyfilePath: z.string().min(1),
});

const PathPasswordKeyfileSchema = z.object({
  path: z.string().min(1),
  password: z.string(),
  keyfilePath: z.string().min(1),
});

const IdSchema = z.object({
  id: KeepassIdSchema,
});

const GroupIdSchema = z.object({
  groupId: KeepassIdSchema,
});

const DbIdSchema = z.object({
  dbId: z.string().min(1),
});

const CustomFieldKeySchema = z.object({
  key: z.string().min(1),
});

const NameSchema = z.object({
  name: z.string().min(1),
});

const UpdateGroupDataSchema = z
  .object({
    name: z.string().trim().min(1).optional(),
    icon: z.string().regex(/^\d+$/).optional(),
  })
  .refine((data) => data.name !== undefined || data.icon !== undefined, {
    message: "At least one field must be provided",
  });

const TagNameSchema = z.string().trim().min(1);

const CopyPasswordSchema = z.object({
  dbId: z.string().min(1),
  entryId: KeepassIdSchema,
  timeoutSecs: z.number().int().positive().optional(),
});

const CopyTextSchema = z.object({
  text: z.string(),
  timeoutSecs: z.number().int().positive().optional(),
});

const CopyProtectedFieldSchema = z.object({
  dbId: z.string().min(1),
  entryId: KeepassIdSchema,
  fieldKey: z.string().min(1),
  timeoutSecs: z.number().int().positive().optional(),
});

const ForceSchema = z.object({
  force: z.boolean().optional(),
});

const CreateDatabaseSchema = z.object({
  path: z.string().min(1),
  name: z.string().min(1),
  password: z.string().optional(),
  keyfilePath: z.string().min(1).optional(),
  options: DatabaseCreationOptionsSchema.optional(),
});

const PathOnlySchema = z.object({
  path: z.string().min(1),
});

/**
 * Database lifecycle commands for opening, creating, saving, and closing a vault.
 */
export const database = {
  async open(path: string, password: string): Promise<DatabaseInfo> {
    PathPasswordSchema.parse({ path, password });
    const result = await invoke("open_database", { path, password });
    return DatabaseInfoSchema.parse(result);
  },

  async close(dbId: string): Promise<void> {
    DbIdSchema.parse({ dbId });
    return invoke("close_database", { dbId });
  },

  async save(dbId: string): Promise<void> {
    DbIdSchema.parse({ dbId });
    return invoke("save_database", { dbId });
  },

  /**
   * Create a new KDBX4 database
   *
   * @param path - File path where the database will be saved
   * @param name - Database name (also used as root group name)
   * @param password - Optional password (required if no keyfile)
   * @param keyfilePath - Optional path to keyfile for authentication
   * @param options - Optional creation options (KDF settings, default groups, description)
   */
  async create(
    path: string,
    name: string,
    password?: string,
    keyfilePath?: string,
    options?: DatabaseCreationOptions
  ): Promise<DatabaseInfo> {
    CreateDatabaseSchema.parse({ path, name, password, keyfilePath, options });
    const result = await invoke("create_database", {
      path,
      name,
      password,
      keyfilePath,
      options,
    });
    return DatabaseInfoSchema.parse(result);
  },

  async openWithKeyfile(
    path: string,
    password: string,
    keyfilePath: string
  ): Promise<DatabaseInfo> {
    PathPasswordKeyfileSchema.parse({ path, password, keyfilePath });
    const result = await invoke("open_database_with_keyfile", {
      path,
      password,
      keyfilePath,
    });
    return DatabaseInfoSchema.parse(result);
  },

  async openWithKeyfileOnly(
    path: string,
    keyfilePath: string
  ): Promise<DatabaseInfo> {
    PathKeyfileSchema.parse({ path, keyfilePath });
    const result = await invoke("open_database_with_keyfile_only", {
      path,
      keyfilePath,
    });
    return DatabaseInfoSchema.parse(result);
  },

  /**
   * Inspect a KDBX file without requiring credentials.
   * Returns header information including version and validity status.
   *
   * @param path - File path to the KDBX database
   */
  async inspect(path: string): Promise<DatabaseHeaderInfo> {
    PathOnlySchema.parse({ path });
    const result = await invoke("inspect_database", { path });
    return DatabaseHeaderInfoSchema.parse(result);
  },

  /**
   * Get the cryptographic configuration of the currently open database.
   * Requires the database to be open (authenticated).
   */
  async getConfig(dbId: string): Promise<DatabaseConfig> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("get_database_config", { dbId });
    return DatabaseConfigSchema.parse(result);
  },

  /**
   * Get info about the currently open database.
   * Returns null if no database is open.
   */
  async getInfo(dbId: string): Promise<DatabaseInfo | null> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("get_database_info", { dbId });
    return result === null ? null : DatabaseInfoSchema.parse(result);
  },

  /**
   * Get custom icon data for an open database.
   */
  async getCustomIcons(dbId: string): Promise<CustomIconMap> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("get_custom_icons", { dbId });
    return CustomIconMapSchema.parse(result);
  },

  async lock(dbId: string): Promise<DatabaseInfo> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("lock_database", { dbId });
    return DatabaseInfoSchema.parse(result);
  },

  async unlock(dbId: string, password?: string): Promise<DatabaseInfo> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("unlock_database", { dbId, password });
    return DatabaseInfoSchema.parse(result);
  },

  async reportActivity(): Promise<void> {
    return invoke("report_activity");
  },

  /**
   * List all currently open databases.
   */
  async listOpen(): Promise<DatabaseInfo[]> {
    const result = await invoke("list_open_databases");
    return z.array(DatabaseInfoSchema).parse(result);
  },
};

/**
 * Entry CRUD operations (excluding passwords which are fetched separately).
 */
export const entries = {
  async list(dbId: string, groupId?: string): Promise<Entry[]> {
    DbIdSchema.parse({ dbId });
    if (groupId) {
      GroupIdSchema.parse({ groupId });
    }
    const result = await invoke(
      "list_entries",
      groupId ? { dbId, groupId } : { dbId }
    );
    return z.array(EntrySchema).parse(result);
  },

  async get(dbId: string, id: string): Promise<Entry> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const result = await invoke("get_entry", { dbId, id });
    return EntrySchema.parse(result);
  },

  async getPassword(dbId: string, id: string): Promise<string> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const result = await invoke("get_entry_password", { dbId, id });
    return z.string().parse(result);
  },

  async getProtectedCustomField(
    dbId: string,
    id: string,
    key: string
  ): Promise<CustomFieldValue> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    CustomFieldKeySchema.parse({ key });
    const result = await invoke("get_entry_protected_custom_field", {
      dbId,
      id,
      key,
    });
    return CustomFieldValueSchema.parse(result);
  },

  async create(
    dbId: string,
    groupId: string,
    data: CreateEntryData
  ): Promise<Entry> {
    DbIdSchema.parse({ dbId });
    GroupIdSchema.parse({ groupId });
    CreateEntryDataSchema.parse(data);
    const result = await invoke("create_entry", { dbId, groupId, data });
    return EntrySchema.parse(result);
  },

  async update(
    dbId: string,
    id: string,
    data: UpdateEntryData
  ): Promise<Entry> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    UpdateEntryDataSchema.parse(data);
    const result = await invoke("update_entry", { dbId, id, data });
    return EntrySchema.parse(result);
  },

  async move(dbId: string, id: string, targetGroupId: string): Promise<Entry> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const result = await invoke("move_entry", { dbId, id, targetGroupId });
    return EntrySchema.parse(result);
  },

  async delete(dbId: string, id: string): Promise<void> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    return invoke("delete_entry", { dbId, id });
  },

  async fetchFavicon(
    dbId: string,
    id: string,
    force?: boolean
  ): Promise<FaviconFetchOutcome> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    ForceSchema.parse({ force });
    const result = await invoke("fetch_entry_favicon", { dbId, id, force });
    return FaviconFetchOutcomeSchema.parse(result);
  },

  async clearCustomIcon(dbId: string, id: string): Promise<boolean> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const result = await invoke("clear_entry_custom_icon", { dbId, id });
    return z.boolean().parse(result);
  },

  async setCustomIcon(
    dbId: string,
    id: string,
    iconUuid: string
  ): Promise<boolean> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const result = await invoke("set_entry_custom_icon", {
      dbId,
      id,
      iconUuid,
    });
    return z.boolean().parse(result);
  },
};

/**
 * Group CRUD operations for organizing entries.
 */
export const groups = {
  async list(dbId: string): Promise<Group[]> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("list_groups", { dbId });
    return z.array(GroupSchema).parse(result);
  },

  async get(dbId: string, id: string): Promise<Group> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const result = await invoke("get_group", { dbId, id });
    return GroupSchema.parse(result);
  },

  async create(dbId: string, parentId: string, name: string): Promise<Group> {
    DbIdSchema.parse({ dbId });
    KeepassIdSchema.parse(parentId);
    NameSchema.parse({ name });
    const result = await invoke("create_group", { dbId, parentId, name });
    return GroupSchema.parse(result);
  },

  async update(
    dbId: string,
    id: string,
    data: { name?: string; icon?: string }
  ): Promise<Group> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    const parsedData = UpdateGroupDataSchema.parse(data);
    const result = await invoke("update_group", { dbId, id, data: parsedData });
    return GroupSchema.parse(result);
  },

  async rename(dbId: string, id: string, name: string): Promise<Group> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    NameSchema.parse({ name });
    const result = await invoke("rename_group", { dbId, id, name });
    return GroupSchema.parse(result);
  },

  async delete(dbId: string, id: string, recursive = false): Promise<void> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    return invoke("delete_group", { dbId, id, recursive });
  },

  async move(
    dbId: string,
    id: string,
    targetParentId?: string
  ): Promise<Group> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    if (targetParentId) {
      KeepassIdSchema.parse(targetParentId);
    }
    const result = await invoke("move_group", { dbId, id, targetParentId });
    return GroupSchema.parse(result);
  },

  async getEntryCounts(dbId: string): Promise<Record<string, number>> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("get_group_entry_counts", { dbId });
    return z.record(z.string(), z.number()).parse(result);
  },

  async getRecycleBinId(dbId: string): Promise<string | null> {
    DbIdSchema.parse({ dbId });
    const result = await invoke("get_recycle_bin_id", { dbId });
    return z.string().nullable().parse(result);
  },
};

/**
 * Bulk tag operations across all entries in the database.
 */
export const tags = {
  async rename(
    dbId: string,
    oldName: string,
    newName: string
  ): Promise<number> {
    DbIdSchema.parse({ dbId });
    const parsedOldName = TagNameSchema.parse(oldName);
    const parsedNewName = TagNameSchema.parse(newName);
    const result = await invoke("rename_tag", {
      dbId,
      oldName: parsedOldName,
      newName: parsedNewName,
    });
    return z.number().parse(result);
  },

  async delete(dbId: string, tagName: string): Promise<number> {
    DbIdSchema.parse({ dbId });
    const parsedTagName = TagNameSchema.parse(tagName);
    const result = await invoke("delete_tag", { dbId, tagName: parsedTagName });
    return z.number().parse(result);
  },
};

/**
 * Password generation commands backed by the Rust generator.
 */
export const generator = {
  async generate(
    options: PasswordGeneratorOptions
  ): Promise<GeneratedPassword> {
    PasswordGeneratorOptionsSchema.parse(options);
    const result = await invoke("generate_password", { options });
    return GeneratedPasswordSchema.parse(result);
  },

  async generatePassphrase(
    options: PassphraseGeneratorOptions
  ): Promise<GeneratedPassphrase> {
    PassphraseGeneratorOptionsSchema.parse(options);
    const result = await invoke("generate_passphrase", { options });
    return GeneratedPassphraseSchema.parse(result);
  },
};

/**
 * Clipboard actions for sensitive data (copy and clear).
 */
export const clipboard = {
  async copyText(text: string, timeoutSecs?: number): Promise<void> {
    CopyTextSchema.parse({ text, timeoutSecs });
    return invoke("copy_text_to_clipboard", { text, timeoutSecs });
  },

  async copyPassword(
    dbId: string,
    entryId: string,
    timeoutSecs?: number
  ): Promise<void> {
    CopyPasswordSchema.parse({ dbId, entryId, timeoutSecs });
    return invoke("copy_password_to_clipboard", { dbId, entryId, timeoutSecs });
  },

  async copyProtectedField(
    dbId: string,
    entryId: string,
    fieldKey: string,
    timeoutSecs?: number
  ): Promise<void> {
    CopyProtectedFieldSchema.parse({ dbId, entryId, fieldKey, timeoutSecs });
    return invoke("copy_protected_field_to_clipboard", {
      dbId,
      entryId,
      fieldKey,
      timeoutSecs,
    });
  },

  async clear(): Promise<void> {
    return invoke("clear_clipboard");
  },
};

const OutputPathSchema = z.object({
  outputPath: z.string().min(1),
});

/**
 * Keyfile generation commands.
 */
export const keyfile = {
  /**
   * Generate a new KeePass 2.x compatible keyfile (.keyx format).
   *
   * The keyfile contains 32 bytes of cryptographically random data
   * in an XML format compatible with KeePass 2.x and other implementations.
   *
   * @param outputPath - Path where the keyfile will be saved
   */
  async generate(outputPath: string): Promise<void> {
    OutputPathSchema.parse({ outputPath });
    return invoke("generate_keyfile", { outputPath });
  },
};

/**
 * Application settings including recent databases and preferences.
 */
export const settings = {
  async getRecentDatabases(): Promise<RecentDatabase[]> {
    const result = await invoke("get_recent_databases");
    return z.array(RecentDatabaseSchema).parse(result);
  },

  async getPreferences(): Promise<AppPreferences> {
    const result = await invoke("get_app_preferences");
    return AppPreferencesSchema.parse(result);
  },

  async updatePreferences(newPreferences: AppPreferences): Promise<void> {
    AppPreferencesSchema.parse(newPreferences);
    return invoke("update_app_preferences", { newPreferences });
  },

  async resetPreferences(): Promise<AppPreferences> {
    const result = await invoke("reset_app_preferences");
    return AppPreferencesSchema.parse(result);
  },

  async addRecentDatabase(path: string, keyfilePath?: string): Promise<void> {
    PathOnlySchema.parse({ path });
    return invoke("add_recent_database", { path, keyfilePath });
  },

  async getKeyfileForDatabase(path: string): Promise<string | null> {
    PathOnlySchema.parse({ path });
    const result = await invoke("get_keyfile_for_database", { path });
    return z.string().nullable().parse(result);
  },

  async removeRecentDatabase(path: string): Promise<void> {
    PathOnlySchema.parse({ path });
    return invoke("remove_recent_database", { path });
  },

  async clearRecentDatabases(): Promise<void> {
    return invoke("clear_recent_databases");
  },
};

/**
 * Backup snapshot listing and deletion. Listing is scoped to one Vault on
 * disk (by path); deletion is gated by the backend to paths inside an
 * open vault's backup directory.
 */
export const backups = {
  async list(databasePath: string): Promise<BackupListEntry[]> {
    PathOnlySchema.parse({ path: databasePath });
    const result = await invoke("list_backups", { databasePath });
    return z.array(BackupListEntrySchema).parse(result);
  },

  async delete(backupPath: string): Promise<void> {
    PathOnlySchema.parse({ path: backupPath });
    return invoke("delete_backup", { backupPath });
  },

  async createManual(databasePath: string): Promise<BackupInfo> {
    PathOnlySchema.parse({ path: databasePath });
    const result = await invoke("create_manual_backup", { databasePath });
    return BackupInfoSchema.parse(result);
  },

  async restore(backupPath: string): Promise<void> {
    PathOnlySchema.parse({ path: backupPath });
    return invoke("restore_backup", { backupPath });
  },
};

export const audit = {
  /// Lists the audit events recorded on this device for the given Vault,
  /// newest-first, plus a session-wide `degraded` flag. `filter` is
  /// accepted but currently ignored on the backend — wire shape is in
  /// place so a UI filter can be added without a command rename.
  async list(
    vaultPath: string,
    filter?: AuditFilter
  ): Promise<AuditEventsResponse> {
    const result = await invoke("get_audit_events", {
      vaultPath,
      filter: filter ?? null,
    });
    return AuditEventsResponseSchema.parse(result);
  },

  /// Truncates the per-Vault audit log and leaves behind a single
  /// `auditCleared` event so the wipe shows up in the panel. The backend
  /// rewrites the file atomically, so on failure the original log is
  /// preserved and this rejects with the backend error.
  async clear(vaultPath: string): Promise<void> {
    await invoke("clear_audit_log", { vaultPath });
  },

  /// Snapshot of audit subsystem runtime state: master gate + session-
  /// wide `degraded` flag. Used by the Settings panel header to render
  /// the degraded indicator independently from the per-Vault event read.
  async getStatus(): Promise<AuditStatus> {
    const result = await invoke("get_audit_status");
    return AuditStatusSchema.parse(result);
  },
};

export const windowProtection = {
  async setProtected(enabled: boolean): Promise<void> {
    return invoke("set_window_content_protected", { enabled });
  },

  async isSupported(): Promise<boolean> {
    const result = await invoke("get_window_content_protection_supported");
    return z.boolean().parse(result);
  },
};
