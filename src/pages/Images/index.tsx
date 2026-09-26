import { Link, useLocation } from "react-router-dom";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ImagePayload {
  iteration: number;
  base64: string;
}

export default function Images() {
  const { search } = useLocation();
  const queryParams = new URLSearchParams(search);

  const id = queryParams.get("id");
  const type = queryParams.get("type") as "png" | "jpeg";

  const [loadingScan, setLoadingScan] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [images, setImages] = useState<string[]>([]);

  const [progress, setProgress] = useState(0);
  const [total, setTotal] = useState(0);

  useEffect(() => {
    const unlistenFound = listen("file-found", (event) => {
      const progress = event.payload as ImagePayload;
      console.log(progress);
      setImages((prev) => [...prev, progress.base64]);
    });

    const unlistenProgress = listen("file-progress", (event) => {
      const progress = event.payload as { current: number; total: number };
      console.log(progress);
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
      await invoke(invokeName, { path: id });
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
            {(progress / 1024).toFixed(2)}/{(total / 1024).toFixed(2)} (
            {images.length})
          </p>
          <div className="imageGrid">
            {images.map((img, index) => (
              <img
                key={index}
                src={`data:image/jpeg;base64,${img}`}
                alt={`Recovered ${index}`}
                className="recoveredImage"
              />
            ))}
          </div>
        </div>
      ) : (
        <div style={{ marginTop: "24px", textAlign: "center" }}>
          <button onClick={handleStartScan}>Start Scan</button>
        </div>
      )}
    </main>
  );
}
