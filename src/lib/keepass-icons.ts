// SPDX-License-Identifier: MIT

import type { LucideIcon } from "lucide-react";
import {
  AlertTriangle,
  Archive,
  Ban,
  Banknote,
  BatteryFull,
  Bookmark,
  BookOpen,
  Building,
  Check,
  Clipboard,
  Cog,
  Contact,
  CreditCard,
  Database,
  Disc,
  Eye,
  FileDigit,
  FileText,
  Film,
  Folder,
  FolderOpen,
  Globe,
  HardDrive,
  Home,
  Image,
  Key,
  KeyRound,
  Languages,
  Laptop,
  Link,
  ListTodo,
  Lock,
  Magnet,
  Mail,
  MessageSquare,
  Monitor,
  Network,
  Newspaper,
  Package,
  Pen,
  Pencil,
  Percent,
  Phone,
  PlugZap,
  Printer,
  Puzzle,
  Scan,
  Search,
  Server,
  Settings,
  Share2,
  Smartphone,
  Smile,
  SquareTerminal,
  Star,
  StickyNote,
  Terminal,
  Trash2,
  Unlock,
  User,
  Users,
  Wifi,
  Wrench,
  Zap,
} from "lucide-react";

/**
 * Maps KeePass standard icon IDs (0–68) to the closest lucide-react equivalent.
 *
 * Reference: https://keepass.info/help/base/keys.html
 */
export const KEEPASS_ICON_MAP: Record<number, LucideIcon> = {
  0: Key, // Key
  1: Globe, // World
  2: AlertTriangle, // Warning
  3: Server, // Server
  4: Clipboard, // Clipboard / Marked directory
  5: MessageSquare, // Speech bubble / User communication
  6: Puzzle, // Puzzle piece / Part
  7: Pencil, // Notepad (text)
  8: PlugZap, // Socket / World star (connection)
  9: Contact, // Identity / Contact
  10: BookOpen, // Paper / Address book
  11: Film, // Camera / Film
  12: Wifi, // IR Communication / Wi-Fi key
  13: KeyRound, // Multi-keys / Key ring
  14: Zap, // Energy / Lightning
  15: Search, // Scanner / Search
  16: Star, // World star
  17: Disc, // CD-ROM / Disc
  18: Monitor, // Screen / Monitor
  19: Mail, // Email / Envelope
  20: Settings, // Gear / Configuration
  21: Newspaper, // Clipboard / Paper
  22: Archive, // Paper / Archive
  23: HardDrive, // Run / Terminal
  24: Cog, // Config / Settings
  25: Database, // Notepad (starred) / Database
  26: Lock, // Power-off / Lock
  27: StickyNote, // Paper (expired) / Sticky note
  28: Ban, // Trash / Ban
  29: Image, // Information / Trashcan (expired)
  30: Network, // Network / Folder
  31: Banknote, // Money / Banknote
  32: CreditCard, // Certificate / Card
  33: Smartphone, // Phone / Smartphone
  34: Printer, // Printer
  35: SquareTerminal, // Programmers / Terminal
  36: Building, // Homebanking / Building
  37: Percent, // Certificate star / Percent
  38: Wrench, // Wrench / Tool
  39: Home, // Home
  40: Star, // Star
  41: Pen, // Tux / Pen
  42: Smile, // Feather / Smiley
  43: Laptop, // Apple / Laptop
  44: Eye, // Wiki / Eye
  45: Banknote, // Dollar / Banknote
  46: Phone, // Phone / Phone
  47: Users, // People / Users
  48: Folder, // Folder (closed)
  49: FolderOpen, // Folder (open)
  50: Package, // Package / Box
  51: Lock, // Lock open / Lock
  52: Unlock, // Lock closed / Unlock
  53: Check, // Checked / Checkmark
  54: Pen, // Pen / Write
  55: Image, // Photo / Thumbnail
  56: Bookmark, // Book address / Bookmark
  57: FileText, // Paper / File with text
  58: Share2, // Share / Network share
  59: Link, // Link / URL
  60: Magnet, // Backup / Magnet
  61: ListTodo, // History / To-do list
  62: BatteryFull, // Term / Battery
  63: Scan, // Term / Scanner
  64: User, // User Key
  65: Terminal, // Terminal / Console
  66: FileDigit, // File save / Disk
  67: Trash2, // Trash / Recycle Bin
  68: Languages, // Note / Languages
};

/**
 * Returns the lucide-react icon component for a given KeePass icon ID.
 * Falls back to `Folder` for unknown IDs.
 */
export function getKeepassIcon(iconId: number): LucideIcon {
  return KEEPASS_ICON_MAP[iconId] ?? Folder;
}

/**
 * Parses the group's string icon field to a numeric ID.
 * Returns `null` if the string is null, empty, or not a valid integer.
 */
export function parseGroupIconId(iconStr: string | null): number | null {
  if (iconStr === null || iconStr === "") return null;
  const parsed = Number.parseInt(iconStr, 10);
  return Number.isNaN(parsed) ? null : parsed;
}

/**
 * Returns true for icon IDs that should use the folder open/close swap behavior.
 * This applies to `null` (no icon set), `48` (Folder), and `49` (FolderOpen).
 */
export function isFolderIcon(iconId: number | null): boolean {
  return iconId === null || iconId === 48 || iconId === 49;
}
