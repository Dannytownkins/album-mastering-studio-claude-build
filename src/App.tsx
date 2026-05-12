import {
  useEffect,
  useRef,
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
  UserPreset,
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
        overrideAlbum={tm.overrideAlbum}
      />
      <main className="workspace">
        {tm.mode === "album" && tm.tracks.length > 0 && (
          <AlbumHeader
            tracks={tm.tracks}
            isExporting={tm.isExportingAlbum}
            onExport={tm.exportAlbum}
          />
        )}
        {tm.selectedTrack ? (
          <TrackMaster tm={tm} />
        ) : (
          <EmptyState onAdd={tm.openImportDialog} />
        )}
      </main>
      {tm.isDragOver && (
        <div className="drop-overlay" aria-hidden>
          <div className="drop-overlay-card">
            <div className="drop-overlay-title">Drop to import</div>
            <div className="drop-overlay-hint">
              WAV · AIFF · FLAC · MP3 · M4A · AAC · OGG · Opus
            </div>
          </div>
        </div>
      )}
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
  overrideAlbum,
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
  overrideAlbum: Set<string>;
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
                <span className="track-name">
                  {t.display_name}
                  {mode === "album" && overrideAlbum.has(t.id) && (
                    <span className="override-mark" title="Overrides album intent">
                      ★
                    </span>
                  )}
                </span>
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
      {tm.mode === "album" && (
        <OverrideBanner
          isOverriding={tm.selectedIsOverriding}
          onToggle={() => tm.toggleOverrideAlbum(track.id)}
        />
      )}
      <TrackHeader
        track={track}
        analysis={tm.selectedAnalysis}
        isAnalyzing={tm.isAnalyzing}
        showStoryTags={tm.mode === "album"}
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
        loopEnabled={!!tm.selectedRegion}
        onPlayPause={tm.togglePlay}
        onPlaybackKindChange={tm.setPlaybackKind}
        onLoopToggle={tm.toggleLoop}
        onVolumeMatchChange={tm.setVolumeMatch}
      />
      <IOGainBar
        inputGainDb={tm.selectedSettings.input_gain_db}
        outputGainDb={tm.selectedSettings.output_gain_db}
        onInputGain={tm.setInputGain}
        onOutputGain={tm.setOutputGain}
      />
      <PresetTiles
        selected={tm.selectedSettings.preset}
        onChange={tm.setPreset}
      />
      <UserPresetSection
        presets={tm.userPresets}
        savingPreset={tm.savingPreset}
        onSave={tm.saveUserPreset}
        onDelete={tm.deleteUserPreset}
        onApply={tm.applyUserPreset}
      />
      <Macros
        settings={tm.selectedSettings}
        onIntensity={tm.setIntensity}
        onEq={tm.setEqBand}
      />
      <UndoRedoBar
        canUndo={tm.canUndo}
        canRedo={tm.canRedo}
        onUndo={tm.undo}
        onRedo={tm.redo}
      />
      <StaleBar
        stale={tm.previewStale}
        isRendering={tm.isRendering}
        onUpdate={tm.updatePreview}
        liveUpdateStats={tm.liveUpdateStats}
        renderProgress={tm.renderProgress}
        peakDbfs={tm.transport.peakDbfs}
        isPlaying={tm.transport.isPlaying}
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

function OverrideBanner({
  isOverriding,
  onToggle,
}: {
  isOverriding: boolean;
  onToggle: () => void;
}) {
  return (
    <section className={"override-banner " + (isOverriding ? "is-overriding" : "follows")}>
      <div className="override-info">
        <span className="section-label">Album adaptation</span>
        <span className="override-state">
          {isOverriding
            ? "This track overrides album intent · its own settings will be applied at export"
            : "This track follows album intent · edits below change the album for every following track"}
        </span>
      </div>
      <div className="override-toggle">
        <button
          type="button"
          className={!isOverriding ? "on" : ""}
          onClick={onToggle}
          disabled={!isOverriding}
        >
          Follow album
        </button>
        <button
          type="button"
          className={isOverriding ? "on" : ""}
          onClick={onToggle}
          disabled={isOverriding}
        >
          Override
        </button>
      </div>
    </section>
  );
}

function TrackHeader({
  track,
  analysis,
  isAnalyzing,
  showStoryTags,
}: {
  track: ImportedTrack;
  analysis: AnalysisResult | undefined;
  isAnalyzing: boolean;
  showStoryTags: boolean;
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
        {analysis && <AnalysisSummary analysis={analysis} />}
        {showStoryTags && analysis && (
          <StoryTags analysis={analysis} />
        )}
      </div>
      <div className="track-badge">
        {isAnalyzing ? "Analyzing…" : analysis ? "Analyzed" : "Pending"}
      </div>
    </section>
  );
}

/// Plain-English commentary on the analysis numbers — one line per dimension.
/// Phase 12.1 Dan feedback: "A more prominent assessment of what was done
/// after analyzation, even perhaps in plain English in a dropdown underneath
/// the stats." Each line maps a numeric range to a short, non-alarmist phrase.
function AnalysisSummary({ analysis }: { analysis: AnalysisResult }) {
  const lines: string[] = [];

  // Loudness commentary.
  const lufs = analysis.lufs_integrated;
  if (lufs > -8) {
    lines.push(
      `Very loud at ${lufs.toFixed(1)} LUFS — may sound flat vs streaming references.`,
    );
  } else if (lufs > -12) {
    lines.push(`Loud at ${lufs.toFixed(1)} LUFS — streaming-loud territory.`);
  } else if (lufs > -16) {
    lines.push(`${lufs.toFixed(1)} LUFS — close to typical streaming targets.`);
  } else {
    lines.push(`${lufs.toFixed(1)} LUFS — conservative loudness, room to push.`);
  }

  // Dynamics commentary.
  const dr = analysis.dynamic_range_lu;
  if (dr < 5) {
    lines.push(`Highly compressed (DR ${dr.toFixed(1)} LU) — limited dynamic contrast.`);
  } else if (dr < 8) {
    lines.push(`Moderately compressed (DR ${dr.toFixed(1)} LU).`);
  } else {
    lines.push(`Healthy dynamic range (DR ${dr.toFixed(1)} LU).`);
  }

  // Spectral commentary.
  const high = analysis.spectral_balance.high;
  const low = analysis.spectral_balance.low;
  if (high > 0.35) {
    lines.push("Bright, presence-forward spectrum.");
  } else if (high < 0.18) {
    lines.push("Dark, low-mid-focused spectrum.");
  } else if (low > 0.45) {
    lines.push("Low-heavy spectrum.");
  } else {
    lines.push("Balanced spectrum.");
  }

  // Stereo width commentary.
  const w = analysis.stereo_width;
  if (w > 0.7) {
    lines.push("Wide stereo image.");
  } else if (w < 0.3) {
    lines.push("Narrow / mono-leaning stereo image.");
  } else {
    lines.push("Standard stereo image.");
  }

  // True peak commentary.
  const tp = analysis.true_peak_dbtp;
  if (tp > -0.1) {
    lines.push(`True peak ${tp.toFixed(2)} dBTP — at or above the digital ceiling, risky.`);
  } else if (tp > -1.0) {
    lines.push(`True peak ${tp.toFixed(2)} dBTP — fine digitally, lossy codecs may overshoot.`);
  } else {
    lines.push(`True peak ${tp.toFixed(2)} dBTP — comfortable headroom.`);
  }

  return (
    <details className="analysis-summary">
      <summary>Details</summary>
      <ul>
        {lines.map((line, i) => (
          <li key={i}>{line}</li>
        ))}
      </ul>
    </details>
  );
}

function StoryTags({ analysis }: { analysis: AnalysisResult }) {
  const role = analysis.inferred_role;
  const roleConf = analysis.role_confidence;
  const character = analysis.inferred_character;
  const charConf = analysis.character_confidence;
  if (!role && !character) return null;
  return (
    <div className="story-tags">
      {role && (
        <span
          className={"tag tag-role conf-" + (roleConf ?? "unsure")}
          title={`Inferred role · ${confidenceLabel(roleConf)}`}
        >
          {humbleVerb(roleConf)} {roleLabel(role)}
        </span>
      )}
      {character && (
        <span
          className={"tag tag-character conf-" + (charConf ?? "unsure")}
          title={`Inferred character · ${confidenceLabel(charConf)}`}
        >
          {humbleVerb(charConf)} {characterLabel(character)}
        </span>
      )}
    </div>
  );
}

function humbleVerb(conf: AnalysisResult["role_confidence"]): string {
  switch (conf) {
    case "strong":
      return "Likely";
    case "moderate":
      return "Appears";
    case "unsure":
    case undefined:
    case null:
    default:
      return "Maybe";
  }
}

function roleLabel(role: NonNullable<AnalysisResult["inferred_role"]>): string {
  switch (role) {
    case "opener":
      return "opener";
    case "closer":
      return "closer";
    case "single":
      return "a single";
    case "ballad":
      return "a ballad";
    case "interlude":
      return "an interlude";
    case "album_track":
      return "an album track";
    default:
      return "an album track";
  }
}

function characterLabel(
  c: NonNullable<AnalysisResult["inferred_character"]>,
): string {
  switch (c) {
    case "bright":
      return "bright";
    case "dark":
      return "dark";
    case "dense":
      return "dense";
    case "sparse":
      return "sparse";
    case "balanced":
      return "balanced";
    default:
      return "balanced";
  }
}

function confidenceLabel(
  conf: AnalysisResult["role_confidence"],
): string {
  switch (conf) {
    case "strong":
      return "strong";
    case "moderate":
      return "moderate";
    case "unsure":
    case undefined:
    case null:
    default:
      return "unsure";
  }
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
  loopEnabled,
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
  loopEnabled: boolean;
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
          disabled={!loopEnabled}
          title={
            loopEnabled
              ? "Loop region"
              : "Shift+drag the waveform to define a region first"
          }
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

function UserPresetSection({
  presets,
  savingPreset,
  onSave,
  onDelete,
  onApply,
}: {
  presets: UserPreset[];
  savingPreset: boolean;
  onSave: (name: string) => void;
  onDelete: (id: string) => void;
  onApply: (preset: UserPreset) => void;
}) {
  const [name, setName] = useState("");

  const handleSave = () => {
    if (!name.trim()) return;
    onSave(name);
    setName("");
  };

  return (
    <section className="user-presets">
      <div className="section-head">
        <span className="section-label">My presets</span>
      </div>
      <div className="user-preset-row">
        {presets.length === 0 && (
          <span className="user-preset-empty">
            Save the current settings as a preset to reuse later.
          </span>
        )}
        {presets.map((p) => (
          <div key={p.id} className="user-preset-chip">
            <button
              type="button"
              className="user-preset-apply"
              onClick={() => onApply(p)}
              title={`Apply "${p.name}"`}
            >
              {p.name}
              <span className="user-preset-kind"> · {p.kind}</span>
            </button>
            <button
              type="button"
              className="user-preset-delete"
              onClick={() => onDelete(p.id)}
              aria-label={`Delete preset ${p.name}`}
              title="Delete preset"
            >
              ×
            </button>
          </div>
        ))}
      </div>
      <form
        className="user-preset-save"
        onSubmit={(e) => {
          e.preventDefault();
          handleSave();
        }}
      >
        <input
          type="text"
          className="user-preset-name"
          placeholder="Save current as…"
          value={name}
          onChange={(e) => setName(e.target.value)}
          maxLength={64}
          disabled={savingPreset}
        />
        <button
          type="submit"
          className="ghost-btn"
          disabled={savingPreset || !name.trim()}
        >
          {savingPreset ? "Saving…" : "Save preset"}
        </button>
      </form>
    </section>
  );
}

function isPresetActive(a: Preset, b: Preset): boolean {
  if (a.kind === "custom" && b.kind === "custom") return a.id === b.id;
  return a.kind === b.kind;
}

function IOGainBar({
  inputGainDb,
  outputGainDb,
  onInputGain,
  onOutputGain,
}: {
  inputGainDb: number;
  outputGainDb: number;
  onInputGain: (db: number) => void;
  onOutputGain: (db: number) => void;
}) {
  return (
    <section className="io-gain">
      <Slider
        label="Input gain"
        value={inputGainDb}
        min={-24}
        max={24}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={onInputGain}
        defaultValue={0}
      />
      <Slider
        label="Output gain"
        value={outputGainDb}
        min={-24}
        max={24}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={onOutputGain}
        defaultValue={0}
      />
    </section>
  );
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
        defaultValue={0.5}
      />
      <Slider
        label="Low"
        value={settings.eq_low_db}
        min={-6}
        max={6}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={(v) => onEq("low", v)}
        defaultValue={0}
      />
      <Slider
        label="Mid"
        value={settings.eq_mid_db}
        min={-6}
        max={6}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={(v) => onEq("mid", v)}
        defaultValue={0}
      />
      <Slider
        label="High"
        value={settings.eq_high_db}
        min={-6}
        max={6}
        step={0.1}
        format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
        onChange={(v) => onEq("high", v)}
        defaultValue={0}
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
  defaultValue,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (v: number) => string;
  onChange: (v: number) => void;
  /// Optional value to snap to on double-click. Phase 12.1 Dan feedback —
  /// dbl-click should return the slider to its neutral / default position
  /// (intensity to 0.5, EQ bands to 0 dB, etc.). When omitted, double-click
  /// is a no-op so callers that don't have a natural default opt out cleanly.
  defaultValue?: number;
}) {
  const handleReset = () => {
    if (defaultValue !== undefined && defaultValue !== value) {
      onChange(defaultValue);
    }
  };
  // Phase 12.1: editable numeric input mirrors the slider. We keep a local
  // string while editing so the user can type "1." or "-" mid-edit without
  // the parent state forcing a re-format. On commit (blur / Enter), parse
  // and clamp. Escape cancels. Sync local state to value when value changes
  // externally and the input is NOT focused.
  const [draft, setDraft] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (
      draft !== null &&
      inputRef.current &&
      document.activeElement !== inputRef.current
    ) {
      setDraft(null);
    }
  }, [value, draft]);
  const commitDraft = (raw: string) => {
    const parsed = parseFloat(raw);
    if (!Number.isFinite(parsed)) {
      setDraft(null);
      return;
    }
    const clamped = Math.max(min, Math.min(max, parsed));
    if (clamped !== value) onChange(clamped);
    setDraft(null);
  };
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
        onDoubleClick={handleReset}
        className="slider-input"
        title={
          defaultValue !== undefined
            ? `Double-click to reset to ${format(defaultValue)}`
            : undefined
        }
      />
      <input
        ref={inputRef}
        type="number"
        className="slider-number"
        min={min}
        max={max}
        step={step}
        value={draft !== null ? draft : value}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={(e) => commitDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            commitDraft((e.target as HTMLInputElement).value);
            (e.target as HTMLInputElement).blur();
          } else if (e.key === "Escape") {
            setDraft(null);
            (e.target as HTMLInputElement).blur();
          }
        }}
        onDoubleClick={handleReset}
        title={
          defaultValue !== undefined
            ? `Type a value or double-click to reset to ${format(defaultValue)}`
            : "Type a value to set precisely"
        }
      />
    </div>
  );
}

function UndoRedoBar({
  canUndo,
  canRedo,
  onUndo,
  onRedo,
}: {
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
}) {
  return (
    <section className="undo-redo-bar">
      <button
        type="button"
        className="ghost-btn"
        onClick={onUndo}
        disabled={!canUndo}
        title="Undo last edit (Ctrl+Z)"
      >
        ↶ Undo
      </button>
      <button
        type="button"
        className="ghost-btn"
        onClick={onRedo}
        disabled={!canRedo}
        title="Redo (Ctrl+Shift+Z or Ctrl+Y)"
      >
        ↷ Redo
      </button>
    </section>
  );
}

function StaleBar({
  stale,
  isRendering,
  onUpdate,
  liveUpdateStats,
  renderProgress,
  peakDbfs,
  isPlaying,
}: {
  stale: boolean;
  isRendering: boolean;
  onUpdate: () => void;
  liveUpdateStats: { attempts: number; applied: number; lastAt: number | null };
  renderProgress: { fraction: number; kind: "preview" | "master" | "album" } | null;
  peakDbfs: number;
  isPlaying: boolean;
}) {
  const progressPct =
    renderProgress !== null
      ? Math.round(Math.max(0, Math.min(1, renderProgress.fraction)) * 100)
      : null;
  return (
    <section className="stale-bar">
      <span className="stale-dot live" aria-hidden />
      <span className="stale-text">
        {progressPct !== null
          ? `Rendering ${renderProgress!.kind} WAV… ${progressPct}%`
          : isRendering
          ? "Rendering preview WAV…"
          : "Mastered playback is live — drag controls and hear the change immediately."}
      </span>
      {progressPct !== null && (
        <div
          className="render-progress"
          role="progressbar"
          aria-valuenow={progressPct}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div
            className="render-progress-fill"
            style={{ width: `${progressPct}%` }}
          />
        </div>
      )}
      <ClippingIndicator peakDbfs={peakDbfs} isPlaying={isPlaying} />
      {/* Phase 12.1 live-update counter — increments every time the frontend
          sends api.updateChain to the backend. If you make adjustments and
          this counter doesn't change, the frontend isn't firing live updates
          (look at this number to verify without DevTools). */}
      <span
        className="live-update-badge"
        title={`Live coeff updates sent / resolved since session start${
          liveUpdateStats.lastAt
            ? `. Last fired ${Math.round((Date.now() - liveUpdateStats.lastAt) / 1000)} s ago.`
            : ". None fired yet."
        }`}
      >
        live: {liveUpdateStats.applied}/{liveUpdateStats.attempts}
      </span>
      <button
        type="button"
        className="ghost-btn"
        onClick={onUpdate}
        disabled={isRendering}
        title="Render a temporary WAV with the current settings so you can audit it in another player or DAW. Not required for live audition — the Mastered button above plays through the chain in real time."
      >
        {stale ? "Render audit WAV" : "Re-render audit WAV"}
      </button>
    </section>
  );
}

// Phase 12.2 — live clipping / output peak indicator. Reads the dBFS peak
// streamed via PlaybackTick (audio thread → atomic → snapshot → tick) and
// renders one of three states: silent (no signal), OK (peak below threshold),
// or CLIP (peak above -0.1 dBFS, the streaming-headroom warning floor).
// Idle (not playing) collapses to a neutral "—" so the meter doesn't read as
// "OK" when there's nothing actually being measured.
const CLIP_THRESHOLD_DBFS = -0.1;
const SILENCE_FLOOR_DBFS = -80;

function ClippingIndicator({
  peakDbfs,
  isPlaying,
}: {
  peakDbfs: number;
  isPlaying: boolean;
}) {
  let state: "idle" | "silent" | "ok" | "clip";
  if (!isPlaying) {
    state = "idle";
  } else if (peakDbfs >= CLIP_THRESHOLD_DBFS) {
    state = "clip";
  } else if (peakDbfs < SILENCE_FLOOR_DBFS) {
    state = "silent";
  } else {
    state = "ok";
  }
  const label = ((): string => {
    if (state === "idle") return "PEAK —";
    if (state === "silent") return "PEAK —";
    if (state === "clip") return "CLIP";
    return `PEAK ${peakDbfs.toFixed(1)} dB`;
  })();
  const title =
    state === "clip"
      ? `Output peak ${peakDbfs.toFixed(2)} dBFS — clipping risk. Lower Output Gain, Intensity, or pull Input Gain down to back off the chain.`
      : state === "ok"
      ? `Output peak ${peakDbfs.toFixed(2)} dBFS (safe headroom).`
      : state === "silent"
      ? "Below -80 dBFS — effectively silent in the last window."
      : "No mastered playback in progress; meter is idle.";
  return (
    <span
      className={`clip-indicator clip-${state}`}
      role="status"
      aria-live="polite"
      title={title}
    >
      {label}
    </span>
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
          label="Warmth (coming soon)"
          value={a.warmth}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("warmth", v)}
        />
        <NumberField
          label="Presence/Air (coming soon)"
          value={a.presence_air}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("presence_air", v)}
        />
        <NumberField
          label="Compression (coming soon)"
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
  // Same draft-while-editing pattern as Slider so the user can type "1." or
  // "-" mid-value without the parent re-formatting on every keystroke.
  const [draft, setDraft] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);
  useEffect(() => {
    if (
      draft !== null &&
      inputRef.current &&
      document.activeElement !== inputRef.current
    ) {
      setDraft(null);
    }
  }, [value, draft]);
  const commitDraft = (raw: string) => {
    const parsed = parseFloat(raw);
    if (!Number.isFinite(parsed)) {
      setDraft(null);
      return;
    }
    const clamped = Math.max(min, Math.min(max, parsed));
    if (clamped !== value) onChange(clamped);
    setDraft(null);
  };
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
        {value === null ? (
          <span className="adv-value">Auto</span>
        ) : (
          <input
            ref={inputRef}
            type="number"
            className="adv-number"
            min={min}
            max={max}
            step={step}
            value={draft !== null ? draft : value}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={(e) => commitDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                commitDraft((e.target as HTMLInputElement).value);
                (e.target as HTMLInputElement).blur();
              } else if (e.key === "Escape") {
                setDraft(null);
                (e.target as HTMLInputElement).blur();
              }
            }}
            title={`Type a value or click Auto to reset. Format: ${format(value)}`}
          />
        )}
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

function AlbumHeader({
  tracks,
  isExporting,
  onExport,
}: {
  tracks: ImportedTrack[];
  isExporting: boolean;
  onExport: () => void;
}) {
  const totalSeconds = tracks.reduce(
    (acc, t) => acc + (t.duration_seconds ?? 0),
    0,
  );
  return (
    <section className="album-header">
      <div className="album-summary">
        <span className="section-label">Album</span>
        <div className="album-stat">
          <strong>{tracks.length}</strong> tracks
          {totalSeconds > 0 && (
            <>
              <span className="dim"> · </span>
              <strong>{formatTime(totalSeconds)}</strong>
            </>
          )}
        </div>
      </div>
      <button
        type="button"
        className="primary"
        onClick={onExport}
        disabled={isExporting}
      >
        {isExporting ? "Rendering album…" : "Export Album"}
      </button>
    </section>
  );
}

function ExportReceiptCard({
  receipt,
  onClose,
}: {
  receipt: ExportReceipt;
  onClose: () => void;
}) {
  const reveal = async (path: string) => {
    if (!path) return;
    try {
      await api.openOutput(path);
    } catch (err) {
      console.error("openOutput failed", err);
    }
  };
  const isAlbum = receipt.kind === "album";
  const paths = receipt.job.output_paths;
  return (
    <div className="receipt-backdrop" onClick={onClose}>
      <div className="receipt" onClick={(e) => e.stopPropagation()}>
        <header>
          <h2>{isAlbum ? "Album export complete" : "Export complete"}</h2>
          <button type="button" className="toast-close" onClick={onClose} aria-label="Close">
            ×
          </button>
        </header>
        <div className="receipt-paths">
          {paths.map((path, i) => (
            <button
              key={path + i}
              type="button"
              className={"receipt-path" + (isAlbum && i === 0 ? " primary-path" : "")}
              onClick={() => reveal(path)}
              title="Reveal in file manager"
            >
              {isAlbum && i === 0 ? "▸ Continuous album · " : ""}
              {path}
            </button>
          ))}
        </div>
        {receipt.checks.length > 0 && (
          <div className="receipt-checks">
            {receipt.checks.map((c, i) => (
              <CheckRow key={i} check={c} />
            ))}
          </div>
        )}
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
