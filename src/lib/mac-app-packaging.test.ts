import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "vitest";

const repoRoot = resolve(__dirname, "../..");

function readJson(path: string) {
  return JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"));
}

function readPngHeader(path: string) {
  const bytes = readFileSync(resolve(repoRoot, path));
  return {
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
    bitDepth: bytes.readUInt8(24),
    colorType: bytes.readUInt8(25),
  };
}

describe("macOS app packaging", () => {
  test("Tauri bundling is enabled with Mac icon assets", () => {
    const config = readJson("src-tauri/tauri.conf.json");
    const icons = config.bundle?.icon ?? [];

    expect(config.identifier).toBe("com.albummasteringstudio.yesmaster");
    expect(config.bundle?.active).toBe(true);
    expect(config.bundle?.targets).toBe("all");
    expect(config.bundle?.macOS?.signingIdentity).toBe("-");
    expect(icons).toContain("icons/icon.icns");
    expect(icons).toContain("icons/icon.ico");
    expect(existsSync(resolve(repoRoot, "src-tauri/icons/icon.icns"))).toBe(true);
  });

  test("package scripts include a Mac app build command", () => {
    const packageJson = readJson("package.json");

    expect(packageJson.scripts?.["build:mac"]).toBe(
      "rm -f src-tauri/target/release/produce_dialog_smoke && tauri build --bundles app,dmg",
    );
  });

  test("release app builds omit development-only helper binaries", () => {
    const cargoToml = readFileSync(resolve(repoRoot, "src-tauri/Cargo.toml"), "utf8");

    expect(cargoToml).toContain("autobins = false");
    expect(cargoToml).toContain('name = "album-mastering-studio"');
    expect(cargoToml).not.toContain('name = "produce_dialog_smoke"');
    expect(existsSync(resolve(repoRoot, "src-tauri/examples/produce_dialog_smoke.rs"))).toBe(true);
  });

  test("runtime PNG icons use 8-bit RGBA pixels", () => {
    expect(readPngHeader("src-tauri/icons/32x32.png")).toEqual({
      width: 32,
      height: 32,
      bitDepth: 8,
      colorType: 6,
    });
    expect(readPngHeader("src-tauri/icons/128x128.png")).toMatchObject({
      width: 128,
      height: 128,
      bitDepth: 8,
      colorType: 6,
    });
    expect(readPngHeader("src-tauri/icons/128x128@2x.png")).toMatchObject({
      width: 256,
      height: 256,
      bitDepth: 8,
      colorType: 6,
    });
  });
});
