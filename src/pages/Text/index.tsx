import { Link, useLocation } from "react-router-dom";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export default function Text() {
  const { search } = useLocation();
  const queryParams = new URLSearchParams(search);

  const id = queryParams.get("id");

  const [loadingScan, setLoadingScan] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [texts, setTexts] = useState<string[]>([]);

  const [progress, setProgress] = useState(0);
  const [total, setTotal] = useState(0);

  const [wordlist, setWordlist] = useState("");
  const [blacklist, setBlacklist] = useState("");

  useEffect(() => {
    const unlistenFound = listen("text-found", (event) => {
      const progress = event.payload as {text: string};
      setTexts((prev) => [...prev, progress.text]);
    });

    const unlistedProgress = listen("file-progress", (event) => {
      const progress = event.payload as {current: number, total: number};
      console.log(progress)
      setProgress(progress.current);
      setTotal(progress.total);
    });

    return () => {
      unlistenFound.then((f) => f());
      unlistedProgress.then((f) => f());
      // Leaving the page stops any scan in progress.
      invoke("stop_scan").catch(() => {});
    };
  }, []);

  const handleStartScan = async () => {
    setError(null);
    // Fresh scan: reset progress/results.
    setTexts([]);
    setProgress(0);
    setTotal(0);
    setLoadingScan(true);
    try {
      const invokeName = "find_txt";

      const blacklistParsed = blacklist.split(",").map((a) => a.trim());
      const blacklistEmpty =
        blacklistParsed.length === 1 && blacklistParsed[0] === "";

      await invoke(invokeName, {
        path: id,
        wordlist: wordlist.split(",").map((a) => a.trim()),
        blacklist: blacklistEmpty ? [] : blacklist.split(",").map((a) => a.trim()),
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
        <h1>✏️ Disk "{id}" (txt)</h1>
      </header>

      <div className="inputGroup">
        <div>
          <label>Wordlist</label>
          <textarea
            placeholder="Ex: password, email, gmail, ..."
            value={wordlist}
            onChange={(e) => setWordlist(e.target.value)}
          ></textarea>
        </div>
        <div>
          <label>Blacklist</label>
          <textarea
            placeholder="Ex: adobe, UUUUU, ..."
            value={blacklist}
            onChange={(e) => setBlacklist(e.target.value)}
          ></textarea>
        </div>
      </div>

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
            {texts.length})
          </p>
          <div className="textGrid">
            {texts.map((text, index) => (
              <div key={index}>
                <p>{text}</p>
              </div>
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
