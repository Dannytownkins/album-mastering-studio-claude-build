// Phase 12.2 P2 — right-rail master-out / quality panels.
//
// Drives all readouts off the user's most-recent analysis (`selectedAnalysis`).
// In v1 this is source-side data — the rendered master is not re-analyzed
// here yet. A follow-up slice will trigger a re-analyze after preview /
// export so the readouts reflect what was actually delivered.

import type { ReactNode } from "react";
import type { AnalysisResult, QualityCheck } from "../bindings";

type RightRailProps = {
  analysis: AnalysisResult | undefined;
  isAnalyzing: boolean;
  lastChecks: QualityCheck[] | undefined;
  // Slot for the AdvancedPanel content. Wrapped in a collapsible details/
  // summary container so it sits between the quality summary and quality
  // check panels (matches the reference layout).
  advancedSlot?: ReactNode;
};

const LUFS_SCALE_MIN = -36;
const LUFS_SCALE_MAX = -6;

export function RightRail({ analysis, isAnalyzing, lastChecks, advancedSlot }: RightRailProps) {
  return (
    <aside className="right-rail">
      <MasterOutPanel analysis={analysis} isAnalyzing={isAnalyzing} />
      <QualitySummaryCard analysis={analysis} />
      {advancedSlot && (
        <details className="panel advanced-panel-slot" open>
          <summary className="panel-head panel-head-summary">
            <span className="panel-title">ADVANCED CONTROLS</span>
            <span className="panel-chevron" aria-hidden>⌄</span>
          </summary>
          <div className="advanced-slot-body">{advancedSlot}</div>
        </details>
      )}
      <QualityCheckPanel checks={lastChecks} analysis={analysis} />
    </aside>
  );
}

function MasterOutPanel({
  analysis,
  isAnalyzing,
}: {
  analysis: AnalysisResult | undefined;
  isAnalyzing: boolean;
}) {
  const lufs = analysis?.lufs_integrated;
  const lufsShort = analysis?.lufs_short_term_max;
  const tp = analysis?.true_peak_dbtp;
  const dr = analysis?.dynamic_range_lu;
  const width = analysis?.stereo_width;

  return (
    <section className="panel master-out">
      <header className="panel-head">
        <span className="panel-title">MASTER OUT</span>
        {isAnalyzing && <span className="panel-hint">analyzing…</span>}
      </header>
      <div className="lufs-meter">
        <LufsScale />
        <div className="lufs-bars">
          <LufsBar value={lufs} channel="L" />
          <LufsBar value={lufs} channel="R" />
          {lufsShort !== undefined && (
            <>
              <LufsBar value={lufsShort} channel="L" tone="short" />
              <LufsBar value={lufsShort} channel="R" tone="short" />
            </>
          )}
        </div>
      </div>
      <dl className="master-readouts">
        <Readout
          label="Integrated LUFS"
          value={lufs !== undefined ? lufs.toFixed(1) : "—"}
          unit=""
        />
        <Readout
          label="True Peak"
          value={tp !== undefined ? tp.toFixed(1) : "—"}
          unit="dBTP"
        />
        <Readout
          label="Dynamic Range"
          value={dr !== undefined ? dr.toFixed(1) : "—"}
          unit="LU"
        />
        <Readout
          label="Stereo Width"
          value={width !== undefined ? `${Math.round(width * 100)}` : "—"}
          unit="%"
        />
      </dl>
    </section>
  );
}

function LufsScale() {
  // Drawing the dB ticks alongside the meter bars. Matches the reference's
  // descending scale from -6 (top) down to -36 (bottom).
  const ticks = [-6, -12, -18, -24, -30, -36];
  return (
    <div className="lufs-scale">
      {ticks.map((db) => (
        <span key={db} className="lufs-tick">{db}</span>
      ))}
    </div>
  );
}

function LufsBar({
  value,
  channel,
  tone = "integrated",
}: {
  value: number | undefined;
  channel: "L" | "R";
  tone?: "integrated" | "short";
}) {
  // Map dBFS value into a 0..1 fill ratio against the meter scale.
  // Values above LUFS_SCALE_MAX are pinned to 1.0; below LUFS_SCALE_MIN, 0.
  let fill = 0;
  if (value !== undefined && Number.isFinite(value)) {
    const clamped = Math.max(LUFS_SCALE_MIN, Math.min(LUFS_SCALE_MAX, value));
    fill = (clamped - LUFS_SCALE_MIN) / (LUFS_SCALE_MAX - LUFS_SCALE_MIN);
  }
  return (
    <div className={`lufs-bar lufs-bar-${tone}`}>
      <div className="lufs-bar-fill" style={{ height: `${fill * 100}%` }} />
      <span className="lufs-bar-label">{channel}</span>
    </div>
  );
}

function Readout({
  label,
  value,
  unit,
}: {
  label: string;
  value: string;
  unit: string;
}) {
  return (
    <div className="readout">
      <dt className="readout-label">{label}</dt>
      <dd className="readout-value">
        <span className="readout-number">{value}</span>
        {unit && <span className="readout-unit">{unit}</span>}
      </dd>
    </div>
  );
}

function QualitySummaryCard({
  analysis,
}: {
  analysis: AnalysisResult | undefined;
}) {
  const grade = gradeAnalysis(analysis);
  return (
    <section className={`panel quality-summary quality-${grade.tone}`}>
      <header className="panel-head">
        <span className="panel-title">QUALITY SUMMARY</span>
      </header>
      <div className="quality-summary-body">
        <span className={`quality-icon quality-icon-${grade.tone}`} aria-hidden>
          {grade.icon}
        </span>
        <div className="quality-summary-text">
          <strong className="quality-grade">{grade.label}</strong>
          <p className="quality-blurb">{grade.blurb}</p>
        </div>
      </div>
    </section>
  );
}

function gradeAnalysis(analysis: AnalysisResult | undefined): {
  tone: "info" | "ok" | "warn" | "bad";
  icon: string;
  label: string;
  blurb: string;
} {
  if (!analysis) {
    return {
      tone: "info",
      icon: "…",
      label: "Awaiting analysis",
      blurb: "Drop a track and press Analyze to see master metering.",
    };
  }
  const tp = analysis.true_peak_dbtp;
  const lufs = analysis.lufs_integrated;
  const dr = analysis.dynamic_range_lu;

  if (tp > -0.1 || !Number.isFinite(lufs)) {
    return {
      tone: "bad",
      icon: "!",
      label: "Critical issue",
      blurb: "True peak is clipping or loudness is non-finite. Back off the chain before exporting.",
    };
  }
  if (tp > -1.0 || dr < 5.0 || lufs > -8.0) {
    return {
      tone: "warn",
      icon: "△",
      label: "Needs attention",
      blurb: "Low streaming headroom, very compressed, or extremely loud. Review before exporting.",
    };
  }
  return {
    tone: "ok",
    icon: "✓",
    label: "Excellent",
    blurb: "Balanced tonality, controlled dynamics, and within streaming-target headroom.",
  };
}

function QualityCheckPanel({
  checks,
  analysis,
}: {
  checks: QualityCheck[] | undefined;
  analysis: AnalysisResult | undefined;
}) {
  const rows = checks && checks.length > 0
    ? checks.map((c, i) => ({
        key: `${c.code}-${i}`,
        ok: c.level === "info",
        warn: c.level === "warning",
        crit: c.level === "critical",
        label: friendlyCheckLabel(c),
        detail: c.message,
      }))
    : derivePreflightChecks(analysis);

  const overallSafe = rows.every((r) => r.ok);
  return (
    <section className={`panel quality-check ${overallSafe ? "is-safe" : "has-issues"}`}>
      <header className="panel-head">
        <span className="panel-title">QUALITY CHECK</span>
        <span className={`quality-badge ${overallSafe ? "badge-safe" : "badge-warn"}`}>
          {overallSafe ? "SAFE" : "REVIEW"}
        </span>
      </header>
      <ul className="quality-check-list">
        {rows.map((r) => (
          <li
            key={r.key}
            className={
              "quality-check-row " +
              (r.crit ? "is-crit" : r.warn ? "is-warn" : "is-ok")
            }
            title={r.detail}
          >
            <span className="quality-check-glyph" aria-hidden>
              {r.crit ? "✗" : r.warn ? "△" : "✓"}
            </span>
            <span className="quality-check-text">{r.label}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}

function friendlyCheckLabel(c: QualityCheck): string {
  // The export checks come in as short technical codes. The reference UI
  // uses plain-language one-liners; surface those when we can recognize the
  // code, fall back to the raw message otherwise.
  switch (c.code) {
    case "export_ok":
      return "No issues detected";
    case "true_peak_high":
      return "True peak above safe ceiling";
    case "streaming_headroom_low":
      return "Low streaming headroom";
    case "lufs_very_loud":
      return "Very loud master";
    case "dynamic_range_low":
      return "Heavy compression detected";
    case "bit_depth_low":
      return "Bit depth below 16 bits";
    case "non_finite_metering":
      return "Non-finite loudness measurement";
    case "comp_density_on_compressed_source":
      return "Already-compressed source";
    default:
      return c.message;
  }
}

function derivePreflightChecks(analysis: AnalysisResult | undefined): {
  key: string;
  ok: boolean;
  warn: boolean;
  crit: boolean;
  label: string;
  detail: string;
}[] {
  if (!analysis) {
    return [
      {
        key: "pre-no-analysis",
        ok: false,
        warn: true,
        crit: false,
        label: "Awaiting analysis",
        detail: "Run Analyze to populate quality checks.",
      },
    ];
  }
  const tp = analysis.true_peak_dbtp;
  const lufs = analysis.lufs_integrated;
  const dr = analysis.dynamic_range_lu;
  return [
    {
      key: "tp",
      ok: tp <= -1.0,
      warn: tp > -1.0 && tp <= -0.1,
      crit: tp > -0.1,
      label: `True peak ${tp.toFixed(1)} dBTP`,
      detail: `True peak at ${tp.toFixed(2)} dBTP.`,
    },
    {
      key: "lufs",
      ok: lufs <= -8.0,
      warn: lufs > -8.0 && lufs <= -6.0,
      crit: lufs > -6.0,
      label: `Loudness ${lufs.toFixed(1)} LUFS`,
      detail: `Integrated loudness at ${lufs.toFixed(2)} LUFS.`,
    },
    {
      key: "dr",
      ok: dr >= 6.0,
      warn: dr >= 4.0 && dr < 6.0,
      crit: dr < 4.0,
      label: `Dynamic range ${dr.toFixed(1)} LU`,
      detail: `Source dynamic range at ${dr.toFixed(2)} LU.`,
    },
  ];
}
