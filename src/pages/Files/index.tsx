import { Link, useLocation } from "react-router-dom";
import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import ScanSettings from "../../components/ScanSettings";

interface FilePayload {
  size: number,
  path: string
}

const PDFIcon = () => (
  <svg style={{height: 48, width: 48}} xmlns="http://www.w3.org/2000/svg" width="75.320129mm" height="92.604164mm" viewBox="0 0 75.320129 92.604164">
    <g transform="translate(53.548057 -183.975276) scale(1.4843)">
      <path fill="#ff2116" d="M-29.632812 123.94727c-3.551967 0-6.44336 2.89347-6.44336 6.44531v49.49804c0 3.55185 2.891393 6.44532 6.44336 6.44532H8.2167969c3.5519661 0 6.4433591-2.89335 6.4433591-6.44532v-40.70117s.101353-1.19181-.416015-2.35156c-.484969-1.08711-1.275391-1.84375-1.275391-1.84375a1.0584391 1.0584391 0 0 0-.0059-.008l-9.3906254-9.21094a1.0584391 1.0584391 0 0 0-.015625-.0156s-.8017392-.76344-1.9902344-1.27344c-1.39939552-.6005-2.8417968-.53711-2.8417968-.53711l.021484-.002z" color="#000" fontFamily="sans-serif" overflow="visible" paintOrder="markers fill stroke" />
      <path fill="#f5f5f5" d="M-29.632812 126.06445h28.3789058a1.0584391 1.0584391 0 0 0 .021484 0s1.13480448.011 1.96484378.36719c.79889772.34282 1.36536982.86176 1.36914062.86524.0000125.00001.00391.004.00391.004l9.3671868 9.18945s.564354.59582.837891 1.20899c.220779.49491.234375 1.40039.234375 1.40039a1.0584391 1.0584391 0 0 0-.002.0449v40.74609c0 2.41592-1.910258 4.32813-4.3261717 4.32813H-29.632812c-2.415914 0-4.326172-1.91209-4.326172-4.32813v-49.49804c0-2.41603 1.910258-4.32813 4.326172-4.32813z" color="#000" fontFamily="sans-serif" overflow="visible" paintOrder="markers fill stroke"/>
      <path fill="#ff2116" d="M-23.40766 161.09299c-1.45669-1.45669.11934-3.45839 4.39648-5.58397l2.69124-1.33743 1.04845-2.29399c.57665-1.26169 1.43729-3.32036 1.91254-4.5748l.8641-2.28082-.59546-1.68793c-.73217-2.07547-.99326-5.19438-.52872-6.31588.62923-1.51909 2.69029-1.36323 3.50626.26515.63727 1.27176.57212 3.57488-.18329 6.47946l-.6193 2.38125.5455.92604c.30003.50932 1.1764 1.71867 1.9475 2.68743l1.44924 1.80272 1.8033728-.23533c5.72900399-.74758 7.6912472.523 7.6912472 2.34476 0 2.29921-4.4984914 2.48899-8.2760865-.16423-.8499666-.59698-1.4336605-1.19001-1.4336605-1.19001s-2.3665326.48178-3.531704.79583c-1.202707.32417-1.80274.52719-3.564509 1.12186 0 0-.61814.89767-1.02094 1.55026-1.49858 2.4279-3.24833 4.43998-4.49793 5.1723-1.3991.81993-2.86584.87582-3.60433.13733zm2.28605-.81668c.81883-.50607 2.47616-2.46625 3.62341-4.28553l.46449-.73658-2.11497 1.06339c-3.26655 1.64239-4.76093 3.19033-3.98386 4.12664.43653.52598.95874.48237 2.01093-.16792zm21.21809-5.95578c.80089-.56097.68463-1.69142-.22082-2.1472-.70466-.35471-1.2726074-.42759-3.1031574-.40057-1.1249.0767-2.9337647.3034-3.2403347.37237 0 0 .993716.68678 1.434896.93922.58731.33544 2.0145161.95811 3.0565161 1.27706 1.02785.31461 1.6224.28144 2.0729-.0409zm-8.53152-3.54594c-.4847-.50952-1.30889-1.57296-1.83152-2.3632-.68353-.89643-1.02629-1.52887-1.02629-1.52887s-.4996 1.60694-.90948 2.57394l-1.27876 3.16076-.37075.71695s1.971043-.64627 2.97389-.90822c1.0621668-.27744 3.21787-.70134 3.21787-.70134zm-2.74938-11.02573c.12363-1.0375.1761-2.07346-.15724-2.59587-.9246-1.01077-2.04057-.16787-1.85154 2.23517.0636.8084.26443 2.19033.53292 3.04209l.48817 1.54863.34358-1.16638c.18897-.64151.47882-2.02015.64411-3.06364z"/>
      <path fill="#2c2c2c" d="M-20.930423 167.83862h2.364986q1.133514 0 1.840213.2169.706698.20991 1.189489.9446.482795.72769.482795 1.75625 0 .94459-.391832 1.6233-.391833.67871-1.056548.97958-.65772.30087-2.02913.30087h-.818651v3.72941h-1.581322zm1.581322 1.22447v3.33058h.783664q1.049552 0 1.44838-.39184.405826-.39183.405826-1.27345 0-.65772-.265887-1.06355-.265884-.41282-.587747-.50378-.314866-.098-1.000572-.098zm5.50664-1.22447h2.148082q1.560333 0 2.4909318.55276.9375993.55276 1.4133973 1.6443.482791 1.09153.482791 2.42096 0 1.3994-.4338151 2.49793-.4268149 1.09153-1.3154348 1.76324-.8816233.67172-2.5189212.67172h-2.267031zm1.581326 1.26645v7.018h.657715q1.378411 0 2.001144-.9516.6227329-.95858.6227329-2.5539 0-3.5125-2.6238769-3.5125zm6.4722254-1.26645h5.30372941v1.26645H-4.2075842v2.85478h2.9807225v1.26646h-2.9807225v4.16322h-1.5813254z" fontFamily="Franklin Gothic Medium Cond" letterSpacing="0" wordSpacing="4.26000023"/>
    </g>
  </svg>
)

const WinrarIcon = () => (
<svg xmlns="http://www.w3.org/2000/svg"  viewBox="0 0 48 48" width="48px" height="48px"><path fill="#CFD8DC" d="M17 24L7 17.467c0 0 1-1.865 1-3.732S7 10 7 10l10 6.533V24zM17 40L7 32.533c0 0 1-.934 1-2.8S7 26 7 26l10 6.533V40zM17 32L7 24.533c0 0 1-.934 1-2.8S7 18 7 18l10 6.533V32z"/><path fill="#3F51B5" d="M17.624,34c-0.219,0-0.44-0.071-0.624-0.219l-10.625-8c-0.432-0.346-0.501-0.975-0.156-1.406c0.344-0.431,0.975-0.501,1.405-0.156l10.625,8c0.432,0.346,0.501,0.975,0.156,1.406C18.208,33.871,17.917,34,17.624,34z"/><path fill="#3F51B5" d="M42,25H16c0,0,1,1.742,1,4.246S16,33,16,33h26c0,0,1-1,1-4S42,25,42,25z"/><path fill="#9C27B0" d="M17.624,26c-0.219,0-0.44-0.071-0.624-0.219l-10.625-8c-0.432-0.346-0.501-0.975-0.156-1.406c0.344-0.431,0.975-0.502,1.405-0.156l10.625,8c0.432,0.346,0.501,0.975,0.156,1.406C18.208,25.871,17.917,26,17.624,26z"/><path fill="#9C27B0" d="M42,17H16c0,0,1,1.742,1,4.246S16,25,16,25h26c0,0,1-1,1-4S42,17,42,17z"/><path fill="#8BC34A" d="M18.609,41c0-0.293-0.113-0.584-0.36-0.781l-10.625-8c-0.43-0.345-1.061-0.274-1.405,0.156c-0.345,0.432-0.275,1.061,0.156,1.406L15.962,41H18.609z"/><path fill="#8BC34A" d="M42,33H16c0,0,1,1.742,1,4.246S16,41,16,41h26c0,0,1-1,1-4S42,33,42,33z"/><path fill="#689F38" d="M42,33H16c0,0,0.441,0.756,0.733,2h26.035C42.473,33.559,42,33,42,33z"/><path fill="#303F9F" d="M42,25H16c0,0,0.441,0.756,0.733,2h26.035C42.473,25.559,42,25,42,25z"/><path fill="#FDD835" d="M21.034 32c.11-.45.299-1.379.299-2.5s-.189-2.05-.299-2.5H19c.003.009.333 1.267.333 2.5S19.002 31.993 19 32H21.034zM21.034 24c.11-.45.299-1.379.299-2.5s-.189-2.05-.299-2.5H19c.003.009.333 1.267.333 2.5S19.002 23.993 19 24H21.034zM21.034 40c.11-.45.299-1.379.299-2.5s-.189-2.05-.299-2.5H19c.003.009.333 1.267.333 2.5S19.002 39.993 19 40H21.034z"/><path fill="#7B1FA2" d="M42.768,19C42.473,17.559,42,17,42,17L30.844,8.591C30.844,8.591,30.063,8,29,8C27.771,8,7,8,7,8l0.018,0.018C6.703,8.012,6.39,8.137,6.191,8.412c-0.325,0.446-0.226,1.072,0.22,1.396l9.737,7.47c0.161,0.325,0.418,0.92,0.607,1.722H42.768z"/><path fill="#AF7000" d="M32,16L21,8h-5l11,8c1.75,1.25,2,1.625,2,3s0,19,0,19s0,3-2,3c1.056,0,3.678,0,5,0c2,0,2-3,2-3s0-18,0-19C34,17,32.438,16.438,32,16z"/><g><path fill="#FFC107" d="M34,27v4h-5v-4H34 M34.25,26h-5.5C28.336,26,28,26.336,28,26.75v4.5c0,0.414,0.336,0.75,0.75,0.75h5.5c0.414,0,0.75-0.336,0.75-0.75v-4.5C35,26.336,34.664,26,34.25,26L34.25,26z"/></g><path fill="#5B3B07" d="M31.5 28.5A0.5 0.5 0 1 0 31.5 29.5A0.5 0.5 0 1 0 31.5 28.5Z"/><path fill="#FFEB3B" d="M31.5,29c-0.276,0-0.5-0.224-0.5-0.5v-3c0-0.276,0.224-0.5,0.5-0.5s0.5,0.224,0.5,0.5v3C32,28.776,31.776,29,31.5,29z"/></svg>
)

export default function Files() {
  const { search } = useLocation();
  const queryParams = new URLSearchParams(search);

  const id = queryParams.get("id");
  const type = queryParams.get("type") as "pdf" | "zip" | "mp4";

  const [loadingScan, setLoadingScan] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [files, setFiles] = useState<FilePayload[]>([]);

  const [progress, setProgress] = useState(0);
  const [total, setTotal] = useState(0);

  // Scan settings (beta) — persisted between sessions.
  const [outputDir, setOutputDir] = useState(
    () => localStorage.getItem("rscovery:outputDir") ?? ""
  );

  useEffect(() => {
    localStorage.setItem("rscovery:outputDir", outputDir);
  }, [outputDir]);

  // Token of the scan started by THIS page (used to stop exactly that scan).
  const scanIdRef = useRef<number | null>(null);
  const unmountedRef = useRef(false);

  useEffect(() => {
    unmountedRef.current = false;

    const unlistenStarted = listen<{ id: number }>("scan-started", (event) => {
      scanIdRef.current = event.payload.id;
      // Page closed before the token arrived: stop the scan right away.
      if (unmountedRef.current) {
        invoke("stop_scan", { id: event.payload.id }).catch(() => {});
      }
    });

    const unlistenFound = listen("file-found", (event) => {
      const progress = event.payload as FilePayload;
      console.log(progress)
      setFiles((prev) => [...prev, progress]);
    });

    const unlistenProgress = listen("file-progress", (event) => {
      const progress = event.payload as {current: number, total: number};
      console.log(progress)
      setProgress(progress.current);
      setTotal(progress.total);
    });


    return () => {
      unmountedRef.current = true;
      unlistenStarted.then((f) => f());
      unlistenFound.then((f) => f());
      unlistenProgress.then((f) => f());
      // Leaving the page stops only the scan this page started.
      if (scanIdRef.current !== null) {
        invoke("stop_scan", { id: scanIdRef.current }).catch(() => {});
      }
    };
  }, []);

  const handleStartScan = async () => {
    setError(null);
    // Fresh scan: reset progress/results.
    setFiles([]);
    setProgress(0);
    setTotal(0);
    setLoadingScan(true);
    try {
      const invokeName = type === "pdf" ? "find_pdf" : type === "mp4" ? "find_mp4" : "find_zip";
      await invoke(invokeName, { path: id, outputDir: outputDir.trim() || null });
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
        <h1>📄 Disk "{id}" ({type})</h1>
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
            {(progress / 1024).toFixed(2)}/{(total / 1024).toFixed(2)} ({files.length})
        </p>
          <div className="filesGrid">
            {files.map(({path, size}, index) => (
              <div key={index}>
                {type == "pdf" ? <PDFIcon /> : type == "mp4" ? <div style={{fontSize: 48}}>📹</div> : <WinrarIcon />}
                <h1>{(size / 1024).toFixed(2)} MB</h1>
                <p>{path}</p>
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div style={{ marginTop: "24px", textAlign: "center" }}>
          <ScanSettings outputDir={outputDir} setOutputDir={setOutputDir} />
          <button onClick={handleStartScan}>Start Scan</button>
        </div>
      )}
    </main>
  );
}
