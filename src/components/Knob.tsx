// UI-tighten — photoreal layered knob.
//
// Geometry: 270° usable sweep, dead 90° gap at the bottom. From outside in:
//   1. Recessed socket ring (dark linear gradient, suggests inset into panel).
//   2. Tick perimeter (24 minor + 5 major ticks; lit ticks track the value).
//   3. Accent arc (bright stroke from 0 to current value, with drop-shadow glow).
//   4. Metallic cap (radial gradient: light center → dark rim, soft top highlight
//      overlay simulates a light source from above the panel).
//   5. Indicator notch (bright accent line on the cap pointing to value).
//   6. Optional centered value text.
//
// Interaction (unchanged from the prior knob): pointer drag vertically (up
// increases, down decreases, hold Shift for fine), wheel to fine-tune,
// double-click to reset to defaultValue. Hidden range input for keyboard /
// screen-reader access.

import { useId, useRef, useState } from "react";

type KnobSize = "sm" | "md" | "lg";
export type KnobTone =
  | "blue"
  | "cyan"
  | "green"
  | "purple"
  | "pink"
  | "gold"
  | "orange"
  | "red";

// Solid color per tone — used for the accent arc, indicator notch, and the
// CSS variable that downstream halo/glow rules reference via color-mix.
const TONE_COLOR: Record<KnobTone, string> = {
  blue: "#4d8bff",
  cyan: "#22d3ee",
  green: "#34d399",
  purple: "#a78bfa",
  pink: "#f472b6",
  gold: "#fbbf24",
  orange: "#fb923c",
  red: "#f87171",
};

type KnobProps = {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  defaultValue?: number;
  size?: KnobSize;
  format: (v: number) => string;
  onChange: (v: number) => void;
  centerValue?: boolean;
  caption?: string;
  disabled?: boolean;
  /// Color identity for this knob's accent ring + indicator. Used to give
  /// each band a distinct character (Low=cyan, Mid=green, High=purple, etc.)
  /// like the reference UI. Defaults to blue (matches --accent-bright).
  tone?: KnobTone;
};

const SIZES: Record<KnobSize, { px: number; font: number; valueFont: number }> = {
  sm: { px: 56, font: 0.75, valueFont: 0.7 },
  md: { px: 76, font: 0.85, valueFont: 0.78 },
  lg: { px: 130, font: 1.7, valueFont: 1.7 },
};

const SWEEP_DEG = 270;
const START_ANGLE_DEG = -225;
const TICK_COUNT = 28;
const MAJOR_EVERY = 7; // every 7 ticks → 4 majors across the sweep + endpoints

export function Knob({
  label,
  value,
  min,
  max,
  step = 0.01,
  defaultValue,
  size = "md",
  format,
  onChange,
  centerValue = false,
  caption,
  disabled = false,
  tone = "blue",
}: KnobProps) {
  const dims = SIZES[size];
  const toneColor = TONE_COLOR[tone];
  const px = dims.px;
  const cx = px / 2;
  const cy = px / 2;

  const socketR = px / 2 - 1;
  const tickOuterR = socketR - 2;
  const tickInnerR = tickOuterR - px * 0.06;
  const tickMajorInnerR = tickOuterR - px * 0.09;
  const arcR = tickInnerR - px * 0.04;
  const capR = arcR - px * 0.07;
  const notchOuterR = capR - px * 0.03;
  const notchInnerR = capR * 0.42;

  const t = max > min ? Math.max(0, Math.min(1, (value - min) / (max - min))) : 0;
  const startAngle = (START_ANGLE_DEG * Math.PI) / 180;
  const endAngle = ((START_ANGLE_DEG + SWEEP_DEG * t) * Math.PI) / 180;
  const fullEndAngle = ((START_ANGLE_DEG + SWEEP_DEG) * Math.PI) / 180;
  const arcPath = describeArc(cx, cy, arcR, startAngle, endAngle, SWEEP_DEG * t);
  const trackPath = describeArc(cx, cy, arcR, startAngle, fullEndAngle, SWEEP_DEG);

  const id = useId().replace(/:/g, "_");

  const dragRef = useRef<{ startY: number; startValue: number } | null>(null);
  const [isDragging, setIsDragging] = useState(false);

  const handlePointerDown = (e: React.PointerEvent<SVGSVGElement>) => {
    if (disabled) return;
    e.preventDefault();
    (e.target as Element).setPointerCapture(e.pointerId);
    dragRef.current = { startY: e.clientY, startValue: value };
    setIsDragging(true);
  };

  const handlePointerMove = (e: React.PointerEvent<SVGSVGElement>) => {
    if (!dragRef.current) return;
    const dy = dragRef.current.startY - e.clientY;
    const sensitivity = e.shiftKey ? 1000 : 200;
    const delta = (dy / sensitivity) * (max - min);
    let next = dragRef.current.startValue + delta;
    next = Math.max(min, Math.min(max, next));
    if (step > 0) next = Math.round(next / step) * step;
    if (Math.abs(next - value) > step / 2) {
      onChange(parseFloat(next.toFixed(5)));
    }
  };

  const handlePointerUp = (e: React.PointerEvent<SVGSVGElement>) => {
    if (dragRef.current) {
      (e.target as Element).releasePointerCapture(e.pointerId);
      dragRef.current = null;
      setIsDragging(false);
    }
  };

  const handleWheel = (e: React.WheelEvent<SVGSVGElement>) => {
    if (disabled) return;
    e.preventDefault();
    const sensitivity = e.shiftKey ? 0.0005 : 0.002;
    const delta = -e.deltaY * sensitivity * (max - min);
    let next = value + delta;
    next = Math.max(min, Math.min(max, next));
    if (step > 0) next = Math.round(next / step) * step;
    onChange(parseFloat(next.toFixed(5)));
  };

  const handleDoubleClick = () => {
    if (disabled) return;
    if (defaultValue !== undefined) onChange(defaultValue);
  };

  // Tick mark geometry — pre-compute angles so the loop body stays tight.
  const ticks: { angle: number; major: boolean; active: boolean }[] = [];
  for (let i = 0; i < TICK_COUNT; i++) {
    const frac = i / (TICK_COUNT - 1);
    const angleDeg = START_ANGLE_DEG + SWEEP_DEG * frac;
    const angle = (angleDeg * Math.PI) / 180;
    const major = i % MAJOR_EVERY === 0 || i === TICK_COUNT - 1;
    const active = frac <= t + 1e-6;
    ticks.push({ angle, major, active });
  }

  const notchAngle = endAngle;
  const notchX1 = cx + notchInnerR * Math.cos(notchAngle);
  const notchY1 = cy + notchInnerR * Math.sin(notchAngle);
  const notchX2 = cx + notchOuterR * Math.cos(notchAngle);
  const notchY2 = cy + notchOuterR * Math.sin(notchAngle);

  return (
    <div
      className={`knob knob-${size} knob-tone-${tone} ${disabled ? "is-disabled" : ""}`}
      style={{ ["--knob-tone" as never]: toneColor }}
    >
      {label && <span className="knob-label">{label}</span>}
      <div className="knob-vis" style={{ width: px, height: px }}>
        <svg
          width={px}
          height={px}
          viewBox={`0 0 ${px} ${px}`}
          onPointerDown={handlePointerDown}
          onPointerMove={handlePointerMove}
          onPointerUp={handlePointerUp}
          onPointerCancel={handlePointerUp}
          onWheel={handleWheel}
          onDoubleClick={handleDoubleClick}
          className={isDragging ? "is-dragging" : ""}
          style={{
            touchAction: "none",
            cursor: disabled ? "default" : isDragging ? "grabbing" : "grab",
            overflow: "visible",
          }}
        >
          <defs>
            <linearGradient id={`socket-${id}`} x1="50%" y1="0%" x2="50%" y2="100%">
              <stop offset="0%" stopColor="#0c1018" />
              <stop offset="100%" stopColor="#1b212e" />
            </linearGradient>
            <radialGradient id={`cap-${id}`} cx="50%" cy="32%" r="65%">
              <stop offset="0%" stopColor="#dde3ee" />
              <stop offset="35%" stopColor="#aeb7c8" />
              <stop offset="70%" stopColor="#6c7689" />
              <stop offset="100%" stopColor="#3a4357" />
            </radialGradient>
            <radialGradient id={`gloss-${id}`} cx="50%" cy="18%" r="42%">
              <stop offset="0%" stopColor="rgba(255,255,255,0.55)" />
              <stop offset="55%" stopColor="rgba(255,255,255,0.08)" />
              <stop offset="100%" stopColor="rgba(255,255,255,0)" />
            </radialGradient>
            <linearGradient id={`cap-rim-${id}`} x1="50%" y1="0%" x2="50%" y2="100%">
              <stop offset="0%" stopColor="rgba(255,255,255,0.18)" />
              <stop offset="100%" stopColor="rgba(0,0,0,0.55)" />
            </linearGradient>
          </defs>

          {/* Recessed socket ring */}
          <circle
            cx={cx}
            cy={cy}
            r={socketR}
            fill={`url(#socket-${id})`}
            stroke="rgba(0,0,0,0.6)"
            strokeWidth={1}
          />

          {/* Tick perimeter — inactive first, then active over the top so
              glow doesn't darken neighboring ticks. */}
          {ticks.map((tk, i) => {
            const innerR = tk.major ? tickMajorInnerR : tickInnerR;
            const x1 = cx + innerR * Math.cos(tk.angle);
            const y1 = cy + innerR * Math.sin(tk.angle);
            const x2 = cx + tickOuterR * Math.cos(tk.angle);
            const y2 = cy + tickOuterR * Math.sin(tk.angle);
            return (
              <line
                key={`tk-${i}`}
                x1={x1}
                y1={y1}
                x2={x2}
                y2={y2}
                stroke={tk.active ? "var(--accent-bright)" : "rgba(120,135,165,0.32)"}
                strokeWidth={tk.major ? 2 : 1}
                strokeLinecap="round"
                opacity={tk.active ? 1 : 0.85}
              />
            );
          })}

          {/* Accent value arc (between ticks and cap) */}
          <path
            d={trackPath}
            fill="none"
            stroke="rgba(40,55,85,0.6)"
            strokeWidth={Math.max(2, px * 0.025)}
            strokeLinecap="round"
          />
          {t > 0 && (
            <path
              d={arcPath}
              fill="none"
              stroke={toneColor}
              strokeWidth={Math.max(2.5, px * 0.028)}
              strokeLinecap="round"
              style={{ filter: `drop-shadow(0 0 6px ${toneColor}cc)` }}
            />
          )}

          {/* Metallic cap base + gloss highlight */}
          <circle
            cx={cx}
            cy={cy}
            r={capR}
            fill={`url(#cap-${id})`}
          />
          <circle
            cx={cx}
            cy={cy}
            r={capR}
            fill={`url(#gloss-${id})`}
          />
          {/* Crisp rim around the cap so it reads as a real bevel. */}
          <circle
            cx={cx}
            cy={cy}
            r={capR}
            fill="none"
            stroke={`url(#cap-rim-${id})`}
            strokeWidth={1.5}
          />

          {/* Indicator notch — bright accent line + glow */}
          <line
            x1={notchX1}
            y1={notchY1}
            x2={notchX2}
            y2={notchY2}
            stroke={toneColor}
            strokeWidth={Math.max(2, px * 0.022)}
            strokeLinecap="round"
            style={{ filter: `drop-shadow(0 0 5px ${toneColor}e0)` }}
          />

          {centerValue && (
            <text
              x={cx}
              y={cy}
              textAnchor="middle"
              dominantBaseline="middle"
              fontSize={`${dims.valueFont}rem`}
              fontWeight={700}
              fill="rgba(15,18,26,0.92)"
              fontFamily="inherit"
              style={{ pointerEvents: "none", userSelect: "none", letterSpacing: "-0.02em" }}
            >
              {format(value)}
            </text>
          )}
        </svg>
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(parseFloat(e.target.value))}
          className="knob-fallback"
          aria-label={label}
          disabled={disabled}
        />
      </div>
      {!centerValue && <span className="knob-value">{format(value)}</span>}
      {caption && <span className="knob-caption">{caption}</span>}
    </div>
  );
}

function describeArc(
  cx: number,
  cy: number,
  r: number,
  startAngle: number,
  endAngle: number,
  sweepDeg: number,
): string {
  const x0 = cx + r * Math.cos(startAngle);
  const y0 = cy + r * Math.sin(startAngle);
  const x1 = cx + r * Math.cos(endAngle);
  const y1 = cy + r * Math.sin(endAngle);
  const largeArc = Math.abs(sweepDeg) > 180 ? 1 : 0;
  return `M ${x0} ${y0} A ${r} ${r} 0 ${largeArc} 1 ${x1} ${y1}`;
}

export function intensityLabel(v: number): string {
  if (v < 0.25) return "Subtle";
  if (v < 0.5) return "Restrained";
  if (v < 0.75) return "Moderate";
  if (v < 0.9) return "Driving";
  return "Aggressive";
}
