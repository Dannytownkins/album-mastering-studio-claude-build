import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "vitest";

const repoRoot = resolve(__dirname, "../..");

function readJson(path: string) {
  return JSON.parse(readFileSync(resolve(repoRoot, path), "utf8"));
}

describe("Windows app packaging", () => {
  test("Tauri bundling is enabled with Windows icon assets", () => {
    const config = readJson("src-tauri/tauri.conf.json");
    const icons = config.bundle?.icon ?? [];

    expect(config.identifier).toBe("com.albummasteringstudio.yesmaster");
    expect(config.bundle?.active).toBe(true);
    expect(config.bundle?.targets).toBe("all");
    expect(config.bundle?.windows).toEqual({
      webviewInstallMode: {
        silent: true,
        type: "downloadBootstrapper",
      },
    });
    expect(icons).toContain("icons/icon.ico");
    expect(existsSync(resolve(repoRoot, "src-tauri/icons/icon.ico"))).toBe(true);
  });

  test("package scripts include a Windows installer build command", () => {
    const packageJson = readJson("package.json");

    expect(packageJson.scripts?.["build:windows"]).toBe(
      "rimraf src-tauri/target/release/produce_dialog_smoke.exe && tauri build --bundles msi,nsis",
    );
    expect(packageJson.devDependencies?.rimraf).toBeDefined();
  });

  test("release app builds omit Windows-specific development helper binaries", () => {
    const cargoToml = readFileSync(resolve(repoRoot, "src-tauri/Cargo.toml"), "utf8");

    expect(cargoToml).toContain("autobins = false");
    expect(cargoToml).toContain('name = "album-mastering-studio"');
    expect(cargoToml).not.toContain('name = "produce_dialog_smoke"');
    expect(cargoToml).not.toContain('name = "produce_dialog_smoke.exe"');
    expect(existsSync(resolve(repoRoot, "src-tauri/examples/produce_dialog_smoke.rs"))).toBe(true);
  });
});
