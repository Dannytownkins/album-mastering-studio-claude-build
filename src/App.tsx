import {
  useEffect,
  useRef,
  useState,
  type DragEvent as ReactDragEvent,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { api } from "./lib/api";
import { useTrackMaster } from "./hooks/useTrackMaster";
import { PresetIcon } from "./components/PresetIcon";
import { RightRail } from "./components/RightRail";
import { AlbumPanel } from "./components/AlbumPanel";
import { Knob, intensityLabel } from "./components/Knob";
import { SignalChain } from "./components/SignalChain";
import type {
  AnalysisResult,
  DeliveryProfile,
  ImportedTrack,
  LoopRegion,
  MasteringSettings,
  Preset,
  UserPreset,
  WaveformPeaks,
  QualityCheck,
  QualityLevel,
} from "./bindings";
import {
  DELIVERY_PROFILE_DISPLAY,
  DELIVERY_PROFILE_TARGET_LUFS,
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
    <div className="app-root">
      <TopHeader
        mode={tm.mode}
        onModeChange={tm.setMode}
        onSaveProject={tm.saveProjectAs}
        onOpenProject={tm.openProjectFromDisk}
      />
    <div className="app">
      <Sidebar
        tracks={tm.tracks}
        selectedId={tm.selectedTrackId}
        onSelect={tm.selectTrack}
        onRemove={tm.removeTrack}
        onAdd={tm.openImportDialog}
        isAnalyzing={tm.isAnalyzing}
        mode={tm.mode}
        onReorder={tm.reorderTracks}
        overrideAlbum={tm.overrideAlbum}
      />
      <main className="workspace">
        {tm.mode === "album" && tm.tracks.length > 0 && (
          <>
            <AlbumHeader
              tracks={tm.tracks}
              isExporting={tm.isExportingAlbum}
              onExport={tm.exportAlbum}
            />
            <AlbumPanel
              tracks={tm.tracks}
              selectedTrackId={tm.selectedTrack?.id ?? null}
              onSelectTrack={tm.selectTrack}
              albumArcKind={tm.albumArcKind}
              albumIntensity={tm.albumIntensity}
              albumTitle={tm.albumTitle}
              albumRendering={tm.albumRendering}
              albumExportReport={tm.albumExportReport}
              onAlbumArc={tm.setAlbumArc}
              onAlbumIntensity={tm.setAlbumIntensity}
              onAlbumTitle={tm.setAlbumTitle}
              onExportAlbum={tm.exportAlbumPlan}
            />
          </>
        )}
        {tm.selectedTrack ? (
          <TrackMaster tm={tm} />
        ) : (
          <EmptyState onAdd={tm.openImportDialog} />
        )}
      </main>
      <RightRail
        analysis={tm.selectedAnalysis}
        isAnalyzing={tm.isAnalyzing}
        lastChecks={tm.lastExportReceipt?.checks}
        peakDbfs={tm.transport.peakDbfs}
        isPlaying={tm.transport.isPlaying}
        compressionGr={tm.transport.compressionGr}
        lufsMomentary={tm.transport.lufsMomentary}
        lufsIntegrated={tm.transport.lufsIntegrated}
        effectiveWidth={
          tm.selectedSettings.advanced.width ??
          (tm.selectedSettings.preset.kind === "spatial" ? 1.3 : 1.0)
        }
        advancedSlot={
          tm.selectedTrack ? (
            <AdvancedPanel
              settings={tm.selectedSettings}
              onAdvanced={tm.setAdvanced}
              onInputGain={tm.setInputGain}
              onOutputGain={tm.setOutputGain}
              onDeliveryProfile={tm.setDeliveryProfile}
            />
          ) : undefined
        }
        canExport={!!tm.selectedAnalysis}
        isExporting={tm.isExporting}
        isRendering={tm.isRendering}
        onExport={tm.exportMaster}
      />
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
    <BottomStatusBar tm={tm} />
    </div>
  );
}

function BottomStatusBar({ tm }: { tm: ReturnType<typeof useTrackMaster> }) {
  const analysis = tm.selectedAnalysis;
  const peak = tm.transport.peakDbfs;
  const isPlaying = tm.transport.isPlaying;

  const peakDisplay = isPlaying && peak > -80 ? `${peak.toFixed(1)} dBFS` : "—";
  const lufsDisplay = analysis ? `${analysis.lufs_integrated.toFixed(1)} LUFS` : "—";

  let processing: { tone: "idle" | "busy" | "ok"; text: string };
  if (tm.isExporting) {
    processing = { tone: "busy", text: "Exporting…" };
  } else if (tm.isRendering) {
    processing = { tone: "busy", text: "Rendering audit…" };
  } else if (tm.isAnalyzing) {
    processing = { tone: "busy", text: "Analyzing…" };
  } else if (tm.isLoadingWaveform) {
    processing = { tone: "busy", text: "Decoding…" };
  } else if (tm.selectedTrack) {
    processing = { tone: "ok", text: "Ready" };
  } else {
    processing = { tone: "idle", text: "Idle" };
  }

  return (
    <footer className="bottom-status">
      <div className="bottom-status-left">
        <StatusDot
          tone={tm.selectedTrack ? (analysis ? "ok" : "warn") : "idle"}
          label={
            !tm.selectedTrack
              ? "No track"
              : analysis
              ? "Analyzed"
              : "Awaiting analysis"
          }
        />
        <StatusDot
          tone={
            tm.lastExportReceipt
              ? tm.lastExportReceipt.checks.some((c) => c.level === "critical")
                ? "bad"
                : tm.lastExportReceipt.checks.some((c) => c.level === "warning")
                ? "warn"
                : "ok"
              : "idle"
          }
          label={
            tm.lastExportReceipt
              ? tm.lastExportReceipt.checks.some((c) => c.level === "critical")
                ? "Quality checks failed"
                : tm.lastExportReceipt.checks.some((c) => c.level === "warning")
                ? "Quality checks (review)"
                : "Quality checks passed"
              : "Quality checks not run"
          }
        />
      </div>
      <div className="bottom-status-center">
        <span className="status-readout">
          <span className="status-readout-label">Peak</span>
          <span className="status-readout-value">{peakDisplay}</span>
        </span>
        <span className="status-readout">
          <span className="status-readout-label">Loudness</span>
          <span className="status-readout-value">{lufsDisplay}</span>
        </span>
      </div>
      <div className="bottom-status-right">
        <span className="status-processing-label">Processing</span>
        <span className={`status-pill status-${processing.tone === "busy" ? "warn" : processing.tone === "ok" ? "ok" : "idle"}`}>
          {processing.text}
        </span>
      </div>
    </footer>
  );
}

function StatusDot({
  tone,
  label,
}: {
  tone: "idle" | "ok" | "warn" | "bad";
  label: string;
}) {
  return (
    <span className={`status-dot-row status-dot-${tone}`} title={label}>
      <span className="status-dot-glyph" aria-hidden />
      <span className="status-dot-label">{label}</span>
    </span>
  );
}

function TopHeader({
  mode,
  onModeChange,
  onSaveProject,
  onOpenProject,
}: {
  mode: "track" | "album";
  onModeChange: (mode: "track" | "album") => void;
  onSaveProject: () => void;
  onOpenProject: () => void;
}) {
  return (
    <header className="top-header">
      <div className="top-header-left">
        <span className="brand-mark" aria-hidden>
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none">
            <path
              d="M4 6h2v12H4zM8 10h2v8H8zM12 4h2v16h-2zM16 8h2v10h-2zM20 12h2v6h-2z"
              fill="currentColor"
            />
          </svg>
        </span>
        <span className="brand-name">Album Mastering Studio</span>
      </div>
      <nav className="top-header-tabs" aria-label="Mode">
        <button
          type="button"
          className={"top-tab " + (mode === "track" ? "is-active" : "")}
          onClick={() => onModeChange("track")}
        >
          Track Master
        </button>
        <button
          type="button"
          className={"top-tab " + (mode === "album" ? "is-active" : "")}
          onClick={() => onModeChange("album")}
        >
          Album Master
        </button>
      </nav>
      <div className="top-header-right">
        <button
          type="button"
          className="icon-tile"
          aria-label="Open project (.ams.json)"
          title="Open project (.ams.json)"
          onClick={onOpenProject}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
        </button>
        <button
          type="button"
          className="icon-tile"
          aria-label="Save project (.ams.json)"
          title="Save project as (.ams.json)"
          onClick={onSaveProject}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
            <polyline points="17 21 17 13 7 13 7 21" />
            <polyline points="7 3 7 8 15 8" />
          </svg>
        </button>
        <button
          type="button"
          className="icon-tile"
          aria-label="Settings (not yet wired)"
          title="Settings (coming soon)"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </button>
        <button
          type="button"
          className="icon-tile"
          aria-label="Help (not yet wired)"
          title="Help (coming soon)"
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="10" />
            <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3" />
            <path d="M12 17h.01" />
          </svg>
        </button>
      </div>
    </header>
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

  // Sum of every track's duration (seconds) — surfaces the album/queue total
  // alongside the count, the way the reference shows "9 tracks · 42:18".
  const totalSeconds = tracks.reduce(
    (acc, t) => acc + (t.duration_seconds ?? 0),
    0,
  );
  const totalLabel = totalSeconds > 0 ? `${tracks.length} tracks · ${formatDuration(totalSeconds)}` : `${tracks.length} tracks`;
  return (
    <aside className="sidebar">
      <div className="sidebar-section sidebar-head-strip">
        <div className="sidebar-head-titles">
          <span className="section-label">
            {mode === "album" ? "Album order" : "Tracks"}
          </span>
          <span className="sidebar-count">{totalLabel}</span>
        </div>
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
              <span className="track-index" aria-hidden>
                {(index + 1).toString().padStart(2, "0")}
              </span>
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
                <span className="track-meta">
                  {t.duration_seconds ? formatDuration(t.duration_seconds) : `.${t.source_format}`}
                </span>
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

      <div className="sidebar-footer">
        {isAnalyzing && <div className="sidebar-status">Analyzing…</div>}
        <button
          type="button"
          className="primary sidebar-import-btn"
          onClick={onAdd}
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2.2} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
          Import Audio
        </button>
      </div>
    </aside>
  );
}

function EmptyState({ onAdd }: { onAdd: () => void }) {
  return (
    <div className="empty-state">
      <div className="empty-state-glyph" aria-hidden>
        <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
          <defs>
            <linearGradient id="emptyglow" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stopColor="#6fa3ff" />
              <stop offset="1" stopColor="#2a6bf2" />
            </linearGradient>
          </defs>
          <circle cx="32" cy="32" r="28" stroke="url(#emptyglow)" strokeWidth="2" opacity="0.5" />
          <path
            d="M14 32h4l2-12 4 24 4-18 4 14 4-10 4 8 4-6 4 4h4"
            stroke="url(#emptyglow)"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            fill="none"
          />
        </svg>
      </div>
      <h1>Drop audio, analyze, export.</h1>
      <p>
        Album Mastering Studio masters one track or a full album. Universal-first
        — no genre wizard, no jargon walls.
      </p>
      <button type="button" className="primary" onClick={onAdd}>
        Import audio
      </button>
      <p className="empty-foot">
        Supports WAV · AIFF · FLAC · MP3 · M4A · AAC · OGG · Opus.
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
      <PresetTiles
        selected={tm.selectedSettings.preset}
        onChange={tm.setPreset}
      />
      <SignalChain settings={tm.selectedSettings} />
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
        onAdvanced={tm.setAdvanced}
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
        isExporting={tm.isExporting}
        onUpdate={tm.updatePreview}
        liveUpdateStats={tm.liveUpdateStats}
        renderProgress={tm.renderProgress}
        peakDbfs={tm.transport.peakDbfs}
        isPlaying={tm.transport.isPlaying}
        compressionGr={tm.transport.compressionGr}
      />
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
  const chips: { key: string; label: string }[] = [];
  if (track.source_format) {
    chips.push({ key: "fmt", label: track.source_format.toUpperCase() });
  }
  if (track.sample_rate) {
    const sr = track.sample_rate;
    const label = sr >= 1000 ? `${(sr / 1000).toFixed(sr % 1000 === 0 ? 0 : 1)} kHz` : `${sr} Hz`;
    chips.push({ key: "sr", label });
  }
  if (track.channels) {
    chips.push({
      key: "ch",
      label: track.channels === 1 ? "Mono" : track.channels === 2 ? "Stereo" : `${track.channels}ch`,
    });
  }
  if (track.duration_seconds) {
    chips.push({ key: "dur", label: formatDuration(track.duration_seconds) });
  }
  return (
    <section className="track-header">
      <div className="track-header-main">
        <h1 className="track-title">{track.display_name}</h1>
        <div className="track-meta-chips">
          {chips.map((c) => (
            <span key={c.key} className="meta-chip">{c.label}</span>
          ))}
        </div>
        {analysis && <AnalysisSummary analysis={analysis} />}
        {showStoryTags && analysis && (
          <StoryTags analysis={analysis} />
        )}
      </div>
      <div className={`track-badge status-pill ${isAnalyzing ? "status-warn" : analysis ? "status-ok" : ""}`}>
        {isAnalyzing ? "Analyzing…" : analysis ? "Analyzed" : "Pending"}
      </div>
    </section>
  );
}

function formatDuration(seconds: number): string {
  const total = Math.max(0, Math.round(seconds));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
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

  // Most actionable headline = first line (loudness commentary). Subsequent
  // lines stay collapsed by default so the card reads as a one-line "insight"
  // until the user clicks for the full breakdown.
  const [headline, ...rest] = lines;
  return (
    <details className="analysis-summary">
      <summary>
        <span className="analysis-summary-icon" aria-hidden>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round">
            <path d="M9 18h6" />
            <path d="M10 22h4" />
            <path d="M12 2a7 7 0 0 0-5 11.9c1 1 1.5 2 1.5 3.1h7c0-1.1.5-2.1 1.5-3.1A7 7 0 0 0 12 2z" />
          </svg>
        </span>
        <span className="analysis-summary-text">
          <span className="analysis-summary-eyebrow">Insight</span>
          <span className="analysis-summary-headline">{headline}</span>
        </span>
        <span className="analysis-summary-chevron" aria-hidden>⌄</span>
      </summary>
      {rest.length > 0 && (
        <ul>
          {rest.map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
      )}
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
      <div className="wf-main">
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
      <WaveformDbScale />
      </div>
      <WaveformOverview
        channel={channel}
        currentTimeSec={currentTimeSec}
        durationSec={durationSec}
        region={displayRegion}
        onSeek={onSeek}
      />
      <p className="wf-hint">
        Click to seek. Shift+drag to define a loop region. Shift+click clears it.
      </p>
    </section>
  );
}

function WaveformDbScale() {
  // Vertical dB scale at the right edge of the main waveform. The waveform
  // canvas is centered around 0 dB (mid-line), so we render ticks at -6,
  // -12, -18, -24 above AND below the centerline. Pure presentation — does
  // not change layout of the waveform itself (uses absolute positioning).
  const ticks = [0, -6, -12, -18, -24];
  return (
    <div className="wf-db-scale" aria-hidden>
      {ticks.map((db, i) => (
        <span
          key={`top-${db}`}
          className={`wf-db-tick${i === 0 ? " wf-db-tick-center" : ""}`}
        >
          {db === 0 ? "0" : db}
        </span>
      ))}
      {ticks.slice(1).map((db) => (
        <span key={`bot-${db}`} className="wf-db-tick">
          {db}
        </span>
      ))}
    </div>
  );
}

function WaveformOverview({
  channel,
  currentTimeSec,
  durationSec,
  region,
  onSeek,
}: {
  channel: number[];
  currentTimeSec: number;
  durationSec: number;
  region: LoopRegion | null;
  onSeek: (positionSec: number) => void;
}) {
  // Compact 48 px-high overview rendered below the main waveform. Click-to-
  // seek only — no shift-drag region edit here, the main waveform handles
  // that. Adds a "viewport" rectangle showing what's currently in the
  // main waveform's visible window; for v1 the main waveform shows the
  // whole track, so the viewport equals the visible region (or the loop
  // region if set).
  const W = 1000;
  const H = 48;
  const playheadX =
    durationSec > 0
      ? Math.max(0, Math.min(W, (currentTimeSec / durationSec) * W))
      : 0;
  const regionRect = region && durationSec > 0
    ? (() => {
        const startX = Math.max(
          0,
          Math.min(W, (Math.min(region.start_sec, region.end_sec) / durationSec) * W),
        );
        const endX = Math.max(
          0,
          Math.min(W, (Math.max(region.start_sec, region.end_sec) / durationSec) * W),
        );
        return { startX, endX };
      })()
    : null;
  const handlePointer = (e: ReactPointerEvent<SVGSVGElement>) => {
    if (durationSec <= 0) return;
    const rect = e.currentTarget.getBoundingClientRect();
    if (rect.width <= 0) return;
    const ratio = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    onSeek(ratio * durationSec);
  };
  return (
    <svg
      className="wf-overview"
      viewBox={`0 0 ${W} ${H}`}
      preserveAspectRatio="none"
      onPointerDown={handlePointer}
      role="slider"
      aria-label="Waveform overview — click to seek"
      aria-valuemin={0}
      aria-valuemax={durationSec}
      aria-valuenow={currentTimeSec}
    >
      {channel.map((v, i) => {
        const x = (i / channel.length) * W;
        const barW = (W / channel.length) * 0.85;
        const barH = v * (H * 0.92);
        const y = (H - barH) / 2;
        return <rect key={i} x={x} y={y} width={barW} height={barH} rx={0.5} />;
      })}
      {regionRect && (
        <rect
          className="wf-overview-region"
          x={regionRect.startX}
          y={0}
          width={Math.max(1, regionRect.endX - regionRect.startX)}
          height={H}
        />
      )}
      <line
        className="wf-overview-playhead"
        x1={playheadX}
        y1={0}
        x2={playheadX}
        y2={H}
      />
    </svg>
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

// Per-preset accent color. Drives the tile's character glow so the imagery
// feels integrated with the tile rather than pasted on. Matches the color
// language of the generated 3D imagery.
const PRESET_ACCENT: Record<Preset["kind"], string> = {
  universal: "#4d8bff",
  clarity: "#22d3ee",
  tape: "#fbbf24",
  spatial: "#a78bfa",
  oomph: "#f87171",
  warmth: "#fb923c",
  punch: "#ef4444",
  loud: "#60a5fa",
  custom: "#9ca3af",
};

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
          const accent = PRESET_ACCENT[p.value.kind];
          return (
            <button
              key={p.label}
              type="button"
              className={"tile " + (active ? "active" : "")}
              style={{ ["--tile-accent" as never]: accent }}
              onClick={() => onChange(p.value)}
            >
              <PresetIcon kind={p.value.kind} className="tile-icon" />
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
  const [isExpanded, setIsExpanded] = useState(false);

  const handleSave = () => {
    if (!name.trim()) return;
    onSave(name);
    setName("");
    setIsExpanded(false);
  };

  // Empty state collapses to a single inline "+ Save current as preset"
  // button so we don't burn a full row of vertical space on nothing.
  if (presets.length === 0 && !isExpanded) {
    return (
      <button
        type="button"
        className="user-presets-add-inline"
        onClick={() => setIsExpanded(true)}
        title="Save the current settings as a named preset"
      >
        + Save current as preset
      </button>
    );
  }

  return (
    <section className="user-presets">
      <div className="user-preset-row">
        <span className="section-label user-preset-row-label">MY PRESETS</span>
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
            autoFocus={isExpanded}
          />
          <button
            type="submit"
            className="ghost-btn"
            disabled={savingPreset || !name.trim()}
          >
            {savingPreset ? "Saving…" : "Save"}
          </button>
          {presets.length === 0 && (
            <button
              type="button"
              className="ghost-btn"
              onClick={() => {
                setName("");
                setIsExpanded(false);
              }}
            >
              Cancel
            </button>
          )}
        </form>
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
  onAdvanced,
}: {
  settings: MasteringSettings;
  onIntensity: (v: number) => void;
  onEq: (band: "low" | "mid" | "high", db: number) => void;
  onAdvanced: (adv: MasteringSettings["advanced"]) => void;
}) {
  return (
    <section className="macros knobs-row">
      <div className="intensity-block">
        <span className="section-label">INTENSITY</span>
        <Knob
          label=""
          size="lg"
          value={settings.intensity}
          min={0}
          max={1}
          step={0.01}
          defaultValue={0.5}
          format={(v) => `${Math.round(v * 100)}%`}
          caption={intensityLabel(settings.intensity)}
          onChange={onIntensity}
          centerValue
        />
      </div>
      <div className="tone-shape-block">
        <span className="section-label">TONE SHAPE</span>
        <div className="tone-shape-knobs">
          <Knob
            label="Low"
            size="md"
            tone="cyan"
            value={settings.eq_low_db}
            min={-12}
            max={12}
            step={0.1}
            defaultValue={0}
            format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
            onChange={(v) => onEq("low", v)}
          />
          <Knob
            label="Mid"
            size="md"
            tone="green"
            value={settings.eq_mid_db}
            min={-12}
            max={12}
            step={0.1}
            defaultValue={0}
            format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
            onChange={(v) => onEq("mid", v)}
          />
          <Knob
            label="High"
            size="md"
            tone="purple"
            value={settings.eq_high_db}
            min={-12}
            max={12}
            step={0.1}
            defaultValue={0}
            format={(v) => `${v > 0 ? "+" : ""}${v.toFixed(1)} dB`}
            onChange={(v) => onEq("high", v)}
          />
        </div>
      </div>
      <LoudnessTarget settings={settings} onAdvanced={onAdvanced} />
    </section>
  );
}

// Delivery profiles — short names + their canonical LUFS targets. Matched on
// the current `lufs_offset_db` so the dropdown reflects what the chain is
// actually doing. Anything outside the known set reads as "Custom".
const LOUDNESS_PROFILES: { id: string; label: string; lufs: number | null }[] = [
  { id: "streaming", label: "Streaming (-14)", lufs: -14 },
  { id: "loud-streaming", label: "Loud streaming (-11)", lufs: -11 },
  { id: "cd-master", label: "CD master (-9)", lufs: -9 },
  { id: "off", label: "Off / Natural", lufs: null },
];

function profileIdFor(lufs: number | null): string {
  if (lufs === null) return "off";
  for (const p of LOUDNESS_PROFILES) {
    if (p.lufs !== null && Math.abs(p.lufs - lufs) < 1e-3) return p.id;
  }
  return "custom";
}

function LoudnessTarget({
  settings,
  onAdvanced,
}: {
  settings: MasteringSettings;
  onAdvanced: (adv: MasteringSettings["advanced"]) => void;
}) {
  const current = settings.advanced.lufs_offset_db;
  const profileId = profileIdFor(current ?? null);

  const handleProfileChange = (id: string) => {
    if (id === "custom") return; // Custom stays at current value (Advanced edits).
    const profile = LOUDNESS_PROFILES.find((p) => p.id === id);
    if (!profile) return;
    onAdvanced({ ...settings.advanced, lufs_offset_db: profile.lufs });
  };

  const display = current !== null && current !== undefined
    ? `${current.toFixed(1)}`
    : "—";

  return (
    <div className="loudness-target-block">
      <span className="section-label">LOUDNESS TARGET</span>
      <div className="loudness-readout">
        <span className="loudness-number">{display}</span>
        <span className="loudness-unit">LUFS</span>
      </div>
      <select
        className="loudness-profile-select"
        value={profileId}
        onChange={(e) => handleProfileChange(e.target.value)}
      >
        {LOUDNESS_PROFILES.map((p) => (
          <option key={p.id} value={p.id}>{p.label}</option>
        ))}
        {profileId === "custom" && (
          <option value="custom">Custom ({display} LUFS)</option>
        )}
      </select>
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
  isExporting,
  onUpdate,
  liveUpdateStats,
  renderProgress,
  peakDbfs,
  isPlaying,
  compressionGr,
}: {
  stale: boolean;
  isRendering: boolean;
  isExporting: boolean;
  onUpdate: () => void;
  liveUpdateStats: { attempts: number; applied: number; lastAt: number | null };
  renderProgress: { fraction: number; kind: "preview" | "master" | "album" } | null;
  peakDbfs: number;
  isPlaying: boolean;
  compressionGr: { low: number; mid: number; high: number };
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
      <GrIndicator label="L" db={compressionGr.low} isPlaying={isPlaying} />
      <GrIndicator label="M" db={compressionGr.mid} isPlaying={isPlaying} />
      <GrIndicator label="H" db={compressionGr.high} isPlaying={isPlaying} />
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
        disabled={isRendering || isExporting}
        title={
          isExporting
            ? "Disabled while an export is in progress — the two operations share render state."
            : "Render a temporary WAV with the current settings so you can audit it in another player or DAW. Not required for live audition — the Mastered button above plays through the chain in real time."
        }
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

// Phase 12.2 — per-band gain-reduction readout chip. Mirrors ClippingIndicator's
// shape: idle (not playing) → "—"; silent sentinel (-120 dB) → "—"; otherwise
// shows the reduction in dB. Color bands: idle/silent muted; >= -3 dB green;
// -3..-6 dB amber; < -6 dB red.
function GrIndicator({
  label,
  db,
  isPlaying,
}: {
  label: string;
  db: number;
  isPlaying: boolean;
}) {
  let state: "idle" | "ok" | "warn" | "hot";
  let text: string;
  if (!isPlaying || db <= -119.9) {
    state = "idle";
    text = `${label} —`;
  } else if (db >= -3.0) {
    state = "ok";
    text = `${label} ${db.toFixed(1)}`;
  } else if (db >= -6.0) {
    state = "warn";
    text = `${label} ${db.toFixed(1)}`;
  } else {
    state = "hot";
    text = `${label} ${db.toFixed(1)}`;
  }
  return (
    <span
      className={`gr-indicator gr-${state}`}
      title={`Compressor gain reduction (${label}): ${db.toFixed(2)} dB`}
    >
      {text}
    </span>
  );
}


function AdvancedPanel({
  settings,
  onAdvanced,
  onInputGain,
  onOutputGain,
  onDeliveryProfile,
}: {
  settings: MasteringSettings;
  onAdvanced: (adv: MasteringSettings["advanced"]) => void;
  onInputGain: (db: number) => void;
  onOutputGain: (db: number) => void;
  onDeliveryProfile: (profile: DeliveryProfile) => void;
}) {
  const a = settings.advanced;
  const update = (
    field: keyof MasteringSettings["advanced"],
    value: number | boolean | null,
  ) => {
    onAdvanced({ ...a, [field]: value });
  };
  const profile = settings.delivery_profile;
  const profileTargetLufs = DELIVERY_PROFILE_TARGET_LUFS[profile];
  return (
    <section className="advanced">
      <div className="section-head">
        <span className="section-label">Advanced</span>
      </div>
      <div className="advanced-delivery-row">
        <label className="adv-label" htmlFor="delivery-profile-select">
          Delivery profile
        </label>
        <select
          id="delivery-profile-select"
          className="loudness-profile-select"
          value={profile}
          onChange={(e) =>
            onDeliveryProfile(e.target.value as DeliveryProfile)
          }
        >
          {(Object.keys(DELIVERY_PROFILE_DISPLAY) as DeliveryProfile[]).map(
            (p) => (
              <option key={p} value={p}>
                {DELIVERY_PROFILE_DISPLAY[p]}
                {DELIVERY_PROFILE_TARGET_LUFS[p] !== null
                  ? ` · ${DELIVERY_PROFILE_TARGET_LUFS[p]} LUFS`
                  : ""}
              </option>
            ),
          )}
        </select>
        {profile !== "custom" && profileTargetLufs !== null && (
          <span className="adv-profile-hint">
            Shadows LUFS / ceiling / bit-depth at render. Pick Custom to use the
            explicit values below.
          </span>
        )}
      </div>
      <div className="advanced-grid">
        <GainField
          label="Input gain"
          value={settings.input_gain_db}
          onChange={onInputGain}
        />
        <GainField
          label="Output gain"
          value={settings.output_gain_db}
          onChange={onOutputGain}
        />
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
          label="Compression density"
          value={a.compression_density}
          step={0.05}
          min={0}
          max={1}
          format={(v) => v.toFixed(2)}
          onChange={(v) => update("compression_density", v)}
        />
        <CompressionPerBandSubsection a={a} onUpdate={update} />
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

function CompressionPerBandSubsection({
  a,
  onUpdate,
}: {
  a: MasteringSettings["advanced"];
  onUpdate: (
    field: keyof MasteringSettings["advanced"],
    value: number | boolean | null,
  ) => void;
}) {
  const [open, setOpen] = useState(false);
  return (
    <details
      className="compression-per-band"
      open={open}
      onToggle={(e) => setOpen((e.target as HTMLDetailsElement).open)}
    >
      <summary className="adv-label">Per-band compressor</summary>
      <div className="compression-link">
        <label>
          <input
            type="checkbox"
            checked={a.compression_link_stereo !== false}
            onChange={(e) =>
              onUpdate("compression_link_stereo", e.target.checked ? null : false)
            }
          />
          {" "}Link stereo (default on — drives both channels from a shared envelope)
        </label>
      </div>
      <div className="compression-per-band-grid">
        <CompressionBandColumn
          label="Low"
          threshold={a.compression_low_threshold_db}
          ratio={a.compression_low_ratio}
          attack={a.compression_low_attack_ms}
          release={a.compression_low_release_ms}
          onThreshold={(v) => onUpdate("compression_low_threshold_db", v)}
          onRatio={(v) => onUpdate("compression_low_ratio", v)}
          onAttack={(v) => onUpdate("compression_low_attack_ms", v)}
          onRelease={(v) => onUpdate("compression_low_release_ms", v)}
        />
        <CompressionBandColumn
          label="Mid"
          threshold={a.compression_mid_threshold_db}
          ratio={a.compression_mid_ratio}
          attack={a.compression_mid_attack_ms}
          release={a.compression_mid_release_ms}
          onThreshold={(v) => onUpdate("compression_mid_threshold_db", v)}
          onRatio={(v) => onUpdate("compression_mid_ratio", v)}
          onAttack={(v) => onUpdate("compression_mid_attack_ms", v)}
          onRelease={(v) => onUpdate("compression_mid_release_ms", v)}
        />
        <CompressionBandColumn
          label="High"
          threshold={a.compression_high_threshold_db}
          ratio={a.compression_high_ratio}
          attack={a.compression_high_attack_ms}
          release={a.compression_high_release_ms}
          onThreshold={(v) => onUpdate("compression_high_threshold_db", v)}
          onRatio={(v) => onUpdate("compression_high_ratio", v)}
          onAttack={(v) => onUpdate("compression_high_attack_ms", v)}
          onRelease={(v) => onUpdate("compression_high_release_ms", v)}
        />
      </div>
    </details>
  );
}

function CompressionBandColumn({
  label,
  threshold,
  ratio,
  attack,
  release,
  onThreshold,
  onRatio,
  onAttack,
  onRelease,
}: {
  label: string;
  threshold: number | null;
  ratio: number | null;
  attack: number | null;
  release: number | null;
  onThreshold: (v: number | null) => void;
  onRatio: (v: number | null) => void;
  onAttack: (v: number | null) => void;
  onRelease: (v: number | null) => void;
}) {
  return (
    <div className="compression-band-column">
      <div className="compression-band-label">{label}</div>
      <NumberField
        label="Threshold"
        value={threshold}
        step={0.5}
        min={-60}
        max={0}
        format={(v) => `${v.toFixed(1)} dB`}
        onChange={onThreshold}
      />
      <NumberField
        label="Ratio"
        value={ratio}
        step={0.1}
        min={1}
        max={20}
        format={(v) => `${v.toFixed(1)}:1`}
        onChange={onRatio}
      />
      <NumberField
        label="Attack"
        value={attack}
        step={1}
        min={0.5}
        max={200}
        format={(v) => `${v.toFixed(1)} ms`}
        onChange={onAttack}
      />
      <NumberField
        label="Release"
        value={release}
        step={5}
        min={5}
        max={2000}
        format={(v) => `${v.toFixed(0)} ms`}
        onChange={onRelease}
      />
    </div>
  );
}

// Always-on slider for required dB trim values (input gain, output gain).
// No "Auto" affordance — the value is always present (default 0 dB), so the
// slider is always active and double-click resets to 0.
function GainField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (db: number) => void;
}) {
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
    const clamped = Math.max(-24, Math.min(24, parsed));
    if (clamped !== value) onChange(clamped);
    setDraft(null);
  };
  return (
    <div className="adv-field">
      <span className="adv-label">{label}</span>
      <div className="adv-control">
        <input
          type="range"
          min={-24}
          max={24}
          step={0.1}
          value={value}
          onChange={(e) => onChange(parseFloat(e.target.value))}
          onDoubleClick={() => onChange(0)}
          title="Double-click to reset to 0 dB"
        />
        <span className="adv-value">
          {value > 0 ? "+" : ""}{value.toFixed(1)} dB
        </span>
        <input
          ref={inputRef}
          type="number"
          className="adv-number"
          min={-24}
          max={24}
          step={0.1}
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
          title="Type a value or double-click slider to reset to 0 dB"
        />
      </div>
    </div>
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
    <div className={"adv-field " + (value === null ? "is-auto" : "")}>
      <span className="adv-label">
        {label}
        {value === null && <span className="adv-auto-pill">AUTO</span>}
      </span>
      <div className="adv-control">
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={effective}
          // Always live: dragging an Auto slider engages it at the dragged
          // value instead of staying greyed out. Double-click reverts to Auto.
          onChange={(e) => onChange(parseFloat(e.target.value))}
          onDoubleClick={() => onChange(null)}
          title={
            value === null
              ? "Drag to engage. Double-click to leave it on Auto."
              : `Drag or type a value. Double-click slider to reset to Auto.`
          }
        />
        <span className="adv-value">{value === null ? "Auto" : format(value)}</span>
        <input
          ref={inputRef}
          type="number"
          className="adv-number"
          min={min}
          max={max}
          step={step}
          value={draft !== null ? draft : value ?? ""}
          placeholder="auto"
          onChange={(e) => {
            if (e.target.value === "") {
              onChange(null);
              setDraft(null);
            } else {
              setDraft(e.target.value);
            }
          }}
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
          title={
            value === null
              ? "Type a number to engage, or leave blank for Auto."
              : `Type a value or clear to reset to Auto.`
          }
        />
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
