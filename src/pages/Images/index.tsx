import { Link, useLocation } from "react-router-dom";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ScanSettings from "../../components/ScanSettings";

interface ImagePayload {
  base64: string; // small thumbnail (the full file is saved on disk)
  path: string;
  size: number; // KB
}

// How many previews to render at once (the list keeps growing in memory
// with tiny thumbnails, but the DOM stays light).
const MAX_RENDERED = 200;

export default function Images() {
  const { search } = useLocation();
  const queryParams = new URLSearchParams(search);

  const id = queryParams.get("id");
  const type = queryParams.get("type") as "png" | "jpeg";

  const [loadingScan, setLoadingScan] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [images, setImages] = useState<ImagePayload[]>([]);

  const [progress, setProgress] = useState(0);
  const [total, setTotal] = useState(0);

  // Scan settings (beta) — persisted between sessions.
  const [outputDir, setOutputDir] = useState(
    () => localStorage.getItem("rscovery:outputDir") ?? ""
  );
  const [filterThumbs, setFilterThumbs] = useState(
    () => localStorage.getItem("rscovery:filterThumbs") === "1"
  );

  useEffect(() => {
    localStorage.setItem("rscovery:outputDir", outputDir);
  }, [outputDir]);

  useEffect(() => {
    localStorage.setItem("rscovery:filterThumbs", filterThumbs ? "1" : "0");
  }, [filterThumbs]);

  useEffect(() => {
    const unlistenFound = listen("file-found", (event) => {
      const payload = event.payload as ImagePayload;
      setImages((prev) => [...prev, payload]);
    });

    const unlistenProgress = listen("file-progress", (event) => {
      const progress = event.payload as { current: number; total: number };
      setProgress(progress.current);
      setTotal(progress.total);
    });

    return () => {
      unlistenFound.then((f) => f());
      unlistenProgress.then((f) => f());
    };
  }, []);

  const handleStartScan = async () => {
    setError(null);
    setLoadingScan(true);
    try {
      const invokeName = type === "jpeg" ? "find_jpeg" : "find_png";
      await invoke(invokeName, {
        path: id,
        outputDir: outputDir.trim() || null,
        minDim: filterThumbs ? 256 : null,
      });
    } catch (err) {
      console.error("Error starting scan:", err);
      setError(String(err));
      setLoadingScan(false);
    }
  };

  return (
    <main className="container">
      <header>
        <div>
          <Link to={`/disk?id=${id}`}>Go Back</Link>
        </div>
        <h1>
          📷 Disk "{id}" ({type})
        </h1>
      </header>

      {error && (
        <div
          style={{
            background: "#3a1212",
            border: "1px solid #ff5252",
            padding: "16px",
            borderRadius: "8px",
            marginTop: "16px",
            whiteSpace: "pre-wrap",
          }}
        >
          ❌ {error}
        </div>
      )}

      {loadingScan ? (
        <div>
          <p>
            {(progress / 1024).toFixed(2)}/{(total / 1024).toFixed(2)} GB (
            {images.length} found)
          </p>

          {images.length > 0 && (
            <p style={{ fontSize: 13, color: "rgb(180, 180, 180)" }}>
              Full-size images are saved on disk — previews below are
              thumbnails.
            </p>
          )}

          <div className="imageGrid">
            {images.slice(0, MAX_RENDERED).map((img, index) => (
              <div key={index} title={img.path} style={{ display: "block", padding: 4 }}>
                <img
                  src={`data:image/jpeg;base64,${img.base64}`}
                  alt={`Recovered ${index}`}
                  className="recoveredImage"
                />
              </div>
            ))}
          </div>

          {images.length > MAX_RENDERED && (
            <p style={{ fontSize: 13, color: "rgb(180, 180, 180)" }}>
              Showing the first {MAX_RENDERED} previews of {images.length}{" "}
              recovered images (all of them are saved on disk).
            </p>
          )}

          {images.length > 0 && (
            <p
              style={{
                fontSize: 13,
                color: "rgb(180, 180, 180)",
                wordBreak: "break-all",
              }}
            >
              📁 Saved to: {images[0].path}
            </p>
          )}
        </div>
      ) : (
        <div style={{ marginTop: "24px", textAlign: "center" }}>
          <ScanSettings
            outputDir={outputDir}
            setOutputDir={setOutputDir}
            filterThumbs={filterThumbs}
            setFilterThumbs={setFilterThumbs}
          />
          <button onClick={handleStartScan}>Start Scan</button>
        </div>
      )}
    </main>
  );
}
