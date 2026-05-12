import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, onPlaybackTick } from "../lib/api";
import type {
  AdvancedSettings,
  AnalysisResult,
  ExportReport,
  ImportedTrack,
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
  const [loadedTrackId, setLoadedTrackId] = useState<TrackId | null>(null);

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
      setSettingsMap((prev) => ({
        ...prev,
        [id]: mutate(prev[id] ?? DEFAULT_SETTINGS),
      }));
      markStale(id);
    },
    [markStale],
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
          const results = await api.analyzeTracks(newIds);
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
      setTransport((t) => ({ ...t, isPlaying: false, currentTimeSec: 0 }));
      if (loadedTrackId && loadedTrackId !== id) {
        api.stopPlayback().catch(() => {
          /* swallow — best-effort */
        });
      }
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
    if (!selectedTrackId) return;
    setIsRendering(true);
    setError(null);
    try {
      await api.renderTrackPreview(selectedTrackId, selectedSettings);
      markFresh(selectedTrackId);
    } catch (err) {
      setError(String(err));
    } finally {
      setIsRendering(false);
    }
  }, [selectedTrackId, selectedSettings, markFresh]);

  const exportMaster = useCallback(async () => {
    if (!selectedTrackId || !selectedAnalysis) return;
    setIsExporting(true);
    setError(null);
    try {
      const job = await api.renderTrackMaster(selectedTrackId, selectedSettings);
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
      setLastExportReceipt({ trackId: selectedTrackId, outputPath, checks, job });
    } catch (err) {
      setError(String(err));
    } finally {
      setIsExporting(false);
    }
  }, [selectedTrackId, selectedAnalysis, selectedSettings, selectedTrack]);

  const togglePlay = useCallback(async () => {
    if (!selectedTrack) return;
    setError(null);
    try {
      if (loadedTrackId !== selectedTrack.id) {
        await api.playTrack(selectedTrack.id, selectedTrack.path);
      } else if (transport.isPlaying) {
        await api.pausePlayback();
      } else {
        await api.resumePlayback();
      }
    } catch (err) {
      setError(String(err));
    }
  }, [selectedTrack, loadedTrackId, transport.isPlaying]);

  const setPlaybackKind = useCallback((kind: PlaybackKindUI) => {
    setTransport((t) => ({ ...t, playbackKind: kind }));
  }, []);

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

  const toggleLoop = useCallback(() => {
    setTransport((t) => ({ ...t, loop: !t.loop }));
  }, []);

  const setVolumeMatch = useCallback((on: boolean) => {
    setTransport((t) => ({ ...t, volumeMatch: on }));
  }, []);

  const toggleAdvanced = useCallback(() => {
    setAdvancedOpen((v) => !v);
  }, []);

  const clearError = useCallback(() => setError(null), []);
  const clearExportReceipt = useCallback(() => setLastExportReceipt(null), []);

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
    clearError,
    clearExportReceipt,
  };
}
