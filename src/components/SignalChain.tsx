// Phase 12.2 P3 — explicit signal-chain visualization. Renders a horizontal
// strip of stages above the transport so the user sees, at a glance, what
// the mastering chain is doing to their audio. Each stage lights up when
// the relevant settings are non-neutral, with a glow proportional to the
// magnitude of the change.

import type { ReactElement } from "react";
import type { MasteringSettings, Preset } from "../bindings";

type Stage = {
  key: string;
  label: string;
  detail: string;
  active: boolean;
  /// 0..1 intensity used to scale the glow. Below 0.05 reads as off.
  intensity: number;
  icon: () => ReactElement;
};

function presetSaturation(preset: Preset): number {
  // Mirror of the saturation_amount table in dsp.rs::ChainCoeffs::from_settings.
  // Kept in sync by hand — if those numbers change, update here too.
  switch (preset.kind) {
    case "tape":
      return 0.25;
    case "warmth":
      return 0.3;
    case "punch":
      return 0.2;
    case "oomph":
      return 0.15;
    case "loud":
      return 0.1;
    default:
      return 0;
  }
}

function presetDefaultWidth(preset: Preset): number {
  switch (preset.kind) {
    case "spatial":
      return 1.3;
    default:
      return 1.0;
  }
}

function buildStages(settings: MasteringSettings): Stage[] {
  const eqMax = Math.max(
    Math.abs(settings.eq_low_db),
    Math.abs(settings.eq_mid_db),
    Math.abs(settings.eq_high_db),
  );
  const eqIntensity = Math.min(1, eqMax / 6);
  const warmth = settings.advanced.warmth ?? 0;
  const air = settings.advanced.presence_air ?? 0;
  const comp = settings.advanced.compression_density ?? 0;
  const presetWidth = presetDefaultWidth(settings.preset);
  const effectiveWidth = settings.advanced.width ?? presetWidth;
  const widthDelta = Math.abs(effectiveWidth - 1.0);
  const sat = presetSaturation(settings.preset) * (0.4 + 1.2 * Math.min(1, Math.max(0, settings.intensity)));
  return [
    {
      key: "in",
      label: "Source",
      detail: `Intensity ${(settings.intensity * 100).toFixed(0)}%`,
      active: true,
      intensity: 1,
      icon: SourceIcon,
    },
    {
      key: "eq",
      label: "EQ",
      detail: `Low ${signed(settings.eq_low_db, 1)} dB · Mid ${signed(settings.eq_mid_db, 1)} dB · High ${signed(settings.eq_high_db, 1)} dB`,
      active: eqIntensity > 0.01,
      intensity: eqIntensity,
      icon: EqIcon,
    },
    {
      key: "warmth",
      label: "Warmth",
      detail: warmth > 0 ? `+${(warmth * 4).toFixed(1)} dB @ 300 Hz` : "off",
      active: warmth > 0.01,
      intensity: warmth,
      icon: WarmthIcon,
    },
    {
      key: "air",
      label: "Air",
      detail: air > 0 ? `+${(air * 4).toFixed(1)} dB @ 10 kHz` : "off",
      active: air > 0.01,
      intensity: air,
      icon: AirIcon,
    },
    {
      key: "comp",
      label: "Comp",
      detail: comp > 0 ? `Density ${(comp * 100).toFixed(0)}% · -${(comp * 24).toFixed(0)} dBFS thr` : "off",
      active: comp > 0.01,
      intensity: comp,
      icon: CompIcon,
    },
    {
      key: "width",
      label: "Width",
      detail: widthDelta < 0.01 ? "neutral" : `${(effectiveWidth * 100).toFixed(0)}%`,
      active: widthDelta > 0.01,
      intensity: Math.min(1, widthDelta * 1.5),
      icon: WidthIcon,
    },
    {
      key: "sat",
      label: "Saturation",
      detail: sat > 0.05 ? `${(sat * 100).toFixed(0)}% drive` : "off",
      active: sat > 0.05,
      intensity: Math.min(1, sat),
      icon: SatIcon,
    },
    {
      key: "limit",
      label: "Limiter",
      detail: `Ceiling ${(settings.advanced.ceiling_dbtp ?? -1).toFixed(1)} dBTP`,
      active: true,
      intensity: 1,
      icon: LimiterIcon,
    },
  ];
}

function signed(v: number, digits: number): string {
  if (v === 0) return "0";
  return `${v > 0 ? "+" : ""}${v.toFixed(digits)}`;
}

export function SignalChain({ settings }: { settings: MasteringSettings }) {
  const stages = buildStages(settings);
  return (
    <section className="signal-chain" aria-label="Signal chain">
      <div className="signal-chain-track">
        {stages.map((s, i) => (
          <Fragment key={s.key}>
            {i > 0 && (
              <span
                className={
                  "chain-link " + (stages[i - 1].active && s.active ? "is-hot" : "")
                }
                aria-hidden
              />
            )}
            <StageNode stage={s} />
          </Fragment>
        ))}
      </div>
    </section>
  );
}

import { Fragment } from "react";

function StageNode({ stage }: { stage: Stage }) {
  const glowOpacity = stage.active ? Math.max(0.25, stage.intensity) : 0;
  return (
    <div
      className={`chain-stage ${stage.active ? "is-active" : "is-off"}`}
      title={`${stage.label} — ${stage.detail}`}
    >
      <span
        className="chain-stage-disc"
        style={{
          // Live glow intensity follows the stage's setting magnitude so a
          // hot compressor reads as obviously hotter than a gentle Warmth nudge.
          boxShadow: stage.active
            ? `0 0 14px rgba(77, 139, 255, ${glowOpacity * 0.7}), inset 0 0 0 1px rgba(111, 163, 255, ${glowOpacity * 0.6})`
            : undefined,
        }}
      >
        <stage.icon />
      </span>
      <span className="chain-stage-label">{stage.label}</span>
      <span className="chain-stage-detail">{stage.detail}</span>
    </div>
  );
}

// Stage icons — small line-style 20×20 svgs, currentColor stroke so they
// inherit the disc's color (which switches between text-2 and accent
// depending on active state).

function SourceIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 12h3l3-8 4 16 3-8h5" />
    </svg>
  );
}

function EqIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <line x1="4" y1="21" x2="4" y2="14" />
      <line x1="4" y1="10" x2="4" y2="3" />
      <line x1="12" y1="21" x2="12" y2="12" />
      <line x1="12" y1="8" x2="12" y2="3" />
      <line x1="20" y1="21" x2="20" y2="16" />
      <line x1="20" y1="12" x2="20" y2="3" />
      <line x1="1" y1="14" x2="7" y2="14" />
      <line x1="9" y1="8" x2="15" y2="8" />
      <line x1="17" y1="16" x2="23" y2="16" />
    </svg>
  );
}

function WarmthIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3q1 4 4 6.5t3 5.5a1 1 0 0 1-14 0 5 5 0 0 1 1-3 1 1 0 0 0 5 0c0-2-1.5-3-1.5-5q0-2 2.5-4" />
    </svg>
  );
}

function AirIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 8h12a3 3 0 1 0-3-3" />
      <path d="M3 14h18a3 3 0 1 1-3 3" />
      <path d="M3 19h7" />
    </svg>
  );
}

function CompIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 17l4-2 3 1 4-6 3 5 4-3" />
      <path d="M3 21h18" />
    </svg>
  );
}

function WidthIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <polyline points="9 18 3 12 9 6" />
      <polyline points="15 6 21 12 15 18" />
    </svg>
  );
}

function SatIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 12c3 0 3-6 6-6s3 12 6 12 3-6 6-6" />
    </svg>
  );
}

function LimiterIcon() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 6h16" />
      <path d="M4 12l4-3 3 2 4-4 3 3 2-2" />
      <path d="M4 18h16" />
    </svg>
  );
}
