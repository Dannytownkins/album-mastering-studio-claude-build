// Phase 12.2 P2 — right-rail master-out / quality panels.
//
// Drives all readouts off the user's most-recent analysis (`selectedAnalysis`).
// In v1 this is source-side data — the rendered master is not re-analyzed
// here yet. A follow-up slice will trigger a re-analyze after preview /
// export so the readouts reflect what was actually delivered.

import type { ReactNode } from "react";
import type { AnalysisResult, QualityCheck } from "../bindings";

type RightRailProps = {
  /// QualityCheckPanel uses this for the preflight checks when no
  /// export receipt has been generated yet.
  analysis: AnalysisResult | undefined;
  lastChecks: QualityCheck[] | undefined;
  /// Slot for the advanced rail cards (Delivery Profile, Advanced
  /// Controls, Per-Band Compressor, Delivery Format). App.tsx composes
  /// them as a fragment so the rail just renders the slot in the right
  /// place, between Quality Check and the sticky Export group.
  advancedSlot?: ReactNode;
  // Export action — promoted from the workspace into the right rail to
  // match the reference layout. Disabled until analysis exists and while
  // any render/export is in flight.
  canExport: boolean;
  isExporting: boolean;
  isRendering: boolean;
  onExport: () => void;
  // UI restyle 2026-05-14: the secondary "Render audit WAV" action used
  // to live in the main StaleBar. Moved here so the playback strip can
  // become a quiet status indicator, while audit-WAV stays one click
  // away from Export Master — its natural neighbor.
  previewStale: boolean;
  canRenderPreview: boolean;
  onUpdatePreview: () => void;
};

const LUFS_SCALE_MIN = -36;
const LUFS_SCALE_MAX = -6;
const CLIP_THRESHOLD_DBFS = -0.1;
const HEADROOM_WARN_DBFS = -1.0;
const SILENCE_FLOOR_DBFS = -80;

export function RightRail({
  analysis,
  lastChecks,
  advancedSlot,
  canExport,
  isExporting,
  isRendering,
  onExport,
  previewStale,
  canRenderPreview,
  onUpdatePreview,
}: RightRailProps) {
  return (
    <aside className="right-rail">
      {/* UI_LAYOUT_REVISION_1600x940 L3 — rail order per spec:
          Quality Check → advancedSlot (Delivery / Advanced / Per-Band /
          Bit+SR cards, App.tsx composes) → sticky Export Master at
          bottom. Levels moved to the waveform deck's meters column;
          MasterOutPanel + StereoWidthGauge moved in L2. */}
      <QualityCheckPanel checks={lastChecks} analysis={analysis} />
      {advancedSlot}
      <div className="right-rail-export-group">
        <button
          type="button"
          className="primary right-rail-export"
          onClick={onExport}
          disabled={!canExport || isExporting || isRendering}
          title={
            isRendering && !isExporting
              ? "Disabled while a render-audit WAV is in progress — they share render state."
              : !canExport
              ? "Analyze a track first."
              : undefined
          }
        >
          {isExporting ? "Exporting…" : "Export Master"}
        </button>
        <details className="right-rail-tools">
          <summary>Tools</summary>
          <button
            type="button"
            className="ghost-btn right-rail-audit"
            onClick={onUpdatePreview}
            disabled={!canRenderPreview || isRendering || isExporting}
            title={
              isExporting
                ? "Disabled while an export is in progress — they share render state."
                : !canRenderPreview
                ? "Import a track first."
                : "Render a temporary WAV with the current settings so you can audit it in another player or DAW. Not required for live audition — the Original/Mastered toggle plays through the chain in real time."
            }
          >
            {previewStale ? "Render audit WAV" : "Re-render audit WAV"}
          </button>
        </details>
      </div>
    </aside>
  );
}

export function MasterOutPanel({
  analysis,
  isAnalyzing,
  peakDbfs,
  isPlaying,
  lufsMomentary,
  lufsIntegrated,
}: {
  analysis: AnalysisResult | undefined;
  isAnalyzing: boolean;
  peakDbfs: number;
  isPlaying: boolean;
  lufsMomentary: number;
  lufsIntegrated: number;
}) {
  const tp = analysis?.true_peak_dbtp;
  const dr = analysis?.dynamic_range_lu;

  // Bars drive off the LIVE BS.1770 momentary LUFS during playback.
  const liveMomentary = isPlaying && lufsMomentary > -120 ? lufsMomentary : undefined;
  // Peak-hold line on the bars: during playback we show the LIVE integrated
  // value so Dan can watch the bar dance around the integrated aggregate as
  // playback progresses through the track. When paused, it shows the last
  // analyzed integrated value so the line still says "this is where it
  // landed last time."
  const analyzedIntegrated = analysis?.lufs_integrated;
  const liveIntegrated =
    isPlaying && lufsIntegrated > -120 ? lufsIntegrated : undefined;
  const peakHoldIntegrated = liveIntegrated ?? analyzedIntegrated;
  // Live TP estimate: use the same live peak (dBFS ≈ dBTP for our chain;
  // not strictly true-peak in the BS.1770 sense, but in the right ballpark
  // for a live indicator).
  const liveTp = isPlaying && peakDbfs > -120 ? peakDbfs : undefined;

  // Readouts. Momentary is silent when not playing; integrated falls back
  // to the analyzed value so the user always sees a meaningful number.
  const momentaryDisplay =
    liveMomentary !== undefined ? liveMomentary.toFixed(1) : "—";
  const integratedDisplay =
    peakHoldIntegrated !== undefined ? peakHoldIntegrated.toFixed(1) : "—";

  return (
    <section className="panel master-out">
      <header className="panel-head">
        <span className="panel-title">MASTER OUT</span>
        {isPlaying ? (
          <span className="panel-live-pill" title="Momentary bars + live integrated readout are metering the playback in real time.">
            <span className="panel-live-dot" aria-hidden /> LIVE
          </span>
        ) : isAnalyzing ? (
          <span className="panel-hint">analyzing…</span>
        ) : (
          <span className="panel-hint">hold</span>
        )}
      </header>
      <div className="lufs-meter">
        <div className="lufs-bars">
          <LufsBar value={liveMomentary} peakHold={peakHoldIntegrated} channel="L" />
          <LufsBar value={liveMomentary} peakHold={peakHoldIntegrated} channel="R" />
        </div>
        <LufsScale />
        <TruePeakBar value={liveTp ?? tp} />
      </div>
      <dl className="master-readouts">
        <Readout
          label="Momentary"
          value={momentaryDisplay}
          unit="LUFS"
        />
        <Readout
          label={isPlaying ? "Integrated (live)" : "Integrated"}
          value={integratedDisplay}
          unit="LUFS"
        />
        <Readout
          label="True Peak"
          value={tp !== undefined ? tp.toFixed(1) : "—"}
          unit="dBTP"
        />
        <Readout
          label="Dyn. Range"
          value={dr !== undefined ? dr.toFixed(1) : "—"}
          unit="LU"
        />
      </dl>
      {/* StereoWidthGauge used to render here as part of MasterOutPanel.
          UI_LAYOUT_REVISION_1600x940 L2 pulls it out so the caller can
          position it as its own section beside Master Out in the
          waveform deck's meters column. */}
    </section>
  );
}

export function StereoWidthGauge({ width }: { width: number }) {
  // The chain's internal width is 0 (mono) → 1 (neutral) → 2 (max widen).
  // Display it on a -1..+1 scale where 0 = neutral. Driven by the live
  // chain setting (user's Width slider or preset default), so dragging
  // the Width control in Advanced moves the needle in real time.
  const display = Number.isFinite(width) ? width - 1.0 : 0;
  const clamped = Math.max(-1, Math.min(1, display));

  // Semi-circular gauge: arc from -135° (left) up through -90° (top) to
  // -45° (right). t in [0, 1] picks the angle along that arc.
  const angleAtT = (t: number) => -180 + t * 180;
  const t = (clamped + 1) / 2;
  const needleDeg = angleAtT(t);

  // SVG geometry. We render the arc as a half-circle in a 220×130 box.
  const cx = 110;
  const cy = 115;
  const r = 90;
  const startA = Math.PI; // 180°
  const endA = 2 * Math.PI; // 360° / 0°
  const arcPath =
    `M ${cx + r * Math.cos(startA)} ${cy + r * Math.sin(startA)} ` +
    `A ${r} ${r} 0 0 1 ${cx + r * Math.cos(endA)} ${cy + r * Math.sin(endA)}`;

  // Tick locations at -1, -0.5, 0, +0.5, +1 (mapped through the same angle).
  const ticks = [-1, -0.5, 0, 0.5, 1].map((v) => {
    const tv = (v + 1) / 2;
    const a = (angleAtT(tv) * Math.PI) / 180;
    return {
      v,
      x1: cx + (r - 6) * Math.cos(a),
      y1: cy + (r - 6) * Math.sin(a),
      x2: cx + r * Math.cos(a),
      y2: cy + r * Math.sin(a),
      major: v === 0 || v === -1 || v === 1,
    };
  });

  const needleA = (needleDeg * Math.PI) / 180;
  const needleR = r - 4;
  const nx = cx + needleR * Math.cos(needleA);
  const ny = cy + needleR * Math.sin(needleA);

  const valueDisplay = display.toFixed(2);
  const caption =
    clamped < -0.3
      ? "Narrow"
      : clamped > 0.3
        ? "Wide"
        : "Balanced";

  return (
    <section className="panel stereo-width-panel">
      <header className="panel-head">
        <span className="panel-title">STEREO WIDTH</span>
      </header>
      <div className="stereo-gauge-vis">
        <svg viewBox="0 0 220 140" width="100%" preserveAspectRatio="xMidYMin meet">
          {/* Background arc track */}
          <path
            d={arcPath}
            fill="none"
            stroke="rgba(120,135,165,0.22)"
            strokeWidth={6}
            strokeLinecap="round"
          />
          {/* Active arc — from center (t=0.5) outward toward the needle, so
              moving away from neutral fills the arc in whichever direction. */}
          {(() => {
            const center = 0.5;
            const lo = Math.min(center, t);
            const hi = Math.max(center, t);
            const aLo = (angleAtT(lo) * Math.PI) / 180;
            const aHi = (angleAtT(hi) * Math.PI) / 180;
            const sweep =
              `M ${cx + r * Math.cos(aLo)} ${cy + r * Math.sin(aLo)} ` +
              `A ${r} ${r} 0 0 1 ${cx + r * Math.cos(aHi)} ${cy + r * Math.sin(aHi)}`;
            return (
              <path
                d={sweep}
                fill="none"
                stroke="var(--accent-bright)"
                strokeWidth={6}
                strokeLinecap="round"
                style={{ filter: "drop-shadow(0 0 6px rgba(111,163,255,0.65))" }}
              />
            );
          })()}
          {/* Ticks */}
          {ticks.map((tk) => (
            <line
              key={`stw-${tk.v}`}
              x1={tk.x1}
              y1={tk.y1}
              x2={tk.x2}
              y2={tk.y2}
              stroke={tk.major ? "rgba(220,230,250,0.7)" : "rgba(150,165,200,0.45)"}
              strokeWidth={tk.major ? 2 : 1}
              strokeLinecap="round"
            />
          ))}
          {/* End labels */}
          <text
            x={cx + (r + 14) * Math.cos(Math.PI)}
            y={cy + (r + 14) * Math.sin(Math.PI) + 4}
            fontSize="10"
            fill="var(--text-2)"
            textAnchor="end"
          >
            -1
          </text>
          <text x={cx} y={cy - r - 6} fontSize="10" fill="var(--text-2)" textAnchor="middle">
            0
          </text>
          <text
            x={cx + (r + 14) * Math.cos(0)}
            y={cy + (r + 14) * Math.sin(0) + 4}
            fontSize="10"
            fill="var(--text-2)"
            textAnchor="start"
          >
            +1
          </text>
          {/* Needle */}
          <line
            x1={cx}
            y1={cy}
            x2={nx}
            y2={ny}
            stroke="var(--text-0)"
            strokeWidth={2.5}
            strokeLinecap="round"
            style={{ filter: "drop-shadow(0 0 4px rgba(255,255,255,0.4))" }}
          />
          <circle cx={cx} cy={cy} r={4} fill="var(--text-0)" />
        </svg>
        <div className="stereo-gauge-readout">
          <span className="stereo-gauge-value">{valueDisplay}</span>
          <span className="stereo-gauge-caption">{caption}</span>
        </div>
      </div>
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
  peakHold,
  channel,
}: {
  value: number | undefined;
  peakHold: number | undefined;
  channel: "L" | "R";
}) {
  // Map a dBFS value into 0..1 fill against the -36..-6 scale.
  const ratio = (db: number): number => {
    if (!Number.isFinite(db)) return 0;
    const clamped = Math.max(LUFS_SCALE_MIN, Math.min(LUFS_SCALE_MAX, db));
    return (clamped - LUFS_SCALE_MIN) / (LUFS_SCALE_MAX - LUFS_SCALE_MIN);
  };
  const fill = value !== undefined ? ratio(value) : 0;
  const peakRatio = peakHold !== undefined ? ratio(peakHold) : null;
  return (
    <div className="lufs-bar">
      <div className="lufs-bar-track" />
      <div className="lufs-bar-fill" style={{ height: `${fill * 100}%` }} />
      {peakRatio !== null && peakRatio > 0 && (
        <div
          className="lufs-peak-hold"
          style={{ bottom: `calc(${peakRatio * 100}% - 1px)` }}
          title="Short-term max"
        />
      )}
      <span className="lufs-bar-label">{channel}</span>
    </div>
  );
}

function TruePeakBar({ value }: { value: number | undefined }) {
  // True peak gets its own narrow bar on a 0..-36 dBTP scale (0 at top means
  // clipping). The fill itself uses the warm-to-hot gradient because high
  // true peak is bad; safe headroom reads as quiet/low fill.
  const TP_MIN = -36;
  const TP_MAX = 0;
  let fill = 0;
  let tone: "ok" | "warn" | "hot" = "ok";
  if (value !== undefined && Number.isFinite(value)) {
    const clamped = Math.max(TP_MIN, Math.min(TP_MAX, value));
    fill = (clamped - TP_MIN) / (TP_MAX - TP_MIN);
    if (value > -0.1) tone = "hot";
    else if (value > -1.0) tone = "warn";
  }
  return (
    <div className={`tp-bar tp-${tone}`}>
      <div className="tp-bar-track" />
      <div className="tp-bar-fill" style={{ height: `${fill * 100}%` }} />
      <div className="tp-clip-line" title="-1 dBTP streaming ceiling" />
      <span className="tp-bar-label">TP</span>
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

export function LevelsPanel({
  peakDbfs,
  isPlaying,
  compressionGr,
}: {
  peakDbfs: number;
  isPlaying: boolean;
  compressionGr: { low: number; mid: number; high: number };
}) {
  // Status reads off the post-output peak we already meter live (no fake
  // grading): idle when not playing, silent when peak is at sentinel,
  // clipping when above -0.1, hot when above -1.0, otherwise safe.
  let tone: "idle" | "silent" | "ok" | "warn" | "clip";
  if (!isPlaying) tone = "idle";
  else if (peakDbfs <= SILENCE_FLOOR_DBFS) tone = "silent";
  else if (peakDbfs >= CLIP_THRESHOLD_DBFS) tone = "clip";
  else if (peakDbfs >= HEADROOM_WARN_DBFS) tone = "warn";
  else tone = "ok";

  const peakDisplay = !isPlaying || peakDbfs <= SILENCE_FLOOR_DBFS
    ? "—"
    : peakDbfs.toFixed(1);

  // Worst (most-negative) GR across the three bands while playing.
  const grValues = [compressionGr.low, compressionGr.mid, compressionGr.high]
    .filter((v) => Number.isFinite(v) && v > SILENCE_FLOOR_DBFS);
  const worstGr = grValues.length > 0 ? Math.min(...grValues) : 0;
  const grDisplay = !isPlaying || grValues.length === 0
    ? "—"
    : worstGr.toFixed(1);

  const label = {
    idle: "Idle",
    silent: "Silent",
    ok: "Safe headroom",
    warn: "Low headroom",
    clip: "Clipping",
  }[tone];

  return (
    <section className={`panel levels levels-${tone}`}>
      <header className="panel-head">
        <span className="panel-title">LEVELS</span>
        <span className={`status-pill status-${tone}`}>{label}</span>
      </header>
      <div className="levels-body">
        <div className="level-readout">
          <span className="level-label">Output peak</span>
          <span className="level-value">
            <span className="level-number">{peakDisplay}</span>
            <span className="level-unit">dBFS</span>
          </span>
        </div>
        <div className="level-readout">
          <span className="level-label">Worst GR</span>
          <span className="level-value">
            <span className="level-number">{grDisplay}</span>
            <span className="level-unit">dB</span>
          </span>
        </div>
      </div>
      <div className="levels-hint">
        {tone === "idle" && "Press play to start metering."}
        {tone === "silent" && "Output is below -80 dBFS in the last window."}
        {tone === "ok" && "Peaks stay safely under streaming ceiling."}
        {tone === "warn" && "Peaks are pushing the streaming ceiling — back off output."}
        {tone === "clip" && "Output is clipping. Drop intensity or output gain."}
      </div>
    </section>
  );
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
