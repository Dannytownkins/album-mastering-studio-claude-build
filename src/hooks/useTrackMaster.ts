import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open, save, getCurrentWebview } from "../lib/tauri-runtime";
import { api, onPlaybackTick, onRenderProgress } from "../lib/api";
import type {
  AdvancedSettings,
  AnalysisResult,
  ExportReport,
  ImportedTrack,
  LoopRegion,
  MasteringSettings,
  Preset,
  PresetKind,
  ProjectMode,
  ProjectState,
  QualityCheck,
  RenderJob,
  TrackId,
  UserPreset,
  WaveformPeaks,
} from "../bindings";

const DEFAULT_SETTINGS: MasteringSettings = {
  preset: { kind: "universal" },
  intensity: 0.5,
  eq_low_db: 0,
  eq_low_mid_db: 0,
  eq_mid_db: 0,
  eq_high_db: 0,
  volume_match: false,
  input_gain_db: 0,
  output_gain_db: 0,
  delivery_profile: "streaming-universal",
  advanced: {
    lufs_offset_db: null,
    ceiling_dbtp: null,
    width: null,
    warmth: null,
    presence_air: null,
    compression_density: null,
    compression_low_threshold_db: null,
    compression_low_ratio: null,
    compression_low_attack_ms: null,
    compression_low_release_ms: null,
    compression_mid_threshold_db: null,
    compression_mid_ratio: null,
    compression_mid_attack_ms: null,
    compression_mid_release_ms: null,
    compression_high_threshold_db: null,
    compression_high_ratio: null,
    compression_high_attack_ms: null,
    compression_high_release_ms: null,
    compression_link_stereo: null,
    bit_depth: null,
    target_sample_rate: null,
  },
};

export type PlaybackKindUI = "source" | "master";

export interface ExportReceipt {
  trackId: TrackId;
  outputPath: string;
  checks: QualityCheck[];
  job: RenderJob;
  kind: "track" | "album";
}

const AUDIO_EXTENSIONS = [
  "wav",
  "aiff",
  "aif",
  "flac",
  "mp3",
  "m4a",
  "aac",
  "ogg",
  "opus",
];

export function useTrackMaster() {
  const [tracks, setTracks] = useState<ImportedTrack[]>([]);
  const [selectedTrackId, setSelectedTrackId] = useState<TrackId | null>(null);
  const [analysisMap, setAnalysisMap] = useState<Record<TrackId, AnalysisResult>>({});
  const [waveformMap, setWaveformMap] = useState<Record<TrackId, WaveformPeaks>>({});
  const [settingsMap, setSettingsMap] = useState<Record<TrackId, MasteringSettings>>({});
  const [staleSet, setStaleSet] = useState<Set<TrackId>>(new Set());
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [isLoadingWaveform, setIsLoadingWaveform] = useState(false);
  const [isRendering, setIsRendering] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [transport, setTransport] = useState({
    isPlaying: false,
    currentTimeSec: 0,
    playbackKind: "source" as PlaybackKindUI,
    loop: false,
    volumeMatch: false,
    // Phase 12.2 live clipping meter — post-output-gain peak since the last
    // tick, in dBFS. -120 means "no signal" (silence sentinel from backend).
    // Stored here so the StaleBar's indicator can flash red on clipping
    // without DevTools or an export round-trip.
    peakDbfs: -120,
    // Phase 12.2 per-band compressor GR readouts. -120 = silence sentinel
    // ("no reduction in the window"). Driven by PlaybackTick → snapshot →
    // atomic-swap on the backend audio thread.
    compressionGr: { low: -120, mid: -120, high: -120 },
    // Phase 12.2 P3 — live BS.1770 momentary LUFS. -120 = silence sentinel.
    lufsMomentary: -120,
    // Phase 12.2 P3+ — live BS.1770-4 integrated LUFS over the current
    // playback session.  Resets when a new playback starts.
    lufsIntegrated: -120,
  });
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [lastExportReceipt, setLastExportReceipt] = useState<ExportReceipt | null>(null);
  const [mode, setMode] = useState<ProjectMode>("track");
  const [albumIntent, setAlbumIntent] = useState<MasteringSettings>(DEFAULT_SETTINGS);
  const [sessionLoaded, setSessionLoaded] = useState(false);
  const [overrideAlbum, setOverrideAlbum] = useState<Set<TrackId>>(new Set());
  const [userPresets, setUserPresets] = useState<UserPreset[]>([]);
  const [savingPreset, setSavingPreset] = useState(false);
  const [isDragOver, setIsDragOver] = useState(false);
  const [loadedTrackId, setLoadedTrackId] = useState<TrackId | null>(null);
  const [loadedKindByTrack, setLoadedKindByTrack] = useState<Record<TrackId, PlaybackKindUI>>({});
  const [regionByTrack, setRegionByTrack] = useState<Record<TrackId, LoopRegion | null>>({});
  // Phase 12.1 live-update visibility: tracks how many api.updateChain calls
  // have been attempted and applied. Rendered as a small badge in the UI so
  // Dan can confirm live updates are firing without opening DevTools.
  const [liveUpdateStats, setLiveUpdateStats] = useState<{
    attempts: number;
    applied: number;
    lastAt: number | null;
  }>({ attempts: 0, applied: 0, lastAt: null });
  // Phase 12.1 render progress: backend emits "render:progress" with a 0-1
  // fraction during render_track_preview / render_track_master. Used to
  // render a real progress bar instead of an indeterminate "Rendering…".
  const [renderProgress, setRenderProgress] = useState<{
    fraction: number;
    kind: "preview" | "master" | "album";
  } | null>(null);
  // Phase 7.4 undo/redo: snapshot-based history of the undoable state pieces.
  // Refs (not state) so commitToHistory mutations don't trigger re-renders by
  // themselves; we bump `historyVersion` separately when undo/redo state
  // changes so canUndo/canRedo derived values re-evaluate.
  type HistorySnapshot = {
    settingsMap: Record<string, MasteringSettings>;
    albumIntent: MasteringSettings;
    overrideAlbum: string[];
  };
  const historyPast = useRef<HistorySnapshot[]>([]);
  const historyFuture = useRef<HistorySnapshot[]>([]);
  // Coalesce window: consecutive commits within this many ms collapse into the
  // first snapshot, so a slider drag becomes ONE undo step rather than N.
  const lastCommitAt = useRef<number>(0);
  const [historyVersion, setHistoryVersion] = useState(0);
  const HISTORY_MAX = 100;
  const HISTORY_COALESCE_MS = 300;

  useEffect(() => {
    let unlistenTick: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    onPlaybackTick((tick) => {
      setLoadedTrackId(tick.is_loaded ? tick.track_id : null);
      setTransport((t) => ({
        ...t,
        currentTimeSec: tick.position_sec,
        isPlaying: tick.is_playing,
        peakDbfs: tick.peak_dbfs,
        compressionGr: {
          low: tick.gr_low_db,
          mid: tick.gr_mid_db,
          high: tick.gr_high_db,
        },
        lufsMomentary: tick.lufs_momentary,
        lufsIntegrated: tick.lufs_integrated,
      }));
    }).then((fn) => {
      unlistenTick = fn;
    });
    onRenderProgress((evt) => {
      setRenderProgress({ fraction: evt.fraction, kind: evt.kind });
      // Clear the bar shortly after reaching 1.0 so it doesn't linger.
      if (evt.fraction >= 1.0) {
        setTimeout(() => setRenderProgress(null), 600);
      }
    }).then((fn) => {
      unlistenProgress = fn;
    });
    return () => {
      unlistenTick?.();
      unlistenProgress?.();
    };
  }, []);

  // Phase 7.3: load user presets on mount; subsequent saves/deletes refresh
  // the list directly so we don't need to re-fetch.
  useEffect(() => {
    let cancelled = false;
    api
      .listUserPresets()
      .then((presets) => {
        if (!cancelled) setUserPresets(presets);
      })
      .catch((err) => {
        console.warn("Failed to load user presets", err);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Phase 7.2: load the autosaved session on mount, then enable autosave.
  useEffect(() => {
    let cancelled = false;
    api
      .loadRecentSession()
      .then(async (session) => {
        if (cancelled || !session || session.schema_version !== 1) {
          setSessionLoaded(true);
          return;
        }
        const restoredTracks = session.tracks ?? [];
        if (restoredTracks.length > 0) {
          setTracks(restoredTracks);
          setSelectedTrackId(restoredTracks[0].id);
        }
        if (session.track_settings) setSettingsMap(session.track_settings);
        if (session.mode) setMode(session.mode);
        if (session.album_intent) setAlbumIntent(session.album_intent);
        if (session.track_override_album) {
          setOverrideAlbum(new Set(session.track_override_album));
        }

        // Best-effort re-analyze + re-waveform for restored tracks.
        if (restoredTracks.length > 0) {
          try {
            const results = await api.analyzeTracks(
              restoredTracks.map((t) => ({ id: t.id, path: t.path })),
            );
            if (!cancelled) {
              const map: Record<TrackId, AnalysisResult> = {};
              for (const r of results) map[r.track_id] = r;
              setAnalysisMap(map);
            }
          } catch (err) {
            console.warn("Session restore: analyze failed", err);
          }
          for (const t of restoredTracks) {
            if (cancelled) break;
            try {
              const wf = await api.prepareWaveform(t.id, t.path, 1200);
              setWaveformMap((prev) => ({ ...prev, [t.id]: wf }));
            } catch (err) {
              console.warn(`Session restore: waveform for ${t.display_name} failed`, err);
            }
          }
        }
        setSessionLoaded(true);
      })
      .catch((err) => {
        console.warn("Session load failed", err);
        setSessionLoaded(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Phase 7.2: debounced autosave on relevant state changes.
  useEffect(() => {
    if (!sessionLoaded) return;
    const handle = setTimeout(() => {
      const state: ProjectState = {
        schema_version: 1,
        mode,
        tracks,
        track_order: tracks.map((t) => t.id),
        track_settings: settingsMap,
        album_intent: albumIntent,
        track_override_album: Array.from(overrideAlbum),
        last_saved_iso: new Date().toISOString(),
      };
      api.autosaveSession(state).catch((err) => {
        console.warn("Autosave failed", err);
      });
    }, 1500);
    return () => clearTimeout(handle);
  }, [sessionLoaded, mode, tracks, settingsMap, albumIntent, overrideAlbum]);

  const selectedTrack = useMemo(
    () => tracks.find((t) => t.id === selectedTrackId),
    [tracks, selectedTrackId],
  );
  const selectedAnalysis = selectedTrackId ? analysisMap[selectedTrackId] : undefined;
  const selectedWaveform = selectedTrackId ? waveformMap[selectedTrackId] : undefined;
  const selectedIsOverriding = selectedTrackId
    ? overrideAlbum.has(selectedTrackId)
    : false;
  const followingAlbumIntent = mode === "album" && !selectedIsOverriding && !!selectedTrackId;
  const selectedSettings: MasteringSettings = followingAlbumIntent
    ? albumIntent
    : (selectedTrackId ? settingsMap[selectedTrackId] : undefined) ?? DEFAULT_SETTINGS;
  const previewStale = selectedTrackId ? staleSet.has(selectedTrackId) : false;
  const selectedRegion: LoopRegion | null = selectedTrackId
    ? regionByTrack[selectedTrackId] ?? null
    : null;

  const markStale = useCallback((id: TrackId) => {
    setStaleSet((prev) => {
      const next = new Set(prev);
      next.add(id);
      return next;
    });
  }, []);

  const markFresh = useCallback((id: TrackId) => {
    setStaleSet((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  }, []);

  // Phase 7.4 — snapshot the undoable state pieces and push onto the past
  // stack. Called BEFORE each mutation so the popped state on undo is the
  // pre-mutation state. Coalesces consecutive commits within
  // HISTORY_COALESCE_MS into a single snapshot (the FIRST one in the burst)
  // so a slider drag is one undo step, not N. New commits always clear the
  // redo stack (standard undo/redo semantics).
  const commitToHistory = useCallback(() => {
    const now = Date.now();
    if (now - lastCommitAt.current < HISTORY_COALESCE_MS) {
      // Inside a drag burst — extend the window but don't add a new snapshot.
      lastCommitAt.current = now;
      return;
    }
    lastCommitAt.current = now;
    const snapshot: HistorySnapshot = {
      settingsMap: { ...settingsMap },
      albumIntent: { ...albumIntent },
      overrideAlbum: Array.from(overrideAlbum),
    };
    const past = historyPast.current;
    historyPast.current = past.length >= HISTORY_MAX
      ? [...past.slice(past.length - HISTORY_MAX + 1), snapshot]
      : [...past, snapshot];
    historyFuture.current = [];
    setHistoryVersion((v) => v + 1);
  }, [settingsMap, albumIntent, overrideAlbum]);

  // Restore a snapshot. Helper used by both undo and redo.
  const restoreSnapshot = useCallback(
    (snapshot: HistorySnapshot) => {
      setSettingsMap(snapshot.settingsMap);
      setAlbumIntent(snapshot.albumIntent);
      setOverrideAlbum(new Set(snapshot.overrideAlbum as TrackId[]));
      // After restoring state, push the restored settings to the live audio
      // chain if the affected track is currently playing as Mastered. Without
      // this, undo would change the UI state but the audible output would lag
      // until the user toggled Original/Master or made another adjustment.
      const id = selectedTrackId;
      if (
        id &&
        (loadedKindByTrack[id] === "master" ||
          (loadedTrackId === id && loadedKindByTrack[id] !== "source"))
      ) {
        const followingAlbum =
          mode === "album" && !snapshot.overrideAlbum.includes(id as string);
        const effective = followingAlbum
          ? snapshot.albumIntent
          : snapshot.settingsMap[id as string] ?? DEFAULT_SETTINGS;
        setLiveUpdateStats((s) => ({
          attempts: s.attempts + 1,
          applied: s.applied,
          lastAt: Date.now(),
        }));
        api
          .updateChain(effective)
          .then(() => {
            setLiveUpdateStats((s) => ({
              attempts: s.attempts,
              applied: s.applied + 1,
              lastAt: Date.now(),
            }));
          })
          .catch((err) => setError(String(err)));
      }
    },
    [selectedTrackId, loadedKindByTrack, loadedTrackId, mode],
  );

  const undo = useCallback(() => {
    const past = historyPast.current;
    if (past.length === 0) return;
    const snapshot = past[past.length - 1];
    const current: HistorySnapshot = {
      settingsMap: { ...settingsMap },
      albumIntent: { ...albumIntent },
      overrideAlbum: Array.from(overrideAlbum),
    };
    historyPast.current = past.slice(0, -1);
    historyFuture.current = [...historyFuture.current, current];
    // Reset the coalesce window so the NEXT user edit always commits a new
    // snapshot rather than collapsing into the just-restored state.
    lastCommitAt.current = 0;
    restoreSnapshot(snapshot);
    setHistoryVersion((v) => v + 1);
  }, [settingsMap, albumIntent, overrideAlbum, restoreSnapshot]);

  const redo = useCallback(() => {
    const future = historyFuture.current;
    if (future.length === 0) return;
    const snapshot = future[future.length - 1];
    const current: HistorySnapshot = {
      settingsMap: { ...settingsMap },
      albumIntent: { ...albumIntent },
      overrideAlbum: Array.from(overrideAlbum),
    };
    historyFuture.current = future.slice(0, -1);
    historyPast.current = [...historyPast.current, current];
    lastCommitAt.current = 0;
    restoreSnapshot(snapshot);
    setHistoryVersion((v) => v + 1);
  }, [settingsMap, albumIntent, overrideAlbum, restoreSnapshot]);

  const canUndo = historyPast.current.length > 0;
  const canRedo = historyFuture.current.length > 0;
  // historyVersion intentionally referenced here so the closures above
  // re-evaluate canUndo / canRedo on each render after a history change.
  void historyVersion;

  const updateSettings = useCallback(
    (id: TrackId, mutate: (prev: MasteringSettings) => MasteringSettings) => {
      const editingAlbumIntent = mode === "album" && !overrideAlbum.has(id);
      // Phase 7.4: capture pre-mutation state for undo. Coalesces within
      // HISTORY_COALESCE_MS so slider drags are one undo step.
      commitToHistory();
      // Compute `nextSettings` from the CURRENT-RENDER closure values, not
      // from inside a setState updater. React 18's batched-updates model
      // makes side-effect assignments inside `setState((prev) => ...)`
      // unreliable when the call site needs to read the result synchronously;
      // pulling the current state into a local variable here removes that
      // hazard entirely so the api.updateChain call below always has a
      // defined value.
      let nextSettings: MasteringSettings;
      if (editingAlbumIntent) {
        nextSettings = mutate(albumIntent);
        setAlbumIntent(nextSettings);
      } else {
        const current = settingsMap[id] ?? DEFAULT_SETTINGS;
        nextSettings = mutate(current);
        setSettingsMap((prev) => ({ ...prev, [id]: nextSettings }));
        markStale(id);
      }

      // Push to live chain when the edit reaches the currently-playing master.
      // We accept either the synchronous `loadedKindByTrack` map (set in
      // playWithKind) or the tick-driven `loadedTrackId` as evidence — a track
      // is "playing as master" if EITHER signal agrees. Belt-and-suspenders
      // covers the case where one signal is briefly stale.
      let shouldPush = false;
      if (editingAlbumIntent) {
        shouldPush = Object.entries(loadedKindByTrack).some(
          ([tid, kind]) =>
            kind === "master" && !overrideAlbum.has(tid as TrackId),
        );
      } else {
        const kindForId = loadedKindByTrack[id];
        shouldPush =
          kindForId === "master" || (loadedTrackId === id && kindForId !== "source");
      }
      if (shouldPush) {
        setLiveUpdateStats((s) => ({
          attempts: s.attempts + 1,
          applied: s.applied,
          lastAt: Date.now(),
        }));
        api
          .updateChain(nextSettings)
          .then(() => {
            setLiveUpdateStats((s) => ({
              attempts: s.attempts,
              applied: s.applied + 1,
              lastAt: Date.now(),
            }));
          })
          .catch((err) => {
            setError(String(err));
          });
      }
    },
    [
      mode,
      overrideAlbum,
      markStale,
      loadedKindByTrack,
      loadedTrackId,
      albumIntent,
      settingsMap,
      commitToHistory,
    ],
  );

  const toggleOverrideAlbum = useCallback(
    (id: TrackId) => {
      commitToHistory();
      const wasOverriding = overrideAlbum.has(id);
      setOverrideAlbum((prev) => {
        const next = new Set(prev);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      });
      if (!wasOverriding) {
        // Entering override — seed per-track settings from the current album
        // intent so the user has a sensible starting point to deviate from.
        setSettingsMap((prev) => ({ ...prev, [id]: { ...albumIntent } }));
      }
    },
    [overrideAlbum, albumIntent, commitToHistory],
  );

  // Stable-ref to importFiles so the drag-drop listener effect doesn't
  // re-attach on every render of the hook.
  const importFilesRef = useRef<(paths: string[]) => Promise<void>>(async () => {});

  const importFiles = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      setError(null);
      try {
        const imported = await api.importTracks(paths);
        if (imported.length === 0) return;

        setTracks((prev) => [...prev, ...imported]);
        setSettingsMap((prev) => {
          const next = { ...prev };
          for (const t of imported) next[t.id] = DEFAULT_SETTINGS;
          return next;
        });

        const newIds = imported.map((t) => t.id);
        for (const id of newIds) markStale(id);

        if (selectedTrackId === null) {
          setSelectedTrackId(imported[0].id);
        }

        setIsAnalyzing(true);
        try {
          const results = await api.analyzeTracks(
            imported.map((t) => ({ id: t.id, path: t.path })),
          );
          setAnalysisMap((prev) => {
            const next = { ...prev };
            for (const r of results) next[r.track_id] = r;
            return next;
          });
          setSettingsMap((prev) => {
            const next = { ...prev };
            for (const r of results) {
              const current = next[r.track_id] ?? DEFAULT_SETTINGS;
              if (current.preset.kind === "universal") {
                next[r.track_id] = r.recommended_universal;
              }
            }
            return next;
          });
        } finally {
          setIsAnalyzing(false);
        }

        setIsLoadingWaveform(true);
        try {
          for (const track of imported) {
            const wf = await api.prepareWaveform(track.id, track.path, 1200);
            setWaveformMap((prev) => ({ ...prev, [track.id]: wf }));
          }
        } finally {
          setIsLoadingWaveform(false);
        }
      } catch (err) {
        setError(String(err));
      }
    },
    [selectedTrackId, markStale],
  );

  // Keep the ref in sync with the latest importFiles closure so the long-lived
  // drag-drop listener always calls the freshest version (with the current
  // `selectedTrackId` selection logic etc.).
  importFilesRef.current = importFiles;

  // Tauri's window-level drag/drop listener. Attaches once on mount, lives for
  // the lifetime of the hook. Filters dropped paths by audio extension so we
  // ignore non-audio files quietly instead of failing import.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (cancelled) return;
        const payload = event.payload as {
          type: "enter" | "over" | "drop" | "leave";
          paths?: string[];
        };
        if (payload.type === "enter") {
          setIsDragOver(true);
        } else if (payload.type === "leave") {
          setIsDragOver(false);
        } else if (payload.type === "drop") {
          setIsDragOver(false);
          const all = payload.paths ?? [];
          const audio = all.filter((p) => {
            const dot = p.lastIndexOf(".");
            if (dot < 0) return false;
            const ext = p.slice(dot + 1).toLowerCase();
            return AUDIO_EXTENSIONS.includes(ext);
          });
          if (audio.length > 0) {
            importFilesRef.current(audio).catch((err) => {
              console.warn("drag-drop import failed", err);
            });
          }
        }
      })
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((err) => {
        console.warn("Failed to attach drag-drop listener", err);
      });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const openImportDialog = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      await importFiles(paths);
    } catch (err) {
      setError(String(err));
    }
  }, [importFiles]);

  const selectTrack = useCallback(
    (id: TrackId) => {
      setSelectedTrackId(id);
      setTransport((t) => ({ ...t, isPlaying: false, currentTimeSec: 0, loop: false }));
      if (loadedTrackId && loadedTrackId !== id) {
        api.stopPlayback().catch(() => {
          /* swallow — best-effort */
        });
      }
      api.setLoopRegion(null).catch(() => {
        /* swallow — best-effort */
      });
    },
    [loadedTrackId],
  );

  const removeTrack = useCallback(
    (id: TrackId) => {
      setTracks((prev) => prev.filter((t) => t.id !== id));
      setAnalysisMap((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      setWaveformMap((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      setSettingsMap((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      setStaleSet((prev) => {
        const next = new Set(prev);
        next.delete(id);
        return next;
      });
      setLoadedKindByTrack((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      if (loadedTrackId === id) {
        api.stopPlayback().catch(() => {
          /* swallow — best-effort */
        });
      }
      if (selectedTrackId === id) {
        const remaining = tracks.filter((t) => t.id !== id);
        setSelectedTrackId(remaining.length > 0 ? remaining[0].id : null);
      }
    },
    [selectedTrackId, tracks],
  );

  const setPreset = useCallback(
    (preset: Preset) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => ({ ...prev, preset }));
    },
    [selectedTrackId, updateSettings],
  );

  const setIntensity = useCallback(
    (intensity: number) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => ({ ...prev, intensity }));
    },
    [selectedTrackId, updateSettings],
  );

  const setEqBand = useCallback(
    (band: "low" | "mid" | "high", db: number) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => {
        const next = { ...prev };
        if (band === "low") next.eq_low_db = db;
        else if (band === "mid") next.eq_mid_db = db;
        else next.eq_high_db = db;
        return next;
      });
    },
    [selectedTrackId, updateSettings],
  );

  const setAdvanced = useCallback(
    (advanced: AdvancedSettings) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => ({ ...prev, advanced }));
    },
    [selectedTrackId, updateSettings],
  );

  const setInputGain = useCallback(
    (db: number) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => ({ ...prev, input_gain_db: db }));
    },
    [selectedTrackId, updateSettings],
  );

  const setOutputGain = useCallback(
    (db: number) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => ({ ...prev, output_gain_db: db }));
    },
    [selectedTrackId, updateSettings],
  );

  // Phase B — Album Master mode controls. Stored on the hook (not
  // serialized in MasteringSettings.album yet — the AlbumPlan is rebuilt
  // at export time from current tracks + analyses + arc + intensity, so
  // the hook only persists the user's *choice* of arc and intensity).
  const [albumArcKind, setAlbumArcKind] =
    useState<import("../bindings").AlbumArcKind>("cinematic");
  const [albumIntensity, setAlbumIntensityState] = useState<number>(1.0);
  const [albumTitle, setAlbumTitle] = useState<string>("");
  const [albumRendering, setAlbumRendering] = useState<boolean>(false);
  const [albumExportReport, setAlbumExportReport] =
    useState<import("../lib/api").AlbumRenderReport | null>(null);

  const setAlbumArc = useCallback(
    (kind: import("../bindings").AlbumArcKind) => setAlbumArcKind(kind),
    [],
  );
  const setAlbumIntensity = useCallback((v: number) => {
    setAlbumIntensityState(Math.max(0, Math.min(2, v)));
  }, []);

  /// Phase B: build + render the album via the new AlbumPlan path. Picks
  /// up the current tracks, per-track analyses, per-track settings,
  /// current arc + intensity, and hands it to the backend. Returns the
  /// AlbumRenderReport via `albumExportReport` state. Distinct from the
  /// legacy `exportAlbum` (below) which uses the older
  /// `render_album_master` command + per-track-override flow.
  const exportAlbumPlan = useCallback(async () => {
    if (tracks.length === 0) return;
    setAlbumRendering(true);
    setError(null);
    try {
      const analyses = tracks
        .map((t) => analysisMap[t.id])
        .filter((a): a is AnalysisResult => !!a);
      if (analyses.length !== tracks.length) {
        throw new Error(
          "Analyze all tracks before exporting the album (some are missing analysis).",
        );
      }
      const durations = tracks.map((t) => t.duration_seconds ?? 0);
      const arc: import("../bindings").AlbumArc = {
        kind: "preset",
        preset: albumArcKind,
      };
      const title = albumTitle.trim() || tracks[0]?.display_name || "Album";
      const plan = await api.planAlbum(
        title,
        analyses,
        durations,
        arc,
        albumIntensity,
      );
      const renderTracks: import("../lib/api").AlbumTrackRenderInput[] =
        plan.tracks.map((entry) => {
          const settings = settingsMap[entry.track_id] ?? albumIntent;
          const sourceTrack = tracks.find((t) => t.id === entry.track_id);
          return {
            track_id: entry.track_id,
            source_path: sourceTrack?.path ?? "",
            settings,
          };
        });
      const report = await api.renderAlbumPlan(plan, renderTracks);
      setAlbumExportReport(report);
    } catch (err) {
      setError(String(err));
    } finally {
      setAlbumRendering(false);
    }
  }, [
    tracks,
    analysisMap,
    settingsMap,
    albumIntent,
    albumArcKind,
    albumIntensity,
    albumTitle,
  ]);

  /// Phase A3 — pick a delivery profile. Replaces lufs_offset_db /
  /// ceiling_dbtp / bit_depth at render time when non-`custom`. Picking
  /// `custom` doesn't touch the user's existing advanced fields.
  const setDeliveryProfile = useCallback(
    (profile: MasteringSettings["delivery_profile"]) => {
      if (!selectedTrackId) return;
      updateSettings(selectedTrackId, (prev) => ({
        ...prev,
        delivery_profile: profile,
      }));
    },
    [selectedTrackId, updateSettings],
  );

  const updatePreview = useCallback(async () => {
    if (!selectedTrackId || !selectedTrack) return;
    setIsRendering(true);
    setError(null);
    try {
      // Phase 5: Mastered playback runs through the live chain, so a "preview
      // render" no longer needs to swap the audio source. The button still
      // produces an offline WAV (useful when auditing the would-be master in
      // another player) and clears the stale flag for export bookkeeping.
      await api.renderTrackPreview(selectedTrackId, selectedTrack.path, selectedSettings);
      markFresh(selectedTrackId);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsRendering(false);
    }
  }, [selectedTrackId, selectedTrack, selectedSettings, markFresh]);

  const exportMaster = useCallback(async () => {
    if (!selectedTrackId || !selectedAnalysis) return;
    setIsExporting(true);
    setError(null);
    try {
      if (!selectedTrack) return;
      const job = await api.renderTrackMaster(
        selectedTrackId,
        selectedTrack.path,
        selectedSettings,
      );
      const outputPath = job.output_paths[0] ?? "";
      const report: ExportReport = {
        track_id: selectedTrackId,
        output_path: outputPath,
        measured_lufs: selectedAnalysis.lufs_integrated,
        measured_true_peak_dbtp: selectedAnalysis.true_peak_dbtp,
        measured_dynamic_range_lu: selectedAnalysis.dynamic_range_lu,
        source_format: selectedTrack?.source_format ?? "unknown",
        destination_format: "wav",
        sample_rate: 44_100,
        bit_depth: selectedSettings.advanced.bit_depth ?? 24,
        checks: [],
      };
      const checks = await api.runExportChecks(report, selectedAnalysis, selectedSettings);
      setLastExportReceipt({
        trackId: selectedTrackId,
        outputPath,
        checks,
        job,
        kind: "track",
      });
    } catch (err) {
      setError(String(err));
    } finally {
      setIsExporting(false);
    }
  }, [selectedTrackId, selectedAnalysis, selectedSettings, selectedTrack]);

  const playWithKind = useCallback(
    async (kind: PlaybackKindUI, positionSec: number) => {
      if (!selectedTrack || !selectedTrackId) return;
      if (kind === "source") {
        await api.playTrack(selectedTrackId, selectedTrack.path, positionSec);
      } else {
        // Phase 5: mastered playback streams the source through the live DSP
        // chain — no offline render required, settings changes are audible
        // immediately via updateChain.
        await api.playMaster(
          selectedTrackId,
          selectedTrack.path,
          selectedSettings,
          positionSec,
        );
      }
      setLoadedKindByTrack((prev) => ({ ...prev, [selectedTrackId]: kind }));
    },
    [selectedTrack, selectedTrackId, selectedSettings],
  );

  const togglePlay = useCallback(async () => {
    if (!selectedTrack || !selectedTrackId) return;
    setError(null);
    try {
      const loadedCorrectTrack = loadedTrackId === selectedTrackId;
      const loadedCorrectKind = loadedKindByTrack[selectedTrackId] === transport.playbackKind;
      // Detect end-of-track: when a song finishes the sink empties but the
      // backend still reports is_loaded=true, so the previous code path
      // called resumePlayback() on a dead sink and nothing happened.  If
      // the playhead is at (or essentially at) the duration AND we're not
      // currently playing, treat this as "re-load and play from start".
      const duration = selectedTrack.duration_seconds ?? Infinity;
      const isAtEnd =
        Number.isFinite(duration) &&
        transport.currentTimeSec >= duration - 0.5 &&
        !transport.isPlaying;
      if (!loadedCorrectTrack || !loadedCorrectKind || isAtEnd) {
        await playWithKind(transport.playbackKind, 0);
      } else if (transport.isPlaying) {
        await api.pausePlayback();
      } else {
        await api.resumePlayback();
      }
    } catch (err) {
      setError(String(err));
    }
  }, [
    selectedTrack,
    selectedTrackId,
    loadedTrackId,
    loadedKindByTrack,
    transport.playbackKind,
    transport.isPlaying,
    transport.currentTimeSec,
    playWithKind,
  ]);

  // Spacebar = toggle play/pause. Skip when focus is in a form control so
  // typing in a value-input field still works (Phase 12.1 Dan feedback flagged
  // this as mandatory). preventDefault stops the page from also scrolling.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.code !== "Space" && e.key !== " ") return;
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName;
      const isFormField =
        tag === "INPUT" ||
        tag === "TEXTAREA" ||
        tag === "SELECT" ||
        (target?.isContentEditable ?? false);
      if (isFormField) return;
      e.preventDefault();
      void togglePlay();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [togglePlay]);

  // Phase 7.4 — Ctrl/Cmd+Z = undo, Ctrl/Cmd+Shift+Z (or Ctrl/Cmd+Y) = redo.
  // Skips when focus is in a text-editable field so the system-native
  // undo in those inputs still works.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const isCtrl = e.ctrlKey || e.metaKey;
      if (!isCtrl) return;
      const target = e.target as HTMLElement | null;
      const tag = target?.tagName;
      const isTextField =
        tag === "TEXTAREA" ||
        (tag === "INPUT" &&
          (target as HTMLInputElement | null)?.type !== "range") ||
        (target?.isContentEditable ?? false);
      if (isTextField) return;
      const key = e.key.toLowerCase();
      if (key === "z") {
        e.preventDefault();
        if (e.shiftKey) redo();
        else undo();
      } else if (key === "y") {
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [undo, redo]);

  const setPlaybackKind = useCallback(
    async (kind: PlaybackKindUI) => {
      if (!selectedTrackId) return;
      setTransport((t) => ({ ...t, playbackKind: kind }));
      // Mid-playback swap: if this track is currently loaded, switch source at the current position.
      if (loadedTrackId === selectedTrackId) {
        setError(null);
        try {
          await playWithKind(kind, transport.currentTimeSec);
        } catch (err) {
          setError(String(err));
        }
      }
    },
    [selectedTrackId, loadedTrackId, transport.currentTimeSec, playWithKind],
  );

  const seek = useCallback(
    async (positionSec: number) => {
      if (!selectedTrack) return;
      const clamped = Math.max(0, positionSec);
      setTransport((t) => ({ ...t, currentTimeSec: clamped }));
      // If the track ran to the end the sink is empty and seekPlayback is a
      // no-op on the backend. Re-prepare the source at the click position
      // so the next play actually starts from the new playhead.
      const duration = selectedTrack.duration_seconds ?? Infinity;
      const wasAtEnd =
        Number.isFinite(duration) &&
        transport.currentTimeSec >= duration - 0.5 &&
        !transport.isPlaying;
      if (loadedTrackId === selectedTrack.id) {
        try {
          if (wasAtEnd) {
            // Re-prep at the new offset (this also unpauses; the user
            // intends "play from here" after a finish).
            await playWithKind(transport.playbackKind, clamped);
          } else {
            await api.seekPlayback(clamped);
          }
        } catch (err) {
          setError(String(err));
        }
      }
    },
    [selectedTrack, loadedTrackId, transport.currentTimeSec, transport.isPlaying, transport.playbackKind, playWithKind],
  );

  const toggleLoop = useCallback(async () => {
    const nextLoop = !transport.loop;
    setTransport((t) => ({ ...t, loop: nextLoop }));
    try {
      if (nextLoop && selectedRegion) {
        await api.setLoopRegion(selectedRegion);
      } else {
        await api.setLoopRegion(null);
      }
    } catch (err) {
      setError(String(err));
    }
  }, [transport.loop, selectedRegion]);

  const setRegion = useCallback(
    async (region: LoopRegion) => {
      if (!selectedTrackId) return;
      setRegionByTrack((prev) => ({ ...prev, [selectedTrackId]: region }));
      if (transport.loop) {
        try {
          await api.setLoopRegion(region);
        } catch (err) {
          setError(String(err));
        }
      }
    },
    [selectedTrackId, transport.loop],
  );

  const clearRegion = useCallback(async () => {
    if (!selectedTrackId) return;
    setRegionByTrack((prev) => {
      const next = { ...prev };
      delete next[selectedTrackId];
      return next;
    });
    if (transport.loop) {
      try {
        await api.setLoopRegion(null);
      } catch (err) {
        setError(String(err));
      }
    }
  }, [selectedTrackId, transport.loop]);

  const setVolumeMatch = useCallback(
    (on: boolean) => {
      setTransport((t) => ({ ...t, volumeMatch: on }));
      // Route through updateSettings so the DSP chain picks up the change
      // (live for Mastered playback via api.updateChain, persisted to
      // settingsMap or albumIntent depending on mode). Source playback is
      // unaffected — it never goes through the chain.
      if (selectedTrackId) {
        updateSettings(selectedTrackId, (prev) => ({
          ...prev,
          volume_match: on,
        }));
      }
    },
    [selectedTrackId, updateSettings],
  );

  const toggleAdvanced = useCallback(() => {
    setAdvancedOpen((v) => !v);
  }, []);

  const clearError = useCallback(() => setError(null), []);
  const clearExportReceipt = useCallback(() => setLastExportReceipt(null), []);

  const saveUserPreset = useCallback(
    async (name: string) => {
      const trimmed = name.trim();
      if (!trimmed) {
        setError("Preset name cannot be empty");
        return;
      }
      setSavingPreset(true);
      setError(null);
      try {
        const kind: PresetKind = mode === "album" ? "album" : "track";
        const snapshot = followingAlbumIntent ? albumIntent : selectedSettings;
        const created = await api.saveUserPreset(trimmed, kind, snapshot);
        setUserPresets((prev) => [...prev, created]);
      } catch (err) {
        setError(String(err));
      } finally {
        setSavingPreset(false);
      }
    },
    [mode, followingAlbumIntent, albumIntent, selectedSettings],
  );

  const deleteUserPreset = useCallback(async (id: string) => {
    try {
      await api.deleteUserPreset(id);
      setUserPresets((prev) => prev.filter((p) => p.id !== id));
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const applyUserPreset = useCallback(
    (preset: UserPreset) => {
      commitToHistory();
      if (mode === "album" && !selectedIsOverriding) {
        // Apply to album intent.
        setAlbumIntent(preset.settings);
      } else if (selectedTrackId) {
        setSettingsMap((prev) => ({
          ...prev,
          [selectedTrackId]: preset.settings,
        }));
        markStale(selectedTrackId);
      }
      // Push to live chain if currently playing the affected master. Same
      // belt-and-suspenders signal as updateSettings: accept either the
      // synchronous loadedKindByTrack map or the tick-driven loadedTrackId.
      let shouldPush = false;
      if (mode === "album" && !selectedIsOverriding) {
        shouldPush = Object.entries(loadedKindByTrack).some(
          ([tid, kind]) =>
            kind === "master" && !overrideAlbum.has(tid as TrackId),
        );
      } else if (selectedTrackId) {
        const kindForId = loadedKindByTrack[selectedTrackId];
        shouldPush =
          kindForId === "master" ||
          (loadedTrackId === selectedTrackId && kindForId !== "source");
      }
      if (shouldPush) {
        setLiveUpdateStats((s) => ({
          attempts: s.attempts + 1,
          applied: s.applied,
          lastAt: Date.now(),
        }));
        api
          .updateChain(preset.settings)
          .then(() => {
            setLiveUpdateStats((s) => ({
              attempts: s.attempts,
              applied: s.applied + 1,
              lastAt: Date.now(),
            }));
          })
          .catch((err) => setError(String(err)));
      }
    },
    [
      mode,
      selectedIsOverriding,
      selectedTrackId,
      markStale,
      loadedKindByTrack,
      loadedTrackId,
      overrideAlbum,
      commitToHistory,
    ],
  );

  const [isExportingAlbum, setIsExportingAlbum] = useState(false);

  const exportAlbum = useCallback(async () => {
    if (tracks.length === 0) return;
    setIsExportingAlbum(true);
    setError(null);
    try {
      // Build per-track overrides from the override set. Only include entries
      // that actually have settings (most do, since toggleOverrideAlbum seeds
      // from albumIntent).
      const perTrackOverrides: Record<string, MasteringSettings> = {};
      for (const id of overrideAlbum) {
        const s = settingsMap[id];
        if (s) perTrackOverrides[id] = s;
      }
      const overridesArg =
        Object.keys(perTrackOverrides).length > 0 ? perTrackOverrides : undefined;

      const job = await api.renderAlbumMaster(
        tracks.map((t) => ({ id: t.id, path: t.path })),
        albumIntent,
        overridesArg,
      );
      const continuousPath = job.output_paths[0] ?? "";
      setLastExportReceipt({
        trackId: tracks[0]?.id ?? "album",
        outputPath: continuousPath,
        checks: [],
        job,
        kind: "album",
      });
    } catch (err) {
      setError(String(err));
    } finally {
      setIsExportingAlbum(false);
    }
  }, [tracks, albumIntent, overrideAlbum, settingsMap]);

  // Phase 12.2 P3 — explicit Save As / Open Project for .ams.json files.
  // Autosave still runs every 1.5 s into app_data/session.json; these flows
  // let the user park a named snapshot anywhere on disk and reload it.
  const saveProjectAs = useCallback(async () => {
    try {
      const defaultName =
        (selectedTrack?.display_name ?? "untitled-project").replace(
          /[^a-z0-9-_]+/gi,
          "_",
        ) + ".ams.json";
      const path = await save({
        defaultPath: defaultName,
        filters: [
          {
            name: "Album Mastering Studio project",
            extensions: ["ams.json", "json"],
          },
        ],
      });
      if (!path) return;
      const state: ProjectState = {
        schema_version: 1,
        mode,
        tracks,
        track_order: tracks.map((t) => t.id),
        track_settings: settingsMap,
        album_intent: albumIntent,
        track_override_album: Array.from(overrideAlbum),
        last_saved_iso: new Date().toISOString(),
      };
      await api.saveProject(path, state);
    } catch (err) {
      setError(String(err));
    }
  }, [
    selectedTrack,
    mode,
    tracks,
    settingsMap,
    albumIntent,
    overrideAlbum,
  ]);

  const openProjectFromDisk = useCallback(async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Album Mastering Studio project",
            extensions: ["ams.json", "json"],
          },
        ],
      });
      if (!selected) return;
      const path = Array.isArray(selected) ? selected[0] : selected;
      const state = await api.loadProject(path);
      if (state.schema_version !== 1) {
        setError(`Unsupported project schema: v${state.schema_version}`);
        return;
      }
      setTracks(state.tracks ?? []);
      setSettingsMap(state.track_settings ?? {});
      setMode(state.mode);
      if (state.album_intent) setAlbumIntent(state.album_intent);
      setOverrideAlbum(new Set(state.track_override_album ?? []));
      if (state.tracks && state.tracks.length > 0) {
        setSelectedTrackId(state.tracks[0].id);
      } else {
        setSelectedTrackId(null);
      }
      // Best-effort re-analyze + re-waveform for the restored tracks so the
      // user lands in a working state without manually pressing Analyze.
      if (state.tracks && state.tracks.length > 0) {
        try {
          const results = await api.analyzeTracks(
            state.tracks.map((t) => ({ id: t.id, path: t.path })),
          );
          const nextAnalysis: Record<TrackId, AnalysisResult> = {};
          for (const r of results) nextAnalysis[r.track_id] = r;
          setAnalysisMap(nextAnalysis);
        } catch (err) {
          console.warn("Re-analyze on open failed", err);
        }
        for (const t of state.tracks) {
          try {
            const wf = await api.prepareWaveform(t.id, t.path, 1200);
            setWaveformMap((prev) => ({ ...prev, [t.id]: wf }));
          } catch (err) {
            console.warn(`Waveform re-decode failed for ${t.display_name}`, err);
          }
        }
      }
    } catch (err) {
      setError(String(err));
    }
  }, []);

  const reorderTracks = useCallback((fromIndex: number, toIndex: number) => {
    setTracks((prev) => {
      if (
        fromIndex < 0 ||
        fromIndex >= prev.length ||
        toIndex < 0 ||
        toIndex >= prev.length ||
        fromIndex === toIndex
      ) {
        return prev;
      }
      const next = prev.slice();
      const [moved] = next.splice(fromIndex, 1);
      next.splice(toIndex, 0, moved);
      return next;
    });
  }, []);

  const updateAlbumIntent = useCallback(
    (mutate: (prev: MasteringSettings) => MasteringSettings) => {
      setAlbumIntent((prev) => mutate(prev));
    },
    [],
  );

  return {
    tracks,
    selectedTrackId,
    selectedTrack,
    selectedAnalysis,
    selectedWaveform,
    selectedSettings,
    previewStale,
    isAnalyzing,
    isLoadingWaveform,
    isRendering,
    isExporting,
    error,
    transport,
    advancedOpen,
    lastExportReceipt,
    liveUpdateStats,
    renderProgress,
    undo,
    redo,
    canUndo,
    canRedo,

    openImportDialog,
    importFiles,
    selectTrack,
    removeTrack,
    setPreset,
    setIntensity,
    setEqBand,
    setAdvanced,
    setInputGain,
    setOutputGain,
    setDeliveryProfile,
    // Phase B — Album Master controls.
    albumArcKind,
    albumIntensity,
    albumTitle,
    albumRendering,
    albumExportReport,
    setAlbumArc,
    setAlbumIntensity,
    setAlbumTitle,
    exportAlbumPlan,
    updatePreview,
    exportMaster,
    togglePlay,
    seek,
    setPlaybackKind,
    toggleLoop,
    setVolumeMatch,
    toggleAdvanced,
    selectedRegion,
    setRegion,
    clearRegion,
    clearError,
    clearExportReceipt,
    mode,
    setMode,
    reorderTracks,
    albumIntent,
    updateAlbumIntent,
    isExportingAlbum,
    exportAlbum,
    overrideAlbum,
    selectedIsOverriding,
    followingAlbumIntent,
    toggleOverrideAlbum,
    userPresets,
    savingPreset,
    saveUserPreset,
    deleteUserPreset,
    applyUserPreset,
    isDragOver,
    saveProjectAs,
    openProjectFromDisk,
  };
}
