import {
  useState,
  type DragEvent as ReactDragEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { api } from "./lib/api";
import { useTrackMaster } from "./hooks/useTrackMaster";
import type {
  AnalysisResult,
  ImportedTrack,
  LoopRegion,
  MasteringSettings,
  Preset,
  WaveformPeaks,
  QualityCheck,
  QualityLevel,
} from "./bindings";
import type { ExportReceipt, PlaybackKindUI } from "./hooks/useTrackMaster";
import "./App.css";

const PRESET_OPTIONS: { value: Preset; label: string; blurb: string }[] = [
  { value: { kind: "universal" }, label: "Universal", blurb: "Safe, well-rounded default" },
  { value: { kind: "clarity" }, label: "Clarity", blurb: "Vocal/upper-mid definition" },
  { value: { kind: "tape" }, label: "Tape", blurb: "Saturation, glue, softer top" },
  { value: { kind: "spatial" }, label: "Spatial", blurb: "Width and depth" },
  { value: { kind: "oomph" }, label: "Oomph", blurb: "Low-end weight, punch" },
  { value: { kind: "warmth" }, label: "Warmth", blurb: "Fuller, smoother body" },
  { value: { kind: "punch" }, label: "Punch", blurb: "Transient impact" },
  { value: { kind: "loud" }, label: "Loud", blurb: "Density + level, with safety" },
];

function App() {
  const tm = useTrackMaster();

  return (
    <div className="app">
      <Sidebar
        tracks={tm.tracks}
        selectedId={tm.selectedTrackId}
        onSelect={tm.selectTrack}
        onRemove={tm.removeTrack}
        onAdd={tm.openImportDialog}
        isAnalyzing={tm.isAnalyzing}
        mode={tm.mode}
        onModeChange={tm.setMode}
        onReorder={tm.reorderTracks}
      />
      <main className="workspace">
        {tm.selectedTrack ? (
          <TrackMaster tm={tm} />
        ) : (
          <EmptyState onAdd={tm.openImportDialog} />
        )}
      </main>
      {tm.error && <Toast message={tm.error} onClose={tm.clearError} />}
      {tm.lastExportReceipt && (
        <ExportReceiptCard
          receipt={tm.lastExportReceipt}
          onClose={tm.clearExportReceipt}
        />
      )}
    </div>
  );
}

function Sidebar({
  tracks,
  selectedId,
  onSelect,
  onRemove,
  onAdd,
  isAnalyzing,
  mode,
  onModeChange,
  onReorder,
}: {
  tracks: ImportedTrack[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onRemove: (id: string) => void;
  onAdd: () => void;
  isAnalyzing: boolean;
  mode: "track" | "album";
  onModeChange: (mode: "track" | "album") => void;
  onReorder: (fromIndex: number, toIndex: number) => void;
}) {
  const [dragFromIndex, setDragFromIndex] = useState<number | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);

  const albumReorderable = mode === "album";

  const handleDragStart = (
    e: ReactDragEvent<HTMLLIElement>,
    index: number,
  ) => {
    if (!albumReorderable) return;
    setDragFromIndex(index);
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", String(index));
  };

  const handleDragOver = (
    e: ReactDragEvent<HTMLLIElement>,
    index: number,
  ) => {
    if (!albumReorderable || dragFromIndex === null) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "move";
    if (dragOverIndex !== index) setDragOverIndex(index);
  };

  const handleDrop = (e: ReactDragEvent<HTMLLIElement>, index: number) => {
    if (!albumReorderable || dragFromIndex === null) return;
    e.preventDefault();
    onReorder(dragFromIndex, index);
    setDragFromIndex(null);
    setDragOverIndex(null);
  };

  const handleDragEnd = () => {
    setDragFromIndex(null);
    setDragOverIndex(null);
  };

  return (
    <aside className="sidebar">
      <header className="sidebar-head">
        <span className="brand">Album Mastering Studio</span>
        <div className="mode-toggle">
          <button
            type="button"
            className={mode === "track" ? "on" : ""}
            onClick={() => onModeChange("track")}
          >
            Track Master
          </button>
          <button
            type="button"
            className={mode === "album" ? "on" : ""}
            onClick={() => onModeChange("album")}
          >
            Album Master
          </button>
        </div>
      </header>

      <div className="sidebar-section">
        <span className="section-label">
          {mode === "album" ? `Album order (${tracks.length})` : `Tracks (${tracks.length})`}
        </span>
        <button type="button" className="add-btn" onClick={onAdd}>
          + Add files
        </button>
      </div>

      <ul className="track-list">
        {tracks.length === 0 && (
          <li className="track-empty">
            {mode === "album"
              ? "No album yet. Drop or add tracks, then drag to reorder."
              : "No tracks yet. Drop or add audio."}
          </li>
        )}
        {tracks.map((t, index) => {
          const classes = ["track-row"];
          if (t.id === selectedId) classes.push("active");
          if (dragFromIndex === index) classes.push("dragging");
          if (dragOverIndex === index && dragFromIndex !== index)
            classes.push("drag-over");
          return (
            <li
              key={t.id}
              className={classes.join(" ")}
              draggable={albumReorderable}
              onDragStart={(e) => handleDragStart(e, index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDrop={(e) => handleDrop(e, index)}
              onDragEnd={handleDragEnd}
              onDragLeave={() => setDragOverIndex(null)}
            >
              {albumReorderable && (
                <span className="track-index" aria-hidden>
                  {index + 1}
                </span>
              )}
              <button
                type="button"
                className="track-pick"
                onClick={() => onSelect(t.id)}
                title={t.path}
              >
                <span className="track-name">{t.display_name}</span>
                <span className="track-meta">.{t.source_format}</span>
              </button>
              <button
                type="button"
                className="track-remove"
                onClick={() => onRemove(t.id)}
                aria-label="Remove track"
                title="Remove"
              >
                ×
              </button>
            </li>
          );
        })}
      </ul>

      {isAnalyzing && <div className="sidebar-status">Analyzing…</div>}
    </aside>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="empty-state">
      <h1>Drop audio, analyze, export.</h1>
      <p>
        Album Mastering Studio masters one track or a full album. Universal-first
        — no genre wizard, no jargon walls.
      </p>
      <button type="button" className="primary" onClick={onAdd}>
        Add files
      </button>
      <p className="empty-foot">
        See <code>docs/PRODUCT.md</code> for the product canon.
      </p>
    </div>
  );
}

function TrackMaster({ tm }: { tm: ReturnType<typeof useTrackMaster> }) {
  const track = tm.selectedTrack;
  if (!track) return null;
  return (
    <>
      <TrackHeader
        track={track}
        analysis={tm.selectedAnalysis}
        isAnalyzing={tm.isAnalyzing}
      />
      <WaveformView
        peaks={tm.selectedWaveform}
        isLoading={tm.isLoadingWaveform}
        currentTimeSec={tm.transport.currentTimeSec}
        durationSec={track.duration_seconds ?? 180}
        region={tm.selectedRegion}
        onSeek={tm.seek}
        onSetRegion={tm.setRegion}
        onClearRegion={tm.clearRegion}
      />
      <Transport
        isPlaying={tm.transport.isPlaying}
        playbackKind={tm.transport.playbackKind}
        loop={tm.transport.loop}
        volumeMatch={tm.transport.volumeMatch}
        durationSec={track.duration_seconds ?? 180}
        currentSec={tm.transport.currentTimeSec}
        onPlayPause={tm.togglePlay}
        onPlaybackKindChange={tm.setPlaybackKind}
        onLoopToggle={tm.toggleLoop}
        onVolumeMatchChange={tm.setVolumeMatch}
      />
      <PresetTiles
        selected={tm.selectedSettings.preset}
        onChange={tm.setPreset}
      />
      <Macros
        settings={tm.selectedSettings}
        onIntensity={tm.setIntensity}
        onEq={tm.setEqBand}
      />
      <StaleBar
        stale={tm.previewStale}
        isRendering={tm.isRendering}
        onUpdate={tm.updatePreview}
      />
      <ExportSection
        canExport={!!tm.selectedAnalysis}
        isExporting={tm.isExporting}
        advancedOpen={tm.advancedOpen}
        onToggleAdvanced={tm.toggleAdvanced}
        onExport={tm.exportMaster}
      />
      {tm.advancedOpen && (
        <AdvancedPanel
          settings={tm.selectedSettings}
          onAdvanced={tm.setAdvanced}
        />
      )}
    </>
  );
}

function TrackHeader({
  track,
  analysis,
  isAnalyzing,
}: {
  track: ImportedTrack;
  analysis: AnalysisResult | undefined;
  isAnalyzing: boolean;
}) {
  return (
    <section className="track-header">
      <div>
        <h1 className="track-title">{track.display_name}</h1>
        <div className="track-sub">
          <span>.{track.source_format}</span>
          {analysis && (
            <>
              <span className="dot">•</span>
              <span>LUFS {analysis.lufs_integrated.toFixed(1)}</span>
              <span className="dot">•</span>
              <span>TP {analysis.true_peak_dbtp.toFixed(2)} dBTP</span>
              <span className="dot">•</span>
              <span>DR {analysis.dynamic_range_lu.toFixed(1)} LU</span>
              <span className="dot">•</span>
              <span>W {analysis.stereo_width.toFixed(2)}</span>
            </>
          )}
        </div>
      </div>
      <div className="track-badge">
        {isAnalyzing ? "Analyzing…" : analysis ? "Analyzed" : "Pending"}
      </div>
    </section>
  );
}

function WaveformView({
  peaks,
  isLoading,
  currentTimeSec,
  durationSec,
  region,
  onSeek,
  onSetRegion,
  onClearRegion,
}: {
  peaks: WaveformPeaks | undefined;
  isLoading: boolean;
  currentTimeSec: number;
  durationSec: number;
  region: LoopRegion | null;
  onSeek: (positionSec: number) => void;
  onSetRegion: (region: LoopRegion) => void;
  onClearRegion: () => void;
}) {
  const [dragRegion, setDragRegion] = useState<LoopRegion | null>(null);

  if (isLoading || !peaks) {
    return (
      <section className="wf-card">
        <div className="wf-empty">{isLoading ? "Loading waveform…" : "No waveform yet."}</div>
      </section>
    );
  }
  const channel = peaks.channels[0] ?? [];
  const W = 1000;
  const H = 240;
  const playheadX =
    durationSec > 0
      ? Math.max(0, Math.min(W, (currentTimeSec / durationSec) * W))
      : 0;

  const timeAtPointer = (e: ReactPointerEvent<SVGSVGElement>): number => {
    const rect = e.currentTarget.getBoundingClientRect();
    if (rect.width <= 0 || durationSec <= 0) return 0;
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    return ratio * durationSec;
  };

  const handlePointerDown = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (durationSec <= 0) return;
    const t = timeAtPointer(e);
    if (e.shiftKey) {
      e.preventDefault();
      try {
        e.currentTarget.setPointerCapture(e.pointerId);
      } catch {
        /* setPointerCapture can throw on some platforms; we still track via state */
      }
      setDragRegion({ start_sec: t, end_sec: t });
    } else {
      onSeek(t);
    }
  };

  const handlePointerMove = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (!dragRegion) return;
    const t = timeAtPointer(e);
    setDragRegion({ start_sec: dragRegion.start_sec, end_sec: t });
  };

  const handlePointerUp = (_e: ReactPointerEvent<SVGSVGElement>) => {
    if (!dragRegion) return;
    const start = Math.min(dragRegion.start_sec, dragRegion.end_sec);
    const end = Math.max(dragRegion.start_sec, dragRegion.end_sec);
    const meaningfulDrag =
      durationSec > 0 && end - start > Math.max(0.1, durationSec * 0.005);
    if (meaningfulDrag) {
      onSetRegion({ start_sec: start, end_sec: end });
    } else if (region) {
      onClearRegion();
    }
    setDragRegion(null);
  };

  const displayRegion: LoopRegion | null = dragRegion ?? region;
  const regionRect = displayRegion && durationSec > 0
    ? (() => {
        const startX = Math.max(
          0,
          Math.min(W, (Math.min(displayRegion.start_sec, displayRegion.end_sec) / durationSec) * W),
        );
        const endX = Math.max(
          0,
          Math.min(W, (Math.max(displayRegion.start_sec, displayRegion.end_sec) / durationSec) * W),
        );
        return { startX, endX };
      })()
    : null;

  return (
    <section className="wf-card">
      <svg
        className="wf"
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        role="slider"
        aria-valuemin={0}
        aria-valuemax={durationSec}
        aria-valuenow={currentTimeSec}
      >
        {channel.map((v, i) => {
          const x = (i / channel.length) * W;
          const barW = (W / channel.length) * 0.85;
          const barH = v * (H * 0.88);
          const y = (H - barH) / 2;
          return <rect key={i} x={x} y={y} width={barW} height={barH} rx={0.5} />;
        })}
        {regionRect && (
          <rect
            className="wf-region"
            x={regionRect.startX}
            y={0}
            width={Math.max(1, regionRect.endX - regionRect.startX)}
            height={H}
          />
        )}
        <line
          className="wf-playhead"
          x1={playheadX}
          y1={0}
          x2={playheadX}
          y2={H}
        />
      </svg>
      <p className="wf-hint">
        Click to seek. Shift+drag to define a loop region. Shift+click clears it.
      </p>
    </section>
  );
}

function Transport({
  isPlaying,
  playbackKind,
  loop,
  volumeMatch,
  durationSec,
  currentSec,
  onPlayPause,
  onPlaybackKindChange,
  onLoopToggle,
  onVolumeMatchChange,
}: {
  isPlaying: boolean;
  playbackKind: PlaybackKindUI;
  loop: boolean;
  volumeMatch: boolean;
  durationSec: number;
  currentSec: number;
  onPlayPause: () => void;
  onPlaybackKindChange: (kind: PlaybackKindUI) => void;
  onLoopToggle: () => void;
  onVolumeMatchChange: (on: boolean) => void;
}) {
  return (
    <section className="transport">
      <div className="transport-left">
        <button
          type="button"
          className="play-btn"
          onClick={onPlayPause}
          aria-label={isPlaying ? "Pause" : "Play"}
        >
          {isPlaying ? "⏸" : "▶"}
        </button>
        <span className="time">
          {formatTime(currentSec)} <span className="dim">/ {formatTime(durationSec)}</span>
        </span>
        <button
          type="button"
          className={"icon-btn " + (loop ? "on" : "")}
          onClick={onLoopToggle}
          title="Loop region"
        >
          ⟲
        </button>
      </div>
      <div className="transport-right">
        <div className="ab-toggle">
          <button
            type="button"
            className={playbackKind === "source" ? "on" : ""}
            onClick={() => onPlaybackKindChange("source")}
          >
            Original
          </button>
          <button
            type="button"
            className={playbackKind === "master" ? "on" : ""}
            onClick={() => onPlaybackKindChange("master")}
          >
            Mastered
          </button>
        </div>
        <label className="vm-toggle" title="Aligns playback loudness for fair tone comparison. Export level is unchanged.">
          <input
            type="checkbox"
            checked={volumeMatch}
            onChange={(e) => onVolumeMatchChange(e.target.checked)}
          />
          <span>Volume Match</span>
        </label>
      </div>
    </section>
  );
}

function PresetTiles({
  selected,
  onChange,
}: {
  selected: Preset;
  onChange: (preset: Preset) => void;
}) {
  return (
    <section className="presets">
      <div className="section-head">
        <span className="section-label">Preset</span>
      </div>
      <div className="tile-row">
        {PRESET_OPTIONS.map((p) => {
          const active = isPresetActive(selected, p.value);
          return (
            <button
              key={p.label}
              type="button"
              className={"tile " + (active ? "active" : "")}
              onClick={() => onChange(p.value)}
            >
              <span className="tile-label">{p.label}</span>
              <span className="tile-blurb">{p.blurb}</span>
            </button>
          );
        })}
      </div>
    </section>
  );
}

function isPresetActive(a: Preset, b: Preset): boolean {
  if (a.kind === "custom" && b.kind === "custom") return a.id === b.id;
  return a.kind === b.kind;
}

function Macros({
  settings,
  onIntensity,
  onEq,
}: {
  settings: MasteringSettings;
  onIntensity: (v: number) => void;
  onEq: (band: "low" | "mid" | "high", db: number) => void;
}) {
  return (
    <section className="macros">
      <Slider
        label="Intensity"
        value={settings.intensity}
        min={0}
        max={1}
        step={0.01}
        format={(v) => v.toFixed(2)}
        onChange={onIntensity}
      />
      <Slider
        label="Low"
        value={settings.eq_low_db}
        min={-6}
        max={6}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={(v) => onEq("low", v)}
      />
      <Slider
        label="Mid"
        value={settings.eq_mid_db}
        min={-6}
        max={6}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={(v) => onEq("mid", v)}
      />
      <Slider
        label="High"
        value={settings.eq_high_db}
        min={-6}
        max={6}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={(v) => onEq("high", v)}
      />
    </section>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <div className="slider-row">
      <label className="slider-label">{label}</label>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(parseFloat(e.target.value))}
        className="slider-input"
      />
      <span className="slider-value">{format(value)}</span>
    </div>
  );
}

function StaleBar({
  stale,
  isRendering,
  onUpdate,
}: {
  stale: boolean;
  isRendering: boolean;
  onUpdate: () => void;
}) {
  return (
    <section className="stale-bar">
      <span className={"stale-dot " + (stale ? "stale" : "fresh")} aria-hidden />
      <span className="stale-text">
        {isRendering
          ? "Rendering preview…"
          : stale
            ? "Preview is stale — settings changed since last render."
            : "Preview matches current settings."}
      </span>
      <button
        type="button"
        className="ghost-btn"
        onClick={onUpdate}
        disabled={isRendering || !stale}
      >
        Update preview
      </button>
    </section>
  );
}

function ExportSection({
  canExport,
  isExporting,
  advancedOpen,
  onToggleAdvanced,
  onExport,
}: {
  canExport: boolean;
  isExporting: boolean;
  advancedOpen: boolean;
  onToggleAdvanced: () => void;
  onExport: () => void;
}) {
  return (
    <section className="export-bar">
      <button
        type="button"
        className="primary export-btn"
        onClick={onExport}
        disabled={!canExport || isExporting}
      >
        {isExporting ? "Exporting…" : "Export Master"}
      </button>
      <button type="button" className="advanced-toggle" onClick={onToggleAdvanced}>
        {advancedOpen ? "▲ Hide advanced" : "▼ Advanced"}
      </button>
    </section>
  );
}

function AdvancedPanel({
  settings,
  onAdvanced,
}: {
  settings: MasteringSettings;
  onAdvanced: (adv: MasteringSettings["advanced"]) => void;
}) {
  const a = settings.advanced;
  const update = (
    field: keyof MasteringSettings["advanced"],
    value: number | null,
  ) => {
    onAdvanced({ ...a, [field]: value });
  };
  return (
    <section className="advanced">
      <div className="section-head">
        <span className="section-label">Advanced</span>
      </div>
      <div className="advanced-grid">
        <NumberField
          label="LUFS target"
          value={a.lufs_offset_db}
          step={0.5}
          min={-24}
          max={-6}
          format={(v) => `${v.toFixed(1)} LUFS`}
          onChange={(v) => update("lufs_offset_db", v)}
        />
        <NumberField
          label="Ceiling"
          value={a.ceiling_dbtp}
          step={0.1}
          min={-3}
          max={0}
          format={(v) => `${v.toFixed(1)} dBTP`}
          onChange={(v) => update("ceiling_dbtp", v)}
        />
        <NumberField
          label="Width"
          value={a.width}
          step={0.05}
          min={0}
          max={1.5}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("width", v)}
        />
        <NumberField
          label="Warmth"
          value={a.warmth}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("warmth", v)}
        />
        <NumberField
          label="Presence/Air"
          value={a.presence_air}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("presence_air", v)}
        />
        <NumberField
          label="Compression"
          value={a.compression_density}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("compression_density", v)}
        />
        <SelectField
          label="Bit depth"
          value={a.bit_depth}
          options={[
            { value: null, label: "Auto" },
            { value: 16, label: "16-bit" },
            { value: 24, label: "24-bit" },
            { value: 32, label: "32-bit float" },
          ]}
          onChange={(v) => update("bit_depth", v)}
        />
        <SelectField
          label="Sample rate"
          value={a.target_sample_rate}
          options={[
            { value: null, label: "Source" },
            { value: 44100, label: "44.1 kHz" },
            { value: 48000, label: "48 kHz" },
            { value: 88200, label: "88.2 kHz" },
            { value: 96000, label: "96 kHz" },
          ]}
          onChange={(v) => update("target_sample_rate", v)}
        />
      </div>
    </section>
  );
}

function NumberField({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: {
  label: string;
  value: number | null;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onChange: (v: number | null) => void;
}) {
  const effective = value ?? min;
  return (
    <div className="adv-field">
      <span className="adv-label">{label}</span>
      <div className="adv-control">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={effective}
          onChange={(e) => onChange(parseFloat(e.target.value))}
          disabled={value === null}
        />
        <button
          type="button"
          className="micro-btn"
          onClick={() => onChange(value === null ? (min + max) / 2 : null)}
        >
          {value === null ? "Set" : "Auto"}
        </button>
        <span className="adv-value">{value === null ? "Auto" : format(value)}</span>
      </div>
    </div>
  );
}

function SelectField({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: number | null;
  options: { value: number | null; label: string }[];
  onChange: (v: number | null) => void;
}) {
  return (
    <div className="adv-field">
      <span className="adv-label">{label}</span>
      <select
        className="adv-select"
        value={value === null ? "" : String(value)}
        onChange={(e) => {
          const v = e.target.value;
          onChange(v === "" ? null : Number(v));
        }}
      >
        {options.map((o) => (
          <option key={o.label} value={o.value === null ? "" : String(o.value)}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function Toast({
  message,
  onClose,
}: {
  message: string;
  onClose: () => void;
}) {
  return (
    <div className="toast">
      <span>{message}</span>
      <button type="button" className="toast-close" onClick={onClose} aria-label="Dismiss">
        ×
      </button>
    </div>
  );
}

function ExportReceiptCard({
  receipt,
  onClose,
}: {
  receipt: ExportReceipt;
  onClose: () => void;
}) {
  const reveal = async () => {
    if (!receipt.outputPath) return;
    try {
      await api.openOutput(receipt.outputPath);
    } catch (err) {
      console.error("openOutput failed", err);
    }
  };
  return (
    <div className="receipt-backdrop" onClick={onClose}>
      <div className="receipt" onClick={(e) => e.stopPropagation()}>
        <header>
          <h2>Export complete</h2>
          <button type="button" className="toast-close" onClick={onClose} aria-label="Close">
            ×
          </button>
        </header>
        <button
          type="button"
          className="receipt-path"
          onClick={reveal}
          title="Reveal in file manager"
        >
          {receipt.outputPath}
        </button>
        <div className="receipt-checks">
          {receipt.checks.map((c, i) => (
            <CheckRow key={i} check={c} />
          ))}
        </div>
      </div>
    </div>
  );
}

function CheckRow({ check }: { check: QualityCheck }) {
  return (
    <div className={"check-row level-" + levelClass(check.level)}>
      <span className="check-level">{check.level}</span>
      <span className="check-msg">{check.message}</span>
    </div>
  );
}

function levelClass(level: QualityLevel): string {
  return level;
}

function formatTime(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

export default App;
