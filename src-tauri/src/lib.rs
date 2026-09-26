use serde::Serialize;
use sysinfo::Disks;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};

mod analyze_blocks;
mod find_file;

#[derive(Debug, Serialize)]
pub struct DiskInfo {
    name: String,
    size: u64,
}

/// Keeps track of the currently running scan so it can be cancelled when the
/// user starts another scan, leaves the page, or asks to stop.
pub struct ScanState {
    pub cancel: Mutex<Option<Arc<AtomicBool>>>,
}

/// Cancels the previous scan (if any), registers a new one and returns its
/// cancellation flag. The scan loops check this flag while reading.
pub fn begin_scan(app_handle: &tauri::AppHandle) -> Arc<AtomicBool> {
    use tauri::Manager;

    let flag = Arc::new(AtomicBool::new(false));
    let state = app_handle.state::<ScanState>();
    let mut current = state.cancel.lock().unwrap();
    if let Some(old) = current.as_ref() {
        old.store(true, Ordering::Relaxed);
    }
    *current = Some(flag.clone());
    flag
}

#[tauri::command]
fn stop_scan(state: tauri::State<ScanState>) {
    if let Some(flag) = state.cancel.lock().unwrap().as_ref() {
        flag.store(true, Ordering::Relaxed);
    }
}


#[tauri::command]
fn list_disks() -> Vec<DiskInfo> {
    #[cfg(target_os = "linux")]
    let mounts = {
        use std::fs::File;

        let mut vec = Vec::new();
        if let Ok(f) = File::open("/proc/mounts") {

            let reader = BufReader::new(f);
            for line in reader.lines().filter_map(Result::ok) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    vec.push((parts[0].to_string(), parts[1].to_string()));
                }
            }
        }
        vec
    };

    #[cfg(target_os = "macos")]
    let mounts = {
        // macOS has no /proc/mounts; parse the output of `mount`:
        // "/dev/disk3s1s1 on / (apfs, local, journaled)"
        let mut vec = Vec::new();
        if let Ok(out) = std::process::Command::new("mount").output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let mut parts = line.splitn(4, ' ');
                let dev = parts.next().unwrap_or("").to_string();
                let on = parts.next().unwrap_or("");
                let mp = parts.next().unwrap_or("").to_string();
                if on == "on" && dev.starts_with("/dev/") && !mp.is_empty() {
                    vec.push((dev, mp));
                }
            }
        }
        vec
    };

    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_list();

    disks
        .iter()
        .map(|disk| {
            let mount = disk.mount_point().to_string_lossy().to_string();
            let size_mb = disk.total_space() / 1024 / 1024;

            // Em Linux/macOS, mapeia o ponto de montagem para o dispositivo
            // (ex.: /dev/sda1 no Linux, /dev/disk3s1s1 no macOS)
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            let device = {
                if let Some((dev, _mp)) = mounts.iter().find(|(_dev, mp)| mp == &mount) {
                    dev.clone()
                } else {
                    let mount_norm = if mount.ends_with('/') {
                        mount.trim_end_matches('/').to_string()
                    } else {
                        format!("{}/", mount)
                    };
                    if let Some((dev, _mp)) = mounts.iter().find(|(_dev, mp)| mp == &mount_norm) {
                        dev.clone()
                    } else {
                        mount.clone()
                    }
                }
            };

            // No Windows, usa o nome do volume/disco (com fallback para o ponto de montagem)
            #[cfg(target_os = "windows")]
            let device = {
                let name = disk.name().to_string_lossy().to_string();
                if name.is_empty() { mount.clone() } else { name }
            };

            DiskInfo {
                name: device,
                size: size_mb,
            }
        })
        .collect()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_disks,
            analyze_blocks::check_root,
            analyze_blocks::analyze_blocks,
            find_file::find_jpeg,
            find_file::find_png,
            find_file::find_pdf,
            find_file::find_zip,
            find_file::find_mp4,
            find_file::find_txt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
