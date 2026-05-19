import type { MasteringSettings, Preset } from "../bindings";

export type CompressionBand = "low" | "mid" | "high";

export interface CompressorAutoBandValues {
  thresholdDb: number;
  ratio: number;
  attackMs: number;
  releaseMs: number;
  thresholdLabel: string;
  ratioLabel: string;
  attackLabel: string;
  releaseLabel: string;
}

interface PresetCompressorCalibration {
  thresholdDb: number;
  ratio: number;
  attackMs: number;
  releaseMs: number;
}

const PRESET_COMPRESSOR: Record<Preset["kind"], PresetCompressorCalibration> = {
  universal: { thresholdDb: -16, ratio: 1.8, attackMs: 15, releaseMs: 250 },
  clarity: { thresholdDb: -16, ratio: 1.8, attackMs: 12, releaseMs: 150 },
  tape: { thresholdDb: -22, ratio: 2.4, attackMs: 30, releaseMs: 400 },
  spatial: { thresholdDb: -16, ratio: 1.8, attackMs: 15, releaseMs: 250 },
  oomph: { thresholdDb: -22, ratio: 2.6, attackMs: 25, releaseMs: 280 },
  warmth: { thresholdDb: -19, ratio: 2.0, attackMs: 20, releaseMs: 280 },
  punch: { thresholdDb: -20, ratio: 2.8, attackMs: 10, releaseMs: 100 },
  loud: { thresholdDb: -23, ratio: 3.5, attackMs: 15, releaseMs: 180 },
  custom: { thresholdDb: -16, ratio: 1.8, attackMs: 15, releaseMs: 200 },
};

const OVERDRIVE_THRESHOLD_DB = -3;
const OVERDRIVE_RATIO = 0.5;

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function formatMs(value: number): string {
  return Number.isInteger(value) ? `${value.toFixed(0)} ms` : `${value.toFixed(1)} ms`;
}

export function compressorAutoReadouts(
  settings: MasteringSettings,
): Record<CompressionBand, CompressorAutoBandValues> {
  const preset = PRESET_COMPRESSOR[settings.preset.kind];
  const defaultDensity = settings.preset.kind === "custom" ? 0 : 0.5;
  const density = clamp01(settings.advanced.compression_density ?? defaultDensity);
  const engagement = Math.min(density * 2, 1);
  const overdrive = Math.max(density * 2 - 1, 0);

  const thresholdDb =
    preset.thresholdDb * engagement + OVERDRIVE_THRESHOLD_DB * overdrive;
  const ratio = Math.max(
    1,
    1 + (preset.ratio - 1) * engagement + OVERDRIVE_RATIO * overdrive,
  );

  const values: CompressorAutoBandValues = {
    thresholdDb,
    ratio,
    attackMs: preset.attackMs,
    releaseMs: preset.releaseMs,
    thresholdLabel: `${thresholdDb.toFixed(1)} dB`,
    ratioLabel: `${ratio.toFixed(1)}:1`,
    attackLabel: formatMs(preset.attackMs),
    releaseLabel: formatMs(preset.releaseMs),
  };

  return {
    low: values,
    mid: values,
    high: values,
  };
}
