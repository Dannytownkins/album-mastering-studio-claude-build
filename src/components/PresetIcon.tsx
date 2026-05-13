// Phase 12.2 — Preset tile icons.
//
// Icons inlined from Lucide (https://lucide.dev), MIT licensed.
// Lucide license: https://github.com/lucide-icons/lucide/blob/main/LICENSE
//
// We inline the path data instead of depending on `lucide-react` because
// we only need 9 icons and the package ships 1000+. Each icon below
// preserves Lucide's standard 24x24 viewBox, 2px stroke width, round
// line caps and joins, and stroke="currentColor" so the SVG inherits
// the parent tile's color (handles active/inactive state automatically).
//
// If any icon needs to be swapped, copy fresh path data from
// `https://lucide.dev/icons/<name>` — do NOT improvise paths.

import type { ReactElement } from "react";
import type { Preset } from "../bindings";

type IconKind = Preset["kind"];

type IconProps = {
  kind: IconKind;
  className?: string;
  "aria-hidden"?: boolean;
};

const STROKE_PROPS = {
  width: 20,
  height: 20,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 2,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

function SparklesPaths() {
  // Lucide "sparkles": https://lucide.dev/icons/sparkles
  return (
    <>
      <path d="M11.017 2.814a1 1 0 0 1 1.966 0l1.051 5.558a2 2 0 0 0 1.594 1.594l5.558 1.051a1 1 0 0 1 0 1.966l-5.558 1.051a2 2 0 0 0-1.594 1.594l-1.051 5.558a1 1 0 0 1-1.966 0l-1.051-5.558a2 2 0 0 0-1.594-1.594l-5.558-1.051a1 1 0 0 1 0-1.966l5.558-1.051a2 2 0 0 0 1.594-1.594z" />
      <path d="M20 2v4" />
      <path d="M22 4h-4" />
      <circle cx="4" cy="20" r="2" />
    </>
  );
}

function EyePaths() {
  // Lucide "eye": https://lucide.dev/icons/eye
  return (
    <>
      <path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0" />
      <circle cx="12" cy="12" r="3" />
    </>
  );
}

function DiscPaths() {
  // Lucide "disc": https://lucide.dev/icons/disc
  return (
    <>
      <circle cx="12" cy="12" r="10" />
      <circle cx="12" cy="12" r="2" />
    </>
  );
}

function Maximize2Paths() {
  // Lucide "maximize-2": https://lucide.dev/icons/maximize-2
  return (
    <>
      <path d="M15 3h6v6" />
      <path d="m21 3-7 7" />
      <path d="m3 21 7-7" />
      <path d="M9 21H3v-6" />
    </>
  );
}

function SpeakerPaths() {
  // Lucide "speaker": https://lucide.dev/icons/speaker
  return (
    <>
      <rect width="16" height="20" x="4" y="2" rx="2" />
      <path d="M12 6h.01" />
      <circle cx="12" cy="14" r="4" />
      <path d="M12 14h.01" />
    </>
  );
}

function FlamePaths() {
  // Lucide "flame": https://lucide.dev/icons/flame
  return (
    <>
      <path d="M12 3q1 4 4 6.5t3 5.5a1 1 0 0 1-14 0 5 5 0 0 1 1-3 1 1 0 0 0 5 0c0-2-1.5-3-1.5-5q0-2 2.5-4" />
    </>
  );
}

function ZapPaths() {
  // Lucide "zap": https://lucide.dev/icons/zap
  return (
    <>
      <path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z" />
    </>
  );
}

function MegaphonePaths() {
  // Lucide "megaphone": https://lucide.dev/icons/megaphone
  return (
    <>
      <path d="M11 6a13 13 0 0 0 8.4-2.8A1 1 0 0 1 21 4v12a1 1 0 0 1-1.6.8A13 13 0 0 0 11 14H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z" />
      <path d="M6 14a12 12 0 0 0 2.4 7.2 2 2 0 0 0 3.2-2.4A8 8 0 0 1 10 14" />
      <path d="M8 6v8" />
    </>
  );
}

function SlidersPaths() {
  // Lucide "sliders": https://lucide.dev/icons/sliders
  return (
    <>
      <path d="M10 8h4" />
      <path d="M12 21v-9" />
      <path d="M12 8V3" />
      <path d="M17 16h4" />
      <path d="M19 12V3" />
      <path d="M19 21v-5" />
      <path d="M3 14h4" />
      <path d="M5 10V3" />
      <path d="M5 21v-7" />
    </>
  );
}

export function PresetIcon({ kind, className, ...rest }: IconProps) {
  let inner: ReactElement;
  switch (kind) {
    case "universal":
      inner = <SparklesPaths />;
      break;
    case "clarity":
      inner = <EyePaths />;
      break;
    case "tape":
      inner = <DiscPaths />;
      break;
    case "spatial":
      inner = <Maximize2Paths />;
      break;
    case "oomph":
      inner = <SpeakerPaths />;
      break;
    case "warmth":
      inner = <FlamePaths />;
      break;
    case "punch":
      inner = <ZapPaths />;
      break;
    case "loud":
      inner = <MegaphonePaths />;
      break;
    case "custom":
      inner = <SlidersPaths />;
      break;
    default: {
      const _exhaustive: never = kind;
      void _exhaustive;
      return null;
    }
  }
  return (
    <svg
      {...STROKE_PROPS}
      className={className}
      aria-hidden={rest["aria-hidden"] ?? true}
    >
      {inner}
    </svg>
  );
}
