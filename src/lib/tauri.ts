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

export const database = {
  async open(path: string, password: string): Promise<DatabaseInfo> {
    return invoke("open_database", { path, password });
  },

  async close(): Promise<void> {
    return invoke("close_database");
  },

  async save(): Promise<void> {
    return invoke("save_database");
  },

  async create(path: string, password: string): Promise<DatabaseInfo> {
    return invoke("create_database", { path, password });
  },
};

export const entries = {
  async list(): Promise<Entry[]> {
    return invoke("list_entries");
  },

  async get(id: string): Promise<Entry> {
    return invoke("get_entry", { id });
  },

  async getPassword(id: string): Promise<string> {
    return invoke("get_entry_password", { id });
  },

  async create(groupId: string, data: CreateEntryData): Promise<Entry> {
    return invoke("create_entry", { groupId, ...data });
  },

  async update(id: string, data: UpdateEntryData): Promise<Entry> {
    return invoke("update_entry", { id, ...data });
  },

  async delete(id: string): Promise<void> {
    return invoke("delete_entry", { id });
  },
};

export const groups = {
  async list(): Promise<Group[]> {
    return invoke("list_groups");
  },

  async get(id: string): Promise<Group> {
    return invoke("get_group", { id });
  },

  async create(parentId: string, name: string): Promise<Group> {
    return invoke("create_group", { parentId, name });
  },

  async rename(id: string, name: string): Promise<Group> {
    return invoke("rename_group", { id, name });
  },

  async delete(id: string): Promise<void> {
    return invoke("delete_group", { id });
  },
};

export const generator = {
  async generate(options: PasswordGeneratorOptions): Promise<string> {
    return invoke("generate_password", { options });
  },
};

export const clipboard = {
  async copyPassword(entryId: string, timeoutMs?: number): Promise<void> {
    return invoke("copy_password_to_clipboard", { entryId, timeoutMs });
  },

  async clear(): Promise<void> {
    return invoke("clear_clipboard");
  },
};
