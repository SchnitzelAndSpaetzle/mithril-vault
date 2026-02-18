// SPDX-License-Identifier: MIT

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod/v4";
import type {
  AppSettings,
  CreateEntryData,
  CustomFieldValue,
  CustomIconMap,
  DatabaseConfig,
  DatabaseCreationOptions,
  DatabaseHeaderInfo,
  DatabaseInfo,
  Entry,
  Group,
  LockStatus,
  PasswordGeneratorOptions,
  UpdateEntryData,
} from "./types";
import {
  AppSettingsSchema,
  CreateEntryDataSchema,
  CustomFieldValueSchema,
  CustomIconMapSchema,
  DatabaseConfigSchema,
  DatabaseCreationOptionsSchema,
  DatabaseHeaderInfoSchema,
  DatabaseInfoSchema,
  EntrySchema,
  GroupSchema,
  LockStatusSchema,
  PasswordGeneratorOptionsSchema,
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

const CopyPasswordSchema = z.object({
  dbId: z.string().min(1),
  entryId: KeepassIdSchema,
  timeoutSecs: z.number().int().positive().optional(),
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

  /**
   * List all currently open databases.
   */
  async listOpen(): Promise<DatabaseInfo[]> {
    const result = await invoke("list_open_databases");
    return z.array(DatabaseInfoSchema).parse(result);
  },

  /**
   * Get the lock status for a database file without opening it.
   * Can be used to check if a database is locked before attempting to open it.
   *
   * @param path - File path to the KDBX database
   */
  async getLockStatus(path: string): Promise<LockStatus> {
    PathOnlySchema.parse({ path });
    const result = await invoke("get_lock_status", { path });
    return LockStatusSchema.parse(result);
  },

  /**
   * Force remove a lock file for recovery purposes.
   *
   * WARNING: Only use this when:
   * - The lock is known to be stale (process crashed)
   * - The user has confirmed they want to force unlock
   *
   * Using this on an actively locked database may cause data corruption.
   *
   * @param path - File path to the KDBX database
   */
  async forceUnlock(path: string): Promise<void> {
    PathOnlySchema.parse({ path });
    return invoke("force_unlock_database", { path });
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

  async delete(dbId: string, id: string): Promise<void> {
    DbIdSchema.parse({ dbId });
    IdSchema.parse({ id });
    return invoke("delete_entry", { dbId, id });
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
 * Password generation commands backed by the Rust generator.
 */
export const generator = {
  async generate(options: PasswordGeneratorOptions): Promise<string> {
    PasswordGeneratorOptionsSchema.parse(options);
    const result = await invoke("generate_password", { options });
    return z.string().parse(result);
  },
};

/**
 * Clipboard actions for sensitive data (copy and clear).
 */
export const clipboard = {
  async copyPassword(
    dbId: string,
    entryId: string,
    timeoutSecs?: number
  ): Promise<void> {
    CopyPasswordSchema.parse({ dbId, entryId, timeoutSecs });
    return invoke("copy_password_to_clipboard", { dbId, entryId, timeoutSecs });
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
  async get(): Promise<AppSettings> {
    const result = await invoke("get_settings");
    return AppSettingsSchema.parse(result);
  },

  async update(newSettings: AppSettings): Promise<void> {
    AppSettingsSchema.parse(newSettings);
    return invoke("update_settings", { newSettings });
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
