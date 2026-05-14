// UI restyle slice 4b — Visual EQ panel v1.
//
// Renders a log-frequency / linear-dB grid with four draggable EQ nodes
// (Low / Low-Mid / Mid / High) pinned to the frequencies the Rust chain
// actually uses (see `ChainCoeffs::from_settings` in src-tauri/src/dsp.rs:
// 200 Hz low shelf, 400 Hz peak Q=0.9, 1500 Hz peak Q=0.8, 6000 Hz high
// shelf). Vertical drag changes a band's gain; double-click resets the
// node to 0 dB. The response curve is an APPROXIMATION of the chain's
// filter cascade — Gaussian peaks + sigmoid shelves in log-frequency
// space — chosen to give the user a fast visual feedback loop, not
// numerically-exact dB-vs-frequency response (the actual Rust chain
// does the audible work; the curve is a "shape preview").
//
// V1 intentionally OMITS:
//   * Horizontal drag (the DSP doesn't yet support variable band
//     frequency or Q — the UI must not promise what the engine can't
//     honor; per the restyle plan, frequency drag waits for variable
//     bands in Rust).
//   * Warmth + Presence/Air nodes (different units — 0..1 saturation
//     drive vs dB EQ — would need separate scaling and don't fit the
//     same plot cleanly).
//   * Live FFT spectrum fill (requires plumbing audio-thread FFT to
//     the frontend; v2 work).

import { useCallback, useRef, useState } from "react";
import type { MasteringSettings } from "../bindings";

type BandId = "low" | "low-mid" | "mid" | "high";
type BandKind = "shelf-low" | "peak" | "shelf-high";

interface Band {
  id: BandId;
  label: string;
  hz: number;
  color: string;
  kind: BandKind;
  /// Q-equivalent in octaves; only used for peak bands' Gaussian width.
  qOctaves: number;
}

// Match the Rust chain's actual filter frequencies — `dsp.rs` line ~620:
//   BiquadCoeffs::low_shelf(sr, 200.0, ...)
//   BiquadCoeffs::peaking(sr, 400.0, 0.9, ...)
//   BiquadCoeffs::peaking(sr, 1500.0, 0.8, ...)
//   BiquadCoeffs::high_shelf(sr, 6000.0, ...)
const BANDS: readonly Band[] = [
  { id: "low", label: "LOW", hz: 200, color: "#22d3ee", kind: "shelf-low", qOctaves: 0 },
  { id: "low-mid", label: "LOW-MID", hz: 400, color: "#4ade80", kind: "peak", qOctaves: 1.0 },
  { id: "mid", label: "MID", hz: 1500, color: "#a78bfa", kind: "peak", qOctaves: 1.2 },
  { id: "high", label: "HIGH", hz: 6000, color: "#60a5fa", kind: "shelf-high", qOctaves: 0 },
];

const F_MIN = 20;
const F_MAX = 20_000;
const DB_MIN = -12;
const DB_MAX = 12;
const GRID_FREQS = [50, 100, 200, 500, 1_000, 2_000, 5_000, 10_000];
const GRID_DBS = [-12, -6, 0, 6, 12];

const LOG_F_MIN = Math.log10(F_MIN);
const LOG_F_MAX = Math.log10(F_MAX);
const LOG_F_SPAN = LOG_F_MAX - LOG_F_MIN;

function freqToX(hz: number, width: number): number {
  return ((Math.log10(hz) - LOG_F_MIN) / LOG_F_SPAN) * width;
}

function dbToY(db: number, height: number): number {
  // 0 dB at the vertical center; +DB_MAX at top, -DB_MIN at bottom.
  return ((DB_MAX - db) / (DB_MAX - DB_MIN)) * height;
}

function yToDb(y: number, height: number): number {
  const raw = DB_MAX - (y / height) * (DB_MAX - DB_MIN);
  return Math.max(DB_MIN, Math.min(DB_MAX, raw));
}

/// Approximate the chain's per-band magnitude response at `hz` for a band
/// configured with `gainDb`. Peaks use a Gaussian whose width tracks the
/// declared Q-octaves; shelves use a logistic sigmoid centered at the
/// shelf frequency. Sum across all bands at each plot point to draw the
/// composite curve.
function bandResponseDb(hz: number, band: Band, gainDb: number): number {
  if (gainDb === 0) return 0;
  const distOctaves = Math.log2(hz / band.hz);
  switch (band.kind) {
    case "peak": {
      // Gaussian whose FWHM ≈ 1 octave at Q=1 (qOctaves ≈ 1).
      const sigma = band.qOctaves * 0.5 / 2.355;
      const safeSigma = sigma > 0 ? sigma : 0.5;
      return gainDb * Math.exp(-0.5 * (distOctaves / safeSigma) ** 2);
    }
    case "shelf-low":
      // Below fc → full gain; above → smoothly returns to 0.
      return gainDb * (1 - 1 / (1 + Math.exp(-1.8 * distOctaves)));
    case "shelf-high":
      // Above fc → full gain; below → smoothly returns to 0.
      return gainDb * (1 / (1 + Math.exp(-1.8 * distOctaves)));
  }
}

function totalResponseDb(hz: number, gains: Record<BandId, number>): number {
  let total = 0;
  for (const band of BANDS) {
    total += bandResponseDb(hz, band, gains[band.id]);
  }
  return total;
}

interface VisualEqPanelProps {
  settings: MasteringSettings;
  onEq: (band: BandId, db: number) => void;
}

export function VisualEqPanel({ settings, onEq }: VisualEqPanelProps) {
  const svgRef = useRef<SVGSVGElement | null>(null);
  // Drag state: which band is being dragged, and the pixel-space y where
  // the pointer is. We don't capture frequency drag at all in v1.
  const [dragging, setDragging] = useState<BandId | null>(null);

  const gains: Record<BandId, number> = {
    "low": settings.eq_low_db,
    "low-mid": settings.eq_low_mid_db,
    "mid": settings.eq_mid_db,
    "high": settings.eq_high_db,
  };

  // SVG drawing constants. Viewport is `width × height` (logical units);
  // CSS sizes the visible panel. Padding leaves room for axis labels.
  const PAD_LEFT = 40;
  const PAD_RIGHT = 12;
  const PAD_TOP = 14;
  // Bottom padding holds two text rows: the frequency axis (50/100/1k/…)
  // on the first line, then the colored band labels (LOW / LOW-MID /
  // MID / HIGH) on the second so they don't overlap the axis ticks.
  const PAD_BOTTOM = 34;
  const AXIS_LABEL_Y_OFFSET = 14;
  const BAND_LABEL_Y_OFFSET = 28;
  const VBW = 720;
  const VBH = 272;
  const plotW = VBW - PAD_LEFT - PAD_RIGHT;
  const plotH = VBH - PAD_TOP - PAD_BOTTOM;

  const localFreqToX = (hz: number) => PAD_LEFT + freqToX(hz, plotW);
  const localDbToY = (db: number) => PAD_TOP + dbToY(db, plotH);
  const yToDbInPlot = (y: number) => yToDb(y - PAD_TOP, plotH);

  // Pre-compute the composite response curve as an SVG path. 180 sample
  // points across the log-frequency range gives smooth visuals without
  // being expensive (this re-runs every settings change).
  const N_SAMPLES = 180;
  const curvePoints: { x: number; y: number }[] = [];
  for (let i = 0; i <= N_SAMPLES; i++) {
    const t = i / N_SAMPLES;
    const logHz = LOG_F_MIN + t * LOG_F_SPAN;
    const hz = Math.pow(10, logHz);
    const db = totalResponseDb(hz, gains);
    curvePoints.push({ x: PAD_LEFT + t * plotW, y: localDbToY(db) });
  }
  const curvePath = curvePoints
    .map((p, i) => (i === 0 ? `M ${p.x.toFixed(2)} ${p.y.toFixed(2)}` : `L ${p.x.toFixed(2)} ${p.y.toFixed(2)}`))
    .join(" ");
  // Filled area under the curve, clipped at the 0-dB line so the fill
  // reads as "lift above zero" / "cut below zero" rather than a giant
  // blob across the panel.
  const zeroY = localDbToY(0);
  const fillPath =
    `M ${PAD_LEFT} ${zeroY.toFixed(2)} ` +
    curvePoints.map((p) => `L ${p.x.toFixed(2)} ${p.y.toFixed(2)}`).join(" ") +
    ` L ${(PAD_LEFT + plotW).toFixed(2)} ${zeroY.toFixed(2)} Z`;

  // Pointer handlers — drag vertically to set gain; double-click resets.
  const handlePointerDown = useCallback(
    (band: BandId, event: React.PointerEvent<SVGElement>) => {
      event.preventDefault();
      const target = event.currentTarget;
      target.setPointerCapture(event.pointerId);
      setDragging(band);
    },
    [],
  );

  const handlePointerMove = useCallback(
    (band: BandId, event: React.PointerEvent<SVGElement>) => {
      if (dragging !== band || !svgRef.current) return;
      const svg = svgRef.current;
      // Use the SVG's CTM to translate clientX/Y into the viewBox's local
      // coordinate space — independent of CSS scaling or window resize.
      const pt = svg.createSVGPoint();
      pt.x = event.clientX;
      pt.y = event.clientY;
      const ctm = svg.getScreenCTM();
      if (!ctm) return;
      const local = pt.matrixTransform(ctm.inverse());
      const newDb = yToDbInPlot(local.y);
      onEq(band, Math.round(newDb * 10) / 10);
    },
    [dragging, onEq, yToDbInPlot],
  );

  const handlePointerUp = useCallback(
    (band: BandId, event: React.PointerEvent<SVGElement>) => {
      if (dragging !== band) return;
      const target = event.currentTarget;
      if (target.hasPointerCapture(event.pointerId)) {
        target.releasePointerCapture(event.pointerId);
      }
      setDragging(null);
    },
    [dragging],
  );

  const handleDoubleClick = useCallback(
    (band: BandId) => {
      onEq(band, 0);
    },
    [onEq],
  );

  return (
    <section className="visual-eq-panel" aria-label="Visual EQ">
      <header className="visual-eq-head">
        <span className="section-label">TONE CURVE</span>
        <span className="visual-eq-hint">Drag a node up or down · double-click to reset</span>
      </header>
      <svg
        ref={svgRef}
        className="eq-overlay"
        viewBox={`0 0 ${VBW} ${VBH}`}
        preserveAspectRatio="none"
      >
        {/* Grid: major frequency lines + minor sub-octave ticks. */}
        {GRID_FREQS.map((hz) => (
          <line
            key={`gx-${hz}`}
            className="eq-grid-major"
            x1={localFreqToX(hz)}
            x2={localFreqToX(hz)}
            y1={PAD_TOP}
            y2={PAD_TOP + plotH}
          />
        ))}
        {/* Horizontal dB grid. */}
        {GRID_DBS.map((db) => (
          <line
            key={`gy-${db}`}
            className={db === 0 ? "eq-zero-line" : "eq-grid-major"}
            x1={PAD_LEFT}
            x2={PAD_LEFT + plotW}
            y1={localDbToY(db)}
            y2={localDbToY(db)}
          />
        ))}
        {/* Frequency axis labels along the bottom (first row). */}
        {GRID_FREQS.map((hz) => (
          <text
            key={`fx-${hz}`}
            className="eq-label"
            x={localFreqToX(hz)}
            y={PAD_TOP + plotH + AXIS_LABEL_Y_OFFSET}
            textAnchor="middle"
          >
            {hz >= 1000 ? `${hz / 1000}k` : `${hz}`}
          </text>
        ))}
        {/* dB axis labels along the left edge. */}
        {GRID_DBS.map((db) => (
          <text
            key={`fy-${db}`}
            className="eq-label"
            x={PAD_LEFT - 8}
            y={localDbToY(db) + 4}
            textAnchor="end"
          >
            {db > 0 ? `+${db}` : `${db}`}
          </text>
        ))}
        {/* Response curve: fill under, then line on top. */}
        <path className="eq-response-fill" d={fillPath} />
        <path className="eq-response-line" d={curvePath} />
        {/* Per-band nodes. Each renders a colored dot + label; the
            invisible hit-target above (eq-node-hit) is twice the size
            so dragging is forgiving. */}
        {BANDS.map((band) => {
          const x = localFreqToX(band.hz);
          const y = localDbToY(gains[band.id]);
          const isDragging = dragging === band.id;
          return (
            <g key={band.id} style={{ "--node-color": band.color } as React.CSSProperties}>
              <circle
                className={`eq-node ${isDragging ? "is-dragging" : ""}`}
                cx={x}
                cy={y}
                r={7}
                fill={band.color}
              />
              <circle
                className="eq-node-hit"
                cx={x}
                cy={y}
                r={18}
                onPointerDown={(e) => handlePointerDown(band.id, e)}
                onPointerMove={(e) => handlePointerMove(band.id, e)}
                onPointerUp={(e) => handlePointerUp(band.id, e)}
                onPointerCancel={(e) => handlePointerUp(band.id, e)}
                onDoubleClick={() => handleDoubleClick(band.id)}
                style={{ cursor: "ns-resize", touchAction: "none" }}
              />
              <text
                className="eq-node-label"
                x={x}
                y={PAD_TOP + plotH + BAND_LABEL_Y_OFFSET}
                textAnchor="middle"
                fill={band.color}
              >
                {band.label}
              </text>
              <text
                className="eq-node-value"
                x={x}
                y={Math.max(PAD_TOP + 12, y - 12)}
                textAnchor="middle"
                fill={band.color}
              >
                {gains[band.id] > 0 ? `+${gains[band.id].toFixed(1)}` : gains[band.id].toFixed(1)}
              </text>
            </g>
          );
        })}
      </svg>
    </section>
  );
}
