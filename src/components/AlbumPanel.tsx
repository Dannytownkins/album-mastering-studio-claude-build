// Phase B Step 4: Album Master panel.
//
// Top-strip control surface for album-mode mastering. Shows:
//   * Arc dropdown (4 named curves)
//   * Album intensity slider
//   * Album title input
//   * Track lane — position number, title, role, arc-offset hint
//   * Export Album CTA (calls plan_album → render_album_plan via the hook)
//   * Last export report when present
//
// Per-track DSP is still edited via the regular Tone Shape / Macros / Advanced
// controls on whichever track the user has selected from the sidebar. The
// album layer only modulates the per-track LUFS target via arc + character.

import type { AlbumArcKind, ImportedTrack, TrackId, TrackRole } from "../bindings";
import { ALBUM_ARC_DISPLAY } from "../bindings";
import type { AlbumRenderReport } from "../lib/api";

type AlbumPanelProps = {
  tracks: ImportedTrack[];
  selectedTrackId: TrackId | null;
  onSelectTrack: (id: TrackId) => void;
  albumArcKind: AlbumArcKind;
  albumIntensity: number;
  albumTitle: string;
  albumRendering: boolean;
  albumExportReport: AlbumRenderReport | null;
  onAlbumArc: (kind: AlbumArcKind) => void;
  onAlbumIntensity: (v: number) => void;
  onAlbumTitle: (v: string) => void;
  onExportAlbum: () => void;
};

const ROLE_LABEL: Record<TrackRole, string> = {
  opener: "Opener",
  closer: "Closer",
  single: "Single",
  ballad: "Ballad",
  interlude: "Interlude",
  album_track: "Album",
};

function inferDisplayRole(index: number, total: number): TrackRole {
  if (total === 0) return "album_track";
  if (index === 0) return "opener";
  if (index === total - 1) return "closer";
  return "album_track";
}

export function AlbumPanel({
  tracks,
  selectedTrackId,
  onSelectTrack,
  albumArcKind,
  albumIntensity,
  albumTitle,
  albumRendering,
  albumExportReport,
  onAlbumArc,
  onAlbumIntensity,
  onAlbumTitle,
  onExportAlbum,
}: AlbumPanelProps) {
  const arcKinds: AlbumArcKind[] = [
    "cinematic",
    "afterhours",
    "club-peak",
    "fever-dream",
  ];
  return (
    <section className="album-panel">
      <header className="album-panel-head">
        <span className="section-label">Album Master</span>
        <input
          type="text"
          className="album-title-input"
          value={albumTitle}
          placeholder="Album title…"
          onChange={(e) => onAlbumTitle(e.target.value)}
          maxLength={120}
        />
      </header>
      <div className="album-panel-controls">
        <label className="adv-label" htmlFor="album-arc-select">
          Arc
        </label>
        <select
          id="album-arc-select"
          className="loudness-profile-select"
          value={albumArcKind}
          onChange={(e) => onAlbumArc(e.target.value as AlbumArcKind)}
        >
          {arcKinds.map((k) => (
            <option key={k} value={k}>
              {ALBUM_ARC_DISPLAY[k]}
            </option>
          ))}
        </select>
        <label className="adv-label" htmlFor="album-intensity-range">
          Intensity
        </label>
        <input
          id="album-intensity-range"
          type="range"
          min={0}
          max={2}
          step={0.05}
          value={albumIntensity}
          onChange={(e) => onAlbumIntensity(parseFloat(e.target.value))}
          className="album-intensity-range"
        />
        <span className="album-intensity-value">
          ×{albumIntensity.toFixed(2)}
        </span>
        <button
          type="button"
          className="primary album-export-btn"
          onClick={onExportAlbum}
          disabled={albumRendering || tracks.length === 0}
        >
          {albumRendering ? "Rendering album…" : "Export Album"}
        </button>
      </div>
      <ol className="album-track-lane">
        {tracks.map((t, i) => {
          const role = inferDisplayRole(i, tracks.length);
          const active = t.id === selectedTrackId;
          return (
            <li
              key={t.id}
              className={"album-track-tile " + (active ? "is-active" : "")}
            >
              <button
                type="button"
                onClick={() => onSelectTrack(t.id)}
                className="album-track-tile-btn"
              >
                <span className="album-track-position">
                  {String(i + 1).padStart(2, "0")}
                </span>
                <span className="album-track-title">{t.display_name}</span>
                <span className={"album-track-role role-" + role}>
                  {ROLE_LABEL[role]}
                </span>
              </button>
            </li>
          );
        })}
      </ol>
      {albumExportReport && (
        <div className="album-export-receipt">
          <span className="album-export-receipt-label">Last export:</span>
          <code className="album-export-receipt-path">
            {albumExportReport.album_wav_path}
          </code>
          <span className="album-export-receipt-meta">
            {albumExportReport.tracks.length} tracks · manifest:{" "}
            {albumExportReport.manifest_path}
          </span>
        </div>
      )}
    </section>
  );
}
