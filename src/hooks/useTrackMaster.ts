import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, onPlaybackTick } from "../lib/api";
import type {
  AdvancedSettings,
  AnalysisResult,
  ExportReport,
  ImportedTrack,
  LoopRegion,
  MasteringSettings,
  Preset,
  QualityCheck,
  RenderJob,
  TrackId,
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
  const [mode, setMode] = useState<"track" | "album">("track");
  const [albumIntent, setAlbumIntent] = useState<MasteringSettings>(DEFAULT_SETTINGS);
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

  const selectedTrack = useMemo(
    () => tracks.find((t) => t.id === selectedTrackId),
    [tracks, selectedTrackId],
  );
  const selectedAnalysis = selectedTrackId ? analysisMap[selectedTrackId] : undefined;
  const selectedWaveform = selectedTrackId ? waveformMap[selectedTrackId] : undefined;
  const selectedSettings =
    (selectedTrackId ? settingsMap[selectedTrackId] : undefined) ?? DEFAULT_SETTINGS;
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
      let nextSettings: MasteringSettings | undefined;
      setSettingsMap((prev) => {
        nextSettings = mutate(prev[id] ?? DEFAULT_SETTINGS);
        return { ...prev, [id]: nextSettings };
      });
      markStale(id);
      // Phase 5 live chain: if we're listening to the Mastered version of this
      // track, push the fresh coeffs to the audio thread so changes are audible
      // without re-rendering or re-loading.
      if (
        nextSettings &&
        loadedTrackId === id &&
        loadedKindByTrack[id] === "master"
      ) {
        api.updateChain(nextSettings).catch((err) => {
          setError(String(err));
        });
      }
    },
    [markStale, loadedTrackId, loadedKindByTrack],
  );

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

  const setVolumeMatch = useCallback((on: boolean) => {
    setTransport((t) => ({ ...t, volumeMatch: on }));
  }, []);

  const toggleAdvanced = useCallback(() => {
    setAdvancedOpen((v) => !v);
  }, []);

  const clearError = useCallback(() => setError(null), []);
  const clearExportReceipt = useCallback(() => setLastExportReceipt(null), []);

  const [isExportingAlbum, setIsExportingAlbum] = useState(false);

  const exportAlbum = useCallback(async () => {
    if (tracks.length === 0) return;
    setIsExportingAlbum(true);
    setError(null);
    try {
      const job = await api.renderAlbumMaster(
        tracks.map((t) => ({ id: t.id, path: t.path })),
        albumIntent,
        undefined,
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
  }, [tracks, albumIntent]);

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
  };
}
