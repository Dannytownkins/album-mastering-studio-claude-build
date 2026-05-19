import { describe, expect, it } from "vitest";

import {
  defaultExportPath,
  exportDirectoryFromPath,
  lastExportDirectory,
  rememberExportDirectory,
  type ExportLocationStore,
} from "./export-location";

function memoryStore(): ExportLocationStore {
  const values = new Map<string, string>();
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => {
      values.set(key, value);
    },
  };
}

describe("export location persistence", () => {
  it("extracts a directory from Mac, Windows, and bare filenames", () => {
    expect(exportDirectoryFromPath("/Users/daniel/Desktop/song.wav")).toBe(
      "/Users/daniel/Desktop",
    );
    expect(exportDirectoryFromPath("C:\\Users\\Dan\\Desktop\\song.wav")).toBe(
      "C:\\Users\\Dan\\Desktop",
    );
    expect(exportDirectoryFromPath("song.wav")).toBeNull();
  });

  it("builds a default export path inside the last track directory", () => {
    const store = memoryStore();
    rememberExportDirectory(store, "track", "/Users/daniel/Desktop/song.wav");

    expect(defaultExportPath(store, "track", "next__master.wav")).toBe(
      "/Users/daniel/Desktop/next__master.wav",
    );
  });

  it("remembers album folders directly for the next folder picker", () => {
    const store = memoryStore();
    rememberExportDirectory(store, "album", "/Users/daniel/Desktop/Album Masters");

    expect(lastExportDirectory(store, "album")).toBe(
      "/Users/daniel/Desktop/Album Masters",
    );
  });
});
