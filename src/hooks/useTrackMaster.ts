import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { api, onPlaybackTick } from "../lib/api";
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
  eq_mid_db: 0,
  eq_high_db: 0,
  volume_match: false,
  advanced: {
    lufs_offset_db: null,
    ceiling_dbtp: null,
    width: null,
    warmth: null,
    presence_air: null,
    compression_density: null,
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

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onPlaybackTick((tick) => {
      setLoadedTrackId(tick.is_loaded ? tick.track_id : null);
      setTransport((t) => ({
        ...t,
        currentTimeSec: tick.position_sec,
        isPlaying: tick.is_playing,
      }));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
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

  const updateSettings = useCallback(
    (id: TrackId, mutate: (prev: MasteringSettings) => MasteringSettings) => {
      const editingAlbumIntent = mode === "album" && !overrideAlbum.has(id);
      let nextSettings: MasteringSettings | undefined;
      if (editingAlbumIntent) {
        // Mutating the album intent affects every track that's following it.
        setAlbumIntent((prev) => {
          nextSettings = mutate(prev);
          return nextSettings;
        });
      } else {
        setSettingsMap((prev) => {
          nextSettings = mutate(prev[id] ?? DEFAULT_SETTINGS);
          return { ...prev, [id]: nextSettings };
        });
        markStale(id);
      }
      // Phase 5 live chain: push fresh coeffs whenever the edit affects a track
      // that's currently loaded as Mastered playback. The previous version of
      // this check gated on `loadedTrackId` (from the backend playback tick),
      // which has a ~50 ms round-trip latency from `playMaster` returning; that
      // window let live edits silently no-op right after starting playback or
      // during fast slider drags. `loadedKindByTrack` is set synchronously in
      // `playWithKind`, so it's the authoritative "is this track playing as
      // master right now?" signal from React's POV.
      if (!nextSettings) return;
      let shouldPush = false;
      if (editingAlbumIntent) {
        // Album intent edit: push if any track currently loaded as master is
        // following the album intent (not overriding).
        shouldPush = Object.entries(loadedKindByTrack).some(
          ([tid, kind]) =>
            kind === "master" && !overrideAlbum.has(tid as TrackId),
        );
      } else {
        // Per-track edit: push if THIS track is currently loaded as master.
        shouldPush = loadedKindByTrack[id] === "master";
      }
      if (shouldPush) {
        api.updateChain(nextSettings).catch((err) => {
          setError(String(err));
        });
      }
    },
    [mode, overrideAlbum, markStale, loadedKindByTrack],
  );

  const toggleOverrideAlbum = useCallback(
    (id: TrackId) => {
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
    [overrideAlbum, albumIntent],
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
      const checks = await api.runExportChecks(report);
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
      if (!loadedCorrectTrack || !loadedCorrectKind) {
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
      if (loadedTrackId === selectedTrack.id) {
        try {
          await api.seekPlayback(clamped);
        } catch (err) {
          setError(String(err));
        }
      }
    },
    [selectedTrack, loadedTrackId],
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
      // shouldPush logic as updateSettings — `loadedKindByTrack` is the
      // synchronous source of truth, not the tick-driven `loadedTrackId`.
      let shouldPush = false;
      if (mode === "album" && !selectedIsOverriding) {
        shouldPush = Object.entries(loadedKindByTrack).some(
          ([tid, kind]) =>
            kind === "master" && !overrideAlbum.has(tid as TrackId),
        );
      } else if (selectedTrackId) {
        shouldPush = loadedKindByTrack[selectedTrackId] === "master";
      }
      if (shouldPush) {
        api.updateChain(preset.settings).catch((err) => setError(String(err)));
      }
    },
    [
      mode,
      selectedIsOverriding,
      selectedTrackId,
      markStale,
      loadedKindByTrack,
      overrideAlbum,
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

    openImportDialog,
    importFiles,
    selectTrack,
    removeTrack,
    setPreset,
    setIntensity,
    setEqBand,
    setAdvanced,
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
  };
}
