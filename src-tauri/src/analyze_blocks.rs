use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use serde::Serialize;

#[derive(Serialize, Clone)]
struct Progress {
    nonEmpty: Vec<i32>,
    total: f64,
    current: f64,
}

/// Devices like /dev/sdc1 (block devices) are special files, they represent
/// a block device (i.e. a hardware abstraction). The kernel does not store
/// their size in the filesystem metadata. That's why using `metadata().len()`
/// does not work.
/// We need to find the number of 512byte sectors to find the size of the device.
/// Returns the size in MB (kept for backwards compatibility with the frontend).
#[cfg(target_os = "linux")]
pub fn get_block_device_size_gb(device: &str) -> std::io::Result<f64> {
    let path = format!("/sys/class/block/{}/size", device.replace("/dev/", ""));
    let blocks: u64 = std::fs::read_to_string(path)?
        .trim()
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let bytes = blocks * 512;
    let mb = bytes as f64 / 1024.0 / 1024.0;
    Ok(mb)
}

/// On Windows/macOS there is no /sys/class/block, so we get the size from
/// sysinfo, matching by mount point or volume name (returns MB too).
#[cfg(not(target_os = "linux"))]
pub fn get_block_device_size_gb(device: &str) -> std::io::Result<f64> {
    use sysinfo::Disks;

    let disks = Disks::new_with_refreshed_list();

    if let Some(disk) = disks.iter().find(|d| {
        d.mount_point().to_string_lossy().as_ref() == device
            || d.name().to_string_lossy().as_ref() == device
    }) {
        return Ok(disk.total_space() as f64 / 1024.0 / 1024.0);
    }

    #[cfg(target_os = "macos")]
    {
        // A device like /dev/disk3s1s1 is not a mount point: resolve the mount
        // point through `mount` output, then match it in sysinfo.
        if let Ok(out) = std::process::Command::new("mount").output() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let mut parts = line.splitn(4, ' ');
                let dev = parts.next().unwrap_or("");
                let on = parts.next().unwrap_or("");
                let mp = parts.next().unwrap_or("");
                if on == "on" && dev == device {
                    if let Some(disk) = disks
                        .iter()
                        .find(|d| d.mount_point().to_string_lossy().as_ref() == mp)
                    {
                        return Ok(disk.total_space() as f64 / 1024.0 / 1024.0);
                    }
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("could not determine the size of '{}'", device),
    ))
}

/// Maps the user-visible disk name to the actual device path that must be
/// opened on each OS:
/// - Windows: "C:\" -> "\\.\C:" (raw volume access)
/// - macOS:   "/dev/disk3s1" -> "/dev/rdisk3s1" (raw device, much faster)
/// - Linux:   already a /dev/* path, nothing to do
pub fn normalize_device_path(path: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        let mut chars = path.chars();
        if let (Some(drive), Some(':')) = (chars.next(), chars.next()) {
            if drive.is_ascii_alphabetic() {
                return format!(r"\\.\{}:", drive.to_ascii_uppercase());
            }
        }
        path.to_string()
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(rest) = path.strip_prefix("/dev/disk") {
            return format!("/dev/rdisk{}", rest);
        }
        path.to_string()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        path.to_string()
    }
}

/// Turns an error while opening the device into a friendly message explaining
/// how to run the app with the permissions it needs.
pub fn friendly_open_error(e: &std::io::Error, path: &str) -> String {
    if e.kind() == std::io::ErrorKind::PermissionDenied {
        #[cfg(target_os = "windows")]
        {
            return format!(
                "Permission denied while opening '{}' ({}). Rscovery needs to read the disk directly: \
                 close the app and reopen it as Administrator (right-click > Run as administrator).",
                path, e
            );
        }
        #[cfg(not(target_os = "windows"))]
        {
            return format!(
                "Permission denied while opening '{}' ({}). Rscovery needs to read the disk directly, \
                 so it must run as root. Close the app and reopen it with sudo \
                 (AppImage example: sudo ./rscovery_x.y.z_amd64.AppImage).",
                path, e
            );
        }
    }
    format!("Failed to open '{}': {}", path, e)
}

/// Returns true when the app is running with elevated permissions
/// (root on Linux/macOS, Administrator on Windows).
#[tauri::command]
pub fn check_root() -> bool {
    #[cfg(unix)]
    {
        // `id -u` prints the effective UID (0 = root)
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        // `net session` only succeeds inside an elevated process
        std::process::Command::new("cmd")
            .args(["/C", "net", "session"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[tauri::command]
pub async fn analyze_blocks(app_handle: tauri::AppHandle, path: String) -> Result<(), String> {
    let flag = crate::begin_scan(&app_handle);
    // Runs on a blocking thread so the heavy read loop never starves the
    // async runtime (keeps the UI/events responsive).
    tauri::async_runtime::spawn_blocking(move || analyze_blocks_sync(app_handle, &path, flag))
        .await
        .map_err(|e| e.to_string())?
}

fn analyze_blocks_sync(
    app_handle: tauri::AppHandle,
    path: &str,
    flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let open_path = normalize_device_path(path);
    let mut file = File::open(&open_path).map_err(|e| friendly_open_error(&e, path))?;

    let total_size = get_block_device_size_gb(path).map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; 32 * 1024 * 1024];
    let mut total_read: u64 = 0;

    let mut iteration = 0;
    let mut non_empty: Vec<i32> = Vec::new();

    loop {
        // Cancelled (new scan started, user left the page or pressed stop).
        if flag.load(Ordering::Relaxed) {
            println!("Scan cancelled.");
            return Ok(());
        }

        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        total_read += bytes_read as u64;

        let valid_bytes = buffer.iter().filter(|&&b| b != 0x00).count();
        if valid_bytes > 0 {
            non_empty.push(iteration);
        }
        iteration += 1;

        let progress = Progress {
            nonEmpty: non_empty.clone(),
            total: total_size,
            current: total_read as f64 / 1024.0 / 1024.0,
        };

        println!("Progresso: {:.2} MB", &progress.current);
        if !flag.load(Ordering::Relaxed) {
            app_handle.emit("scan-progress", progress).unwrap();
        }
    }

    println!("Reading completed.");
    Ok(())
}
