import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const css = readFileSync(resolve(process.cwd(), "src/App.css"), "utf8");

function block(selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escaped}\\s*\\{([^}]*)\\}`, "m"));
  if (!match) throw new Error(`CSS block not found: ${selector}`);
  return match[1];
}

describe("console layout CSS", () => {
  it("aligns the preset tiles and signal chain on the same 8-column grid", () => {
    expect(block(".tile-row")).toContain(
      "grid-template-columns: repeat(auto-fit, minmax(90px, 1fr))",
    );
    expect(block(".signal-chain-track")).toContain("display: grid");
    expect(block(".signal-chain-track")).toContain(
      "grid-template-columns: repeat(8, minmax(0, 1fr))",
    );
    expect(block(".chain-link")).toContain("display: none");
  });

  it("keeps the preset header and save plus visually compact", () => {
    expect(block(".presets .section-label")).toContain("font-size: 0.72rem");
    expect(block(".preset-save-plus")).toContain("width: 18px");
    expect(block(".preset-save-plus")).toContain("height: 18px");
    expect(block(".preset-save-plus")).toContain("font-size: 0.72rem");
  });
});
