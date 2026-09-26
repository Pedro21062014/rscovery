import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "react-router-dom";

type DiskInfo = {
  name: string;
  size: number;
};

export default function Main() {
  const [disks, setDisks] = useState<DiskInfo[]>([]);
  const [isRoot, setIsRoot] = useState<boolean | null>(null);

  useEffect(() => {
    const fetchDisks = async () => {
      try {
        setDisks(await invoke<DiskInfo[]>("list_disks"));
      } catch (err) {
        console.log(err);
      }
    };

    const checkPermissions = async () => {
      try {
        setIsRoot(await invoke<boolean>("check_root"));
      } catch (err) {
        console.log(err);
      }
    };

    fetchDisks();
    checkPermissions();
  }, []);

  return (
    <main className="container">
      <h1>Rscovery 🦀</h1>
      <p>Select the device you want to recover.</p>

      {isRoot === false && (
        <div
          className="warningContainer"
          style={{
            background: "#3a2a12",
            border: "1px solid #ffb020",
            padding: "16px",
            borderRadius: "8px",
            lineHeight: 1.6,
          }}
        >
          ⚠️ <strong>Running without elevated permissions.</strong>
          <br />
          Rscovery reads disks directly and needs root/administrator access to
          scan. Close the app and reopen it elevated:
          <br />
          🐧 Linux: <code>sudo /path/to/rscovery</code> (AppImage:{" "}
          <code>sudo ./rscovery_x.y.z_amd64.AppImage</code>)
          <br />
          🪟 Windows: right-click → <em>Run as administrator</em>
        </div>
      )}

      {disks.length == 0 && (
        <div className="warningContainer">No devices found...</div>
      )}

      <div className="deviceList">
        {disks.map(({ name, size }) => (
          <Link
            to={`/disk?id=${name}`}
            key={name}
            style={{ textDecoration: "none", color: "inherit" }}
          >
            <div className="device">
              <h1>{name}</h1>
              <p>{(size / 1024).toFixed(2)} GB</p>
            </div>
          </Link>
        ))}
      </div>
    </main>
  );
}
