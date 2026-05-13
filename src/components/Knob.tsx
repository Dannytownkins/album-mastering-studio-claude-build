// Phase 12.2 P2 — knob control for Intensity / Tone Shape / advanced params.
//
// Visual: 270° SVG arc, foreground filled from -135° up to the value angle.
// Interaction: pointer-down + drag vertically (up increases, down decreases),
// wheel to fine-tune, double-click to reset to default. The hidden <input
// type="range"> keeps keyboard + screen-reader access working for free.

import { useRef, useState } from "react";

type KnobSize = "sm" | "md" | "lg";

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
  // Centered numeric instead of below — used for the big Intensity knob.
  centerValue?: boolean;
  // Optional caption under the value (e.g., "Moderate" for Intensity@60%).
  caption?: string;
  // Disabled / inactive state — dims the knob; pointer interaction off.
  disabled?: boolean;
};

const SIZES: Record<KnobSize, { px: number; stroke: number; font: number }> = {
  sm: { px: 56, stroke: 5, font: 0.75 },
  md: { px: 72, stroke: 6, font: 0.85 },
  lg: { px: 108, stroke: 8, font: 1.6 },
};

// 270° sweep total — leaves a 90° dead zone at the bottom for the gap.
const SWEEP_DEG = 270;
const START_ANGLE_DEG = -225; // -135 below the horizontal, going clockwise.

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
}: KnobProps) {
  const dims = SIZES[size];
  const r = (dims.px - dims.stroke) / 2;
  const cx = dims.px / 2;
  const cy = dims.px / 2;

  // Normalize value to [0, 1] for arc length.
  const t = max > min ? Math.max(0, Math.min(1, (value - min) / (max - min))) : 0;

  const startAngle = (START_ANGLE_DEG * Math.PI) / 180;
  const endAngle = ((START_ANGLE_DEG + SWEEP_DEG * t) * Math.PI) / 180;
  const fullEndAngle = ((START_ANGLE_DEG + SWEEP_DEG) * Math.PI) / 180;

  const arcPath = describeArc(cx, cy, r, startAngle, endAngle, SWEEP_DEG * t);
  const trackPath = describeArc(cx, cy, r, startAngle, fullEndAngle, SWEEP_DEG);

  // Pointer line at end of arc.
  const pointerX = cx + r * Math.cos(endAngle);
  const pointerY = cy + r * Math.sin(endAngle);
  const innerR = r * 0.55;
  const pointerInnerX = cx + innerR * Math.cos(endAngle);
  const pointerInnerY = cy + innerR * Math.sin(endAngle);

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
    // 200 px of drag = full range. Hold shift for fine (5x slower).
    const sensitivity = e.shiftKey ? 1000 : 200;
    const delta = (dy / sensitivity) * (max - min);
    let next = dragRef.current.startValue + delta;
    next = Math.max(min, Math.min(max, next));
    if (step > 0) {
      next = Math.round(next / step) * step;
    }
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
    if (step > 0) {
      next = Math.round(next / step) * step;
    }
    onChange(parseFloat(next.toFixed(5)));
  };

  const handleDoubleClick = () => {
    if (disabled) return;
    if (defaultValue !== undefined) {
      onChange(defaultValue);
    }
  };

  return (
    <div className={`knob knob-${size} ${disabled ? "is-disabled" : ""}`}>
      <span className="knob-label">{label}</span>
      <div className="knob-vis" style={{ width: dims.px, height: dims.px }}>
        <svg
          width={dims.px}
          height={dims.px}
          viewBox={`0 0 ${dims.px} ${dims.px}`}
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
          }}
        >
          <path
            d={trackPath}
            fill="none"
            stroke="var(--knob-track, var(--bg-3))"
            strokeWidth={dims.stroke}
            strokeLinecap="round"
          />
          {t > 0 && (
            <path
              d={arcPath}
              fill="none"
              stroke="var(--knob-fill, var(--accent))"
              strokeWidth={dims.stroke}
              strokeLinecap="round"
            />
          )}
          <line
            x1={pointerInnerX}
            y1={pointerInnerY}
            x2={pointerX}
            y2={pointerY}
            stroke="var(--knob-pointer, var(--text-0))"
            strokeWidth={Math.max(1.5, dims.stroke * 0.45)}
            strokeLinecap="round"
          />
          {centerValue && (
            <text
              x={cx}
              y={cy}
              textAnchor="middle"
              dominantBaseline="middle"
              fontSize={`${dims.font}rem`}
              fontWeight={600}
              fill="var(--text-0)"
              fontFamily="inherit"
              style={{ pointerEvents: "none", userSelect: "none" }}
            >
              {format(value)}
            </text>
          )}
        </svg>
        {/* Hidden range input for keyboard accessibility (Tab + arrow keys). */}
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
      {!centerValue && (
        <span className="knob-value">{format(value)}</span>
      )}
      {caption && <span className="knob-caption">{caption}</span>}
    </div>
  );
}

// Build an SVG arc path from start angle to end angle (radians) around (cx, cy)
// with radius r. sweepDeg is the angular distance covered (for choosing the
// large-arc flag).
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

// Helper for Intensity caption: classify a 0..1 macro into a few human labels.
export function intensityLabel(v: number): string {
  if (v < 0.25) return "Subtle";
  if (v < 0.5) return "Restrained";
  if (v < 0.75) return "Moderate";
  if (v < 0.9) return "Driving";
  return "Aggressive";
}
