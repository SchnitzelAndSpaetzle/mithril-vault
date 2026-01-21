// SPDX-License-Identifier: MIT

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod/v4";
import type {
  CreateEntryData,
  CustomFieldValue,
  DatabaseConfig,
  DatabaseCreationOptions,
  DatabaseHeaderInfo,
  DatabaseInfo,
  Entry,
  Group,
  PasswordGeneratorOptions,
  UpdateEntryData,
} from "./types";
import {
  CreateEntryDataSchema,
  CustomFieldValueSchema,
  DatabaseConfigSchema,
  DatabaseCreationOptionsSchema,
  DatabaseHeaderInfoSchema,
  DatabaseInfoSchema,
  EntrySchema,
  GroupSchema,
  PasswordGeneratorOptionsSchema,
  UpdateEntryDataSchema,
} from "./types";

const PathPasswordSchema = z.object({
  path: z.string().min(1),
  password: z.string().min(8),
});

const PathKeyfileSchema = z.object({
  path: z.string().min(1),
  keyfilePath: z.string().min(1),
});

const PathPasswordKeyfileSchema = z.object({
  path: z.string().min(1),
  password: z.string().min(8),
  keyfilePath: z.string().min(1),
});

const IdSchema = z.object({
  id: z.uuid(),
});

const GroupIdSchema = z.object({
  groupId: z.string().uuid(),
});

const CustomFieldKeySchema = z.object({
  key: z.string().min(1),
});

const NameSchema = z.object({
  name: z.string().min(1),
});

const CopyPasswordSchema = z.object({
  entryId: z.uuid(),
  timeoutMs: z.number().int().positive().optional(),
});

const CreateDatabaseSchema = z.object({
  path: z.string().min(1),
  name: z.string().min(1),
  password: z.string().min(8).optional(),
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

  async close(): Promise<void> {
    return invoke("close_database");
  },

  async save(): Promise<void> {
    return invoke("save_database");
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
  async getConfig(): Promise<DatabaseConfig> {
    const result = await invoke("get_database_config");
    return DatabaseConfigSchema.parse(result);
  },
};

/**
 * Entry CRUD operations (excluding passwords which are fetched separately).
 */
export const entries = {
  async list(groupId?: string): Promise<Entry[]> {
    if (groupId) {
      GroupIdSchema.parse({ groupId });
    }
    const result = await invoke("list_entries", groupId ? { groupId } : {});
    return z.array(EntrySchema).parse(result);
  },

  async get(id: string): Promise<Entry> {
    IdSchema.parse({ id });
    const result = await invoke("get_entry", { id });
    return EntrySchema.parse(result);
  },

  async getPassword(id: string): Promise<string> {
    IdSchema.parse({ id });
    const result = await invoke("get_entry_password", { id });
    return z.string().parse(result);
  },

  async getProtectedCustomField(
    id: string,
    key: string
  ): Promise<CustomFieldValue> {
    IdSchema.parse({ id });
    CustomFieldKeySchema.parse({ key });
    const result = await invoke("get_entry_protected_custom_field", {
      id,
      key,
    });
    return CustomFieldValueSchema.parse(result);
  },

  async create(groupId: string, data: CreateEntryData): Promise<Entry> {
    GroupIdSchema.parse({ groupId });
    CreateEntryDataSchema.parse(data);
    const result = await invoke("create_entry", { groupId, data });
    return EntrySchema.parse(result);
  },

  async update(id: string, data: UpdateEntryData): Promise<Entry> {
    IdSchema.parse({ id });
    UpdateEntryDataSchema.parse(data);
    const result = await invoke("update_entry", { id, data });
    return EntrySchema.parse(result);
  },

  async delete(id: string): Promise<void> {
    IdSchema.parse({ id });
    return invoke("delete_entry", { id });
  },
};

/**
 * Group CRUD operations for organizing entries.
 */
export const groups = {
  async list(): Promise<Group[]> {
    const result = await invoke("list_groups");
    return z.array(GroupSchema).parse(result);
  },

  async get(id: string): Promise<Group> {
    IdSchema.parse({ id });
    const result = await invoke("get_group", { id });
    return GroupSchema.parse(result);
  },

  async create(parentId: string, name: string): Promise<Group> {
    z.uuid().parse(parentId);
    NameSchema.parse({ name });
    const result = await invoke("create_group", { parentId, name });
    return GroupSchema.parse(result);
  },

  async rename(id: string, name: string): Promise<Group> {
    IdSchema.parse({ id });
    NameSchema.parse({ name });
    const result = await invoke("rename_group", { id, name });
    return GroupSchema.parse(result);
  },

  async delete(id: string): Promise<void> {
    IdSchema.parse({ id });
    return invoke("delete_group", { id });
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
  async copyPassword(entryId: string, timeoutMs?: number): Promise<void> {
    CopyPasswordSchema.parse({ entryId, timeoutMs });
    return invoke("copy_password_to_clipboard", { entryId, timeoutMs });
  },

  async clear(): Promise<void> {
    return invoke("clear_clipboard");
  },
};
