import { describe, expect, it } from "vitest";

import type {
  AdvancedSettings,
  DeliveryProfile,
  MasteringSettings,
  Preset,
} from "../bindings";
import { compressorAutoReadouts } from "./compressor-auto";

const DEFAULT_ADVANCED: AdvancedSettings = {
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
};

function makeSettings(
  preset: Preset,
  advanced: Partial<AdvancedSettings> = {},
  deliveryProfile: DeliveryProfile = "streaming-universal",
): MasteringSettings {
  return {
    preset,
    intensity: 0.5,
    eq_sub_db: 0,
    eq_low_db: 0,
    eq_low_mid_db: 0,
    eq_mid_db: 0,
    eq_high_mid_db: 0,
    eq_high_db: 0,
    eq_sparkle_db: 0,
    volume_match: false,
    input_gain_db: 0,
    output_gain_db: 0,
    delivery_profile: deliveryProfile,
    advanced: { ...DEFAULT_ADVANCED, ...advanced },
  };
}

describe("compressorAutoReadouts", () => {
  it("shows Universal's default auto compressor values with units", () => {
    const auto = compressorAutoReadouts(makeSettings({ kind: "universal" }));

    expect(auto.low).toMatchObject({
      thresholdLabel: "-16.0 dB",
      ratioLabel: "1.8:1",
      attackLabel: "15 ms",
      releaseLabel: "250 ms",
    });
    expect(auto.mid).toEqual(auto.low);
    expect(auto.high).toEqual(auto.low);
  });

  it("tracks the density macro for the Loud preset", () => {
    const auto = compressorAutoReadouts(
      makeSettings({ kind: "loud" }, { compression_density: 1 }),
    );

    expect(auto.low).toMatchObject({
      thresholdLabel: "-26.0 dB",
      ratioLabel: "4.0:1",
      attackLabel: "15 ms",
      releaseLabel: "180 ms",
    });
  });

  it("shows Custom as identity until the density macro is raised", () => {
    const auto = compressorAutoReadouts(
      makeSettings({ kind: "custom", id: "custom" }, {}, "custom"),
    );

    expect(auto.low).toMatchObject({
      thresholdLabel: "0.0 dB",
      ratioLabel: "1.0:1",
      attackLabel: "15 ms",
      releaseLabel: "200 ms",
    });
  });
});
