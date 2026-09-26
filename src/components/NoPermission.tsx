import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type OSTab = "linux-appimage" | "linux-deb" | "windows" | "macos";

const TABS: { id: OSTab; label: string }[] = [
  { id: "linux-appimage", label: "🐧 Linux (AppImage)" },
  { id: "linux-deb", label: "🐧 Linux (.deb)" },
  { id: "windows", label: "🪟 Windows" },
  { id: "macos", label: "🍎 macOS" },
];

const STEPS: Record<OSTab, { text: string; cmd?: string }[]> = {
  "linux-appimage": [
    { text: "Close Rscovery completely." },
    { text: "Open a terminal in the folder where you downloaded the AppImage." },
    { text: "Make the file executable:", cmd: "chmod +x rscovery_*_amd64.AppImage" },
    { text: "Run the app as root:", cmd: "sudo ./rscovery_*_amd64.AppImage" },
    {
      text: "Type your user password and press Enter (nothing is shown while typing — that's normal).",
    },
  ],
  "linux-deb": [
    { text: "Close Rscovery completely." },
    { text: "Open a terminal." },
    { text: "Run the app as root:", cmd: "sudo rscovery" },
    {
      text: "Type your user password and press Enter (nothing is shown while typing — that's normal).",
    },
  ],
  windows: [
    { text: "Close Rscovery completely." },
    { text: 'Open the Start menu and search for "Rscovery".' },
    { text: "Right-click the app and choose “Run as administrator”." },
    { text: "Click “Yes” in the Windows permission dialog (UAC)." },
  ],
  macos: [
    { text: "Close Rscovery completely." },
    { text: 'Open the Terminal app (press ⌘ + Space and type "Terminal").' },
    {
      text: "Run the app as root:",
      cmd: "sudo /Applications/rscovery.app/Contents/MacOS/rscovery",
    },
    { text: "Type your user password and press Enter." },
  ],
};

function detectOS(): OSTab {
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac/i.test(ua)) return "macos";
  return "linux-appimage";
}

function Command({ cmd }: { cmd: string }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(cmd);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard not available in this context
    }
  };

  return (
    <div
      style={{
        display: "flex",
        gap: 8,
        alignItems: "center",
        marginTop: 8,
        flexWrap: "wrap",
      }}
    >
      <code
        style={{
          background: "#1d1d1d",
          border: "1px solid rgb(108, 108, 108)",
          borderRadius: 6,
          padding: "8px 12px",
          fontSize: 14,
          color: "#7ee787",
          userSelect: "all",
          flex: 1,
          minWidth: 240,
          textAlign: "left",
        }}
      >
        {cmd}
      </code>
      <button onClick={copy} style={{ fontSize: 13, padding: "8px 14px" }}>
        {copied ? "✓ Copied!" : "Copy"}
      </button>
    </div>
  );
}

export default function NoPermission() {
  const [tab, setTab] = useState<OSTab>(detectOS);
  const [checking, setChecking] = useState(false);

  const recheck = async () => {
    setChecking(true);
    try {
      const ok = await invoke<boolean>("check_root");
      if (ok) {
        window.location.reload();
      }
    } catch (err) {
      console.error(err);
    } finally {
      setChecking(false);
    }
  };

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "#2f2f2f",
        zIndex: 9999,
        overflowY: "auto",
        display: "flex",
        justifyContent: "center",
        padding: "40px 16px",
      }}
    >
      <div style={{ maxWidth: 660, width: "100%" }}>
        <h1 style={{ fontSize: "1.9em" }}>🔒 Administrator privileges required</h1>

        <p style={{ color: "rgb(200, 200, 200)", lineHeight: 1.6 }}>
          Rscovery reads disks <strong>directly (raw disk)</strong> to recover
          files, so it must run as <strong>administrator/root</strong>. Follow
          the steps below for your system, then open the app again.
        </p>

        {/* OS selector */}
        <div
          style={{
            display: "flex",
            gap: 8,
            flexWrap: "wrap",
            justifyContent: "center",
            margin: "20px 0",
          }}
        >
          {TABS.map(({ id, label }) => (
            <button
              key={id}
              onClick={() => setTab(id)}
              style={{
                fontSize: 14,
                padding: "8px 14px",
                borderColor: tab === id ? "#396cd8" : "transparent",
                backgroundColor: tab === id ? "#1d3a6d" : "#0f0f0f98",
              }}
            >
              {label}
            </button>
          ))}
        </div>

        {/* Steps */}
        <div
          style={{
            background: "rgb(62, 62, 62)",
            borderRadius: 12,
            padding: "20px 24px",
            textAlign: "left",
          }}
        >
          <ol style={{ margin: 0, paddingLeft: 24, display: "flex", flexDirection: "column", gap: 14 }}>
            {STEPS[tab].map((step, i) => (
              <li key={i} style={{ color: "#f6f6f6", lineHeight: 1.5 }}>
                {step.text}
                {step.cmd && <Command cmd={step.cmd} />}
              </li>
            ))}
          </ol>
        </div>

        {/* Recheck */}
        <div style={{ marginTop: 24 }}>
          <button onClick={recheck} disabled={checking} style={{ fontSize: 15 }}>
            {checking ? "Checking..." : "✅ I already reopened with admin — check again"}
          </button>
          <p style={{ color: "rgb(160, 160, 160)", fontSize: 13, marginTop: 12 }}>
            Tip: the app needs to be <strong>closed and reopened</strong> with
            elevated permissions — this window cannot gain admin rights by itself.
          </p>
        </div>
      </div>
    </div>
  );
}
