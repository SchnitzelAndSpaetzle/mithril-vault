// SPDX-License-Identifier: GPL-3.0-or-later

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod/v4";
import {
  DatabaseInfoSchema,
  EntrySchema,
  GroupSchema,
  CreateEntryDataSchema,
  UpdateEntryDataSchema,
  PasswordGeneratorOptionsSchema,
} from "./types";
import type {
  DatabaseInfo,
  Entry,
  Group,
  CreateEntryData,
  UpdateEntryData,
  PasswordGeneratorOptions,
} from "./types";

const PathPasswordSchema = z.object({
  path: z.string().min(1),
  password: z.string().min(1),
});

const IdSchema = z.object({
  id: z.string().uuid(),
});

const GroupIdSchema = z.object({
  groupId: z.string().uuid(),
});

const NameSchema = z.object({
  name: z.string().min(1),
});

const CopyPasswordSchema = z.object({
  entryId: z.string().uuid(),
  timeoutMs: z.number().int().positive().optional(),
});

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

  async create(path: string, password: string): Promise<DatabaseInfo> {
    PathPasswordSchema.parse({ path, password });
    const result = await invoke("create_database", { path, password });
    return DatabaseInfoSchema.parse(result);
  },
};

export const entries = {
  async list(): Promise<Entry[]> {
    const result = await invoke("list_entries");
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

  async create(groupId: string, data: CreateEntryData): Promise<Entry> {
    GroupIdSchema.parse({ groupId });
    CreateEntryDataSchema.parse(data);
    const result = await invoke("create_entry", { groupId, ...data });
    return EntrySchema.parse(result);
  },

  async update(id: string, data: UpdateEntryData): Promise<Entry> {
    IdSchema.parse({ id });
    UpdateEntryDataSchema.parse(data);
    const result = await invoke("update_entry", { id, ...data });
    return EntrySchema.parse(result);
  },

  async delete(id: string): Promise<void> {
    IdSchema.parse({ id });
    return invoke("delete_entry", { id });
  },
};

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
    z.string().uuid().parse(parentId);
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

export const generator = {
  async generate(options: PasswordGeneratorOptions): Promise<string> {
    PasswordGeneratorOptionsSchema.parse(options);
    const result = await invoke("generate_password", { options });
    return z.string().parse(result);
  },
};

export const clipboard = {
  async copyPassword(entryId: string, timeoutMs?: number): Promise<void> {
    CopyPasswordSchema.parse({ entryId, timeoutMs });
    return invoke("copy_password_to_clipboard", { entryId, timeoutMs });
  },

  async clear(): Promise<void> {
    return invoke("clear_clipboard");
  },
};
