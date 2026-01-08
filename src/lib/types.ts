// SPDX-License-Identifier: GPL-3.0-or-later

export interface DatabaseInfo {
  name: string;
  path: string;
  isModified: boolean;
  rootGroupId: string;
}

export interface Entry {
  id: string;
  groupId: string;
  title: string;
  username: string;
  url?: string;
  notes?: string;
  createdAt: string;
  modifiedAt: string;
}

export interface Group {
  id: string;
  parentId?: string;
  name: string;
  icon?: string;
  children: Group[];
}

export interface PasswordGeneratorOptions {
  length: number;
  uppercase: boolean;
  lowercase: boolean;
  numbers: boolean;
  symbols: boolean;
  excludeAmbiguous: boolean;
  excludeChars?: string;
}

export interface CreateEntryData {
  title: string;
  username: string;
  password: string;
  url?: string;
  notes?: string;
}

export interface UpdateEntryData {
  title?: string;
  username?: string;
  password?: string;
  url?: string;
  notes?: string;
}
