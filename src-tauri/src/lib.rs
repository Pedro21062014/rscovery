use serde::Serialize;
use sysinfo::Disks;
#[cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};

mod analyze_blocks;
mod find_file;

#[derive(Debug, Serialize)]
pub struct DiskInfo {
    name: String,
    size: u64,
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

    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh_list();

    disks
        .iter()
        .map(|disk| {
            let mount = disk.mount_point().to_string_lossy().to_string();
            let size_mb = disk.total_space() / 1024 / 1024;

            // Em Linux, mapeia o ponto de montagem para o dispositivo (ex.: /dev/sda1)
            #[cfg(target_os = "linux")]
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

            // Em Windows/macOS, usa o nome do volume/disco (com fallback para o ponto de montagem)
            #[cfg(not(target_os = "linux"))]
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
        .invoke_handler(tauri::generate_handler![
            list_disks,
            analyze_blocks::analyze_blocks,
            find_file::find_jpeg,
            find_file::find_png,
            find_file::find_pdf,
            find_file::find_zip,
            find_file::find_txt,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
