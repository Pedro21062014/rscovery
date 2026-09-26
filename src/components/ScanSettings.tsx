import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

interface ScanSettingsProps {
  outputDir: string;
  setOutputDir: (dir: string) => void;
  /** When provided, shows the "filter thumbnails" toggle (images page only). */
  filterThumbs?: boolean;
  setFilterThumbs?: (v: boolean) => void;
}

const BetaTag = () => (
  <span
    style={{
      background: "#ffb020",
      color: "#1d1d1d",
      fontSize: 10,
      fontWeight: 700,
      letterSpacing: 1,
      padding: "2px 8px",
      borderRadius: 999,
      marginLeft: 8,
      verticalAlign: "middle",
    }}
  >
    BETA
  </span>
);

export default function ScanSettings({
  outputDir,
  setOutputDir,
  filterThumbs,
  setFilterThumbs,
}: ScanSettingsProps) {
  const [picking, setPicking] = useState(false);

  const pickFolder = async () => {
    setPicking(true);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Choose where to save the recovered files",
      });
      if (typeof selected === "string" && selected.length > 0) {
        setOutputDir(selected);
      }
    } catch (err) {
      // Native dialog unavailable (e.g. no zenity/kdialog): the user can
      // still type/paste the path manually in the field.
      console.error(err);
    } finally {
      setPicking(false);
    }
  };

  return (
    <div
      style={{
        background: "rgb(62, 62, 62)",
        border: "1px solid rgb(108, 108, 108)",
        borderRadius: 12,
        padding: "16px 20px",
        maxWidth: 520,
        margin: "20px auto",
        textAlign: "left",
      }}
    >
      <div style={{ fontWeight: 600, fontSize: 15, marginBottom: 12 }}>
        ⚙️ Scan settings
        <BetaTag />
      </div>

      <label style={{ fontSize: 13, color: "rgb(200, 200, 200)" }}>
        📁 Save recovered files to:
      </label>
      <div style={{ display: "flex", gap: 8, marginTop: 6 }}>
        <input
          type="text"
          value={outputDir}
          placeholder="Default: 'found' folder next to the app"
          onChange={(e) => setOutputDir(e.target.value)}
          style={{
            flex: 1,
            background: "rgb(50, 50, 50)",
            border: "1px solid rgb(108, 108, 108)",
            borderRadius: 6,
            padding: "8px 10px",
            color: "white",
            fontSize: 13,
          }}
        />
        <button onClick={pickFolder} disabled={picking} style={{ fontSize: 13 }}>
          {picking ? "..." : "Browse"}
        </button>
      </div>

      {setFilterThumbs !== undefined && (
        <label
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 8,
            marginTop: 14,
            fontSize: 13,
            color: "rgb(220, 220, 220)",
            cursor: "pointer",
            lineHeight: 1.4,
          }}
        >
          <input
            type="checkbox"
            checked={filterThumbs ?? false}
            onChange={(e) => setFilterThumbs(e.target.checked)}
            style={{ marginTop: 2, width: "auto" }}
          />
          <span>
            🖼️ <strong>Filter thumbnails</strong> — skip small images
            (&lt; 256px or &lt; 10 KB) and recover only real photos.
          </span>
        </label>
      )}

      <div style={{ marginTop: 14, textAlign: "right" }}>
        <button
          onClick={() => {
            setOutputDir("");
            if (setFilterThumbs !== undefined) setFilterThumbs(false);
            localStorage.removeItem("rscovery:outputDir");
            localStorage.removeItem("rscovery:filterThumbs");
          }}
          style={{
            fontSize: 12,
            padding: "6px 12px",
            background: "transparent",
            border: "1px solid rgb(108, 108, 108)",
            color: "rgb(200, 200, 200)",
            borderRadius: 6,
          }}
        >
          ↺ Reset to defaults
        </button>
      </div>
    </div>
  );
}
