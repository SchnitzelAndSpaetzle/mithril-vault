// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from "@tauri-apps/api/core";
import type {
  DatabaseInfo,
  Entry,
  Group,
  CreateEntryData,
  UpdateEntryData,
  PasswordGeneratorOptions,
} from "./types";

/**
 * Database operations for opening, creating, saving, and closing KDBX databases.
 * All operations communicate with the Rust backend via Tauri IPC.
 */
export const database = {
  /**
   * Opens an existing KDBX database file.
   * @param path - Absolute path to the .kdbx file
   * @param password - Master password to decrypt the database
   * @returns Database metadata including name, path, and root group ID
   * @throws Error if file not found, invalid format, or wrong password
   */
  async open(path: string, password: string): Promise<DatabaseInfo> {
    return invoke("open_database", { path, password });
  },

  /**
   * Closes the currently open database and clears sensitive data from memory.
   * @throws Error if no database is currently open
   */
  async close(): Promise<void> {
    return invoke("close_database");
  },

  /**
   * Saves all pending changes to the database file.
   * @throws Error if no database is open or write fails
   */
  async save(): Promise<void> {
    return invoke("save_database");
  },

  /**
   * Creates a new KDBX database file.
   * @param path - Absolute path where the new .kdbx file will be created
   * @param password - Master password to encrypt the database
   * @returns Database metadata for the newly created database
   * @throws Error if file already exists or path is invalid
   */
  async create(path: string, password: string): Promise<DatabaseInfo> {
    return invoke("create_database", { path, password });
  },
};

/**
 * Entry operations for managing password entries within the database.
 * Entries contain credentials like username, password, URL, and notes.
 */
export const entries = {
  /**
   * Lists all entries in the currently open database.
   * @returns Array of entries (without passwords for security)
   * @throws Error if no database is open
   */
  async list(): Promise<Entry[]> {
    return invoke("list_entries");
  },

  /**
   * Retrieves a single entry by ID.
   * @param id - Unique identifier of the entry
   * @returns Entry data (without password)
   * @throws Error if entry not found or no database is open
   */
  async get(id: string): Promise<Entry> {
    return invoke("get_entry", { id });
  },

  /**
   * Retrieves the decrypted password for an entry.
   * Password is fetched separately to minimize exposure of sensitive data.
   * @param id - Unique identifier of the entry
   * @returns Decrypted password string
   * @throws Error if entry not found or no database is open
   */
  async getPassword(id: string): Promise<string> {
    return invoke("get_entry_password", { id });
  },

  /**
   * Creates a new entry in the specified group.
   * @param groupId - ID of the parent group
   * @param data - Entry data including title, username, password
   * @returns The newly created entry
   * @throws Error if group not found or validation fails
   */
  async create(groupId: string, data: CreateEntryData): Promise<Entry> {
    return invoke("create_entry", { groupId, ...data });
  },

  /**
   * Updates an existing entry.
   * @param id - Unique identifier of the entry to update
   * @param data - Partial entry data with fields to update
   * @returns The updated entry
   * @throws Error if entry not found or validation fails
   */
  async update(id: string, data: UpdateEntryData): Promise<Entry> {
    return invoke("update_entry", { id, ...data });
  },

  /**
   * Permanently deletes an entry.
   * @param id - Unique identifier of the entry to delete
   * @throws Error if entry not found
   */
  async delete(id: string): Promise<void> {
    return invoke("delete_entry", { id });
  },
};

/**
 * Group operations for organizing entries into folders/categories.
 * Groups form a hierarchical tree structure within the database.
 */
export const groups = {
  /**
   * Lists all groups in the database as a flat array.
   * @returns Array of all groups with their hierarchy info
   * @throws Error if no database is open
   */
  async list(): Promise<Group[]> {
    return invoke("list_groups");
  },

  /**
   * Retrieves a single group by ID with its children.
   * @param id - Unique identifier of the group
   * @returns Group data including nested children
   * @throws Error if group not found
   */
  async get(id: string): Promise<Group> {
    return invoke("get_group", { id });
  },

  /**
   * Creates a new group as a child of the specified parent.
   * @param parentId - ID of the parent group
   * @param name - Display name for the new group
   * @returns The newly created group
   * @throws Error if parent not found or name is invalid
   */
  async create(parentId: string, name: string): Promise<Group> {
    return invoke("create_group", { parentId, name });
  },

  /**
   * Renames an existing group.
   * @param id - Unique identifier of the group
   * @param name - New display name
   * @returns The updated group
   * @throws Error if group not found or name is invalid
   */
  async rename(id: string, name: string): Promise<Group> {
    return invoke("rename_group", { id, name });
  },

  /**
   * Permanently deletes a group and optionally its contents.
   * @param id - Unique identifier of the group to delete
   * @throws Error if group not found or is the root group
   */
  async delete(id: string): Promise<void> {
    return invoke("delete_group", { id });
  },
};

/**
 * Password generator for creating secure random passwords.
 */
export const generator = {
  /**
   * Generates a random password based on the specified options.
   * @param options - Configuration for password generation
   * @returns Generated password string
   */
  async generate(options: PasswordGeneratorOptions): Promise<string> {
    return invoke("generate_password", { options });
  },
};

/**
 * Clipboard operations with automatic clearing for security.
 */
export const clipboard = {
  /**
   * Copies an entry's password to the system clipboard.
   * The clipboard is automatically cleared after the timeout.
   * @param entryId - ID of the entry whose password to copy
   * @param timeoutMs - Optional timeout in ms before auto-clear (default: 30s)
   * @throws Error if entry not found
   */
  async copyPassword(entryId: string, timeoutMs?: number): Promise<void> {
    return invoke("copy_password_to_clipboard", { entryId, timeoutMs });
  },

  /**
   * Immediately clears the system clipboard.
   */
  async clear(): Promise<void> {
    return invoke("clear_clipboard");
  },
};
