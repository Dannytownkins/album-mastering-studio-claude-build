export type ExportLocationKind = "track" | "album";

export interface ExportLocationStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

const LAST_EXPORT_DIR_KEYS: Record<ExportLocationKind, string> = {
  track: "yes-master:last-track-export-dir",
  album: "yes-master:last-album-export-dir",
};

function cleanDirectory(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  if (trimmed === "/" || /^[A-Za-z]:\\$/.test(trimmed)) return trimmed;
  return trimmed.replace(/[\\/]+$/, "") || null;
}

function joinDirectoryAndName(directory: string, filename: string): string {
  const separator = directory.includes("\\") && !directory.includes("/") ? "\\" : "/";
  return `${directory.replace(/[\\/]+$/, "")}${separator}${filename}`;
}

export function exportDirectoryFromPath(path: string): string | null {
  const trimmed = path.trim().replace(/[\\/]+$/, "");
  if (!trimmed) return null;
  const lastSlash = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  if (lastSlash < 0) return null;
  if (lastSlash === 0) return "/";
  return cleanDirectory(trimmed.slice(0, lastSlash));
}

export function lastExportDirectory(
  store: ExportLocationStore | null,
  kind: ExportLocationKind,
): string | null {
  if (!store) return null;
  return cleanDirectory(store.getItem(LAST_EXPORT_DIR_KEYS[kind]) ?? "");
}

export function rememberExportDirectory(
  store: ExportLocationStore | null,
  kind: ExportLocationKind,
  selectedPath: string,
): void {
  const directory =
    kind === "track" ? exportDirectoryFromPath(selectedPath) : cleanDirectory(selectedPath);
  if (!store || !directory) return;
  store.setItem(LAST_EXPORT_DIR_KEYS[kind], directory);
}

export function defaultExportPath(
  store: ExportLocationStore | null,
  kind: ExportLocationKind,
  filename: string,
): string {
  const directory = lastExportDirectory(store, kind);
  return directory ? joinDirectoryAndName(directory, filename) : filename;
}

export function browserExportLocationStore(): ExportLocationStore | null {
  try {
    const storage = globalThis.localStorage;
    if (
      storage &&
      typeof storage.getItem === "function" &&
      typeof storage.setItem === "function"
    ) {
      return storage;
    }
  } catch {
    return null;
  }
  return null;
}
