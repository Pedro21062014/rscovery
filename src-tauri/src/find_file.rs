// User sends the device path and we scan the raw bytes looking for known
// magic byte signatures, carving the files out of the stream.
//
// Memory strategy:
// - Images (JPEG/PNG): buffered in memory only until the end signature
//   (capped at `max_size`), validated, saved to disk, and only a small
//   base64 thumbnail is sent to the frontend.
// - PDF/ZIP: streamed straight to a temp file on disk while scanning,
//   so RAM usage stays constant no matter how big the file is.
// - MP4: carved by walking the MP4 box structure (ftyp/moov/mdat...) and
//   copied to disk in chunks — also constant RAM.

use std::collections::{HashSet, VecDeque};
use std::fs::{self, File};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use base64::Engine;
use serde::Serialize;
use sha256::digest;
use tauri::Emitter;

use crate::analyze_blocks::{friendly_open_error, get_block_device_size_gb, normalize_device_path};

const BLOCK_SIZE: usize = 32 * 1024 * 1024;
/// How many leading bytes of a carved file are hashed for deduplication.
const SAMPLE_SIZE: usize = 2 * 1024 * 1024;
const FOUND_DIR: &str = "../found";
/// Hard cap for a single carved MP4 (4 GB).
const MAX_MP4_SIZE: u64 = 4 * 1024 * 1024 * 1024;

/// All default signatures
/// When `is_image`, the file is validated + saved to disk and a base64
/// thumbnail is sent to the frontend. Otherwise it is streamed to disk.
pub struct MagicByte<'s> {
    signature: &'s [u8],
    end: &'s [u8],
    extension: &'s str,
    max_size: usize,
    pub name: &'s str,
    is_image: bool,
}

#[derive(Serialize, Clone)]
struct ImageFound {
    base64: String,
    path: String,
    size: f64,
}

#[derive(Serialize, Clone)]
struct FileFind {
    path: String,
    size: f64,
}

#[derive(Serialize, Clone)]
struct Progress {
    current: f64,
    total: f64,
}

#[tauri::command]
pub async fn find_jpeg(
    app_handle: tauri::AppHandle,
    path: String,
    output_dir: Option<String>,
    min_dim: Option<u32>,
) -> Result<(), String> {
    let (scan_id, flag) = crate::begin_scan(&app_handle);
    crate::announce_scan(&app_handle, scan_id);
    let min_dim = min_dim.unwrap_or(0);
    tauri::async_runtime::spawn_blocking(move || {
        MagicByte {
            signature: &[0xFF, 0xD8],
            end: &[0xFF, 0xD9],
            extension: "jpeg",
            max_size: 64 * 1024 * 1024,
            name: "JPEG",
            is_image: true,
        }
        .extract(
            app_handle,
            &path,
            i32::MAX,
            output_dir.as_deref(),
            min_dim,
            flag,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn find_png(
    app_handle: tauri::AppHandle,
    path: String,
    output_dir: Option<String>,
    min_dim: Option<u32>,
) -> Result<(), String> {
    let (scan_id, flag) = crate::begin_scan(&app_handle);
    crate::announce_scan(&app_handle, scan_id);
    let min_dim = min_dim.unwrap_or(0);
    tauri::async_runtime::spawn_blocking(move || {
        MagicByte {
            signature: &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            end: &[0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82],
            extension: "png",
            max_size: 64 * 1024 * 1024,
            name: "PNG",
            is_image: true,
        }
        .extract(
            app_handle,
            &path,
            i32::MAX,
            output_dir.as_deref(),
            min_dim,
            flag,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn find_pdf(
    app_handle: tauri::AppHandle,
    path: String,
    output_dir: Option<String>,
) -> Result<(), String> {
    let (scan_id, flag) = crate::begin_scan(&app_handle);
    crate::announce_scan(&app_handle, scan_id);
    tauri::async_runtime::spawn_blocking(move || {
        MagicByte {
            signature: &[0x25, 0x50, 0x44, 0x46, 0x2D],
            end: &[0x25, 0x25, 0x45, 0x4F, 0x46],
            extension: "pdf",
            max_size: 500 * 1024 * 1024,
            name: "PDF",
            is_image: false,
        }
        .extract(app_handle, &path, i32::MAX, output_dir.as_deref(), 0, flag)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn find_zip(
    app_handle: tauri::AppHandle,
    path: String,
    output_dir: Option<String>,
) -> Result<(), String> {
    let (scan_id, flag) = crate::begin_scan(&app_handle);
    crate::announce_scan(&app_handle, scan_id);
    tauri::async_runtime::spawn_blocking(move || {
        MagicByte {
            signature: &[0x50, 0x4B, 0x03, 0x04],
            end: &[0x50, 0x4B, 0x05, 0x06],
            extension: "zip",
            max_size: 500 * 1024 * 1024,
            name: "ZIP",
            is_image: false,
        }
        .extract(app_handle, &path, i32::MAX, output_dir.as_deref(), 0, flag)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Streaming sink used for PDF/ZIP: bytes go straight to a temp file so the
/// scanner never holds the whole candidate in RAM.
struct DiskSink {
    tmp_path: String,
    final_path: String,
    size: u64,
    end_window: VecDeque<u8>,
    sample: Vec<u8>,
}

enum DiskFeed {
    Ok,
    /// Candidate aborted (write error or bigger than max_size).
    Aborted,
    /// End signature found and the file was finalized on disk.
    Complete(String, u64, String), // final_path, size, dedup_key
}

/// Feeds one byte into the streaming (PDF/ZIP) sink.
fn feed_disk(
    disk: &mut Option<(BufWriter<File>, DiskSink)>,
    b: u8,
    end_sig: &[u8],
    max_size: usize,
) -> DiskFeed {
    // Phase 1: write the byte and update the rolling window.
    let (write_ok, size_now, end_matched) = {
        let (writer, sink) = match disk.as_mut() {
            Some(x) => x,
            None => return DiskFeed::Aborted,
        };

        let write_ok = writer.write_all(&[b]).is_ok();
        if write_ok {
            sink.size += 1;
            if sink.sample.len() < SAMPLE_SIZE {
                sink.sample.push(b);
            }
            sink.end_window.push_back(b);
            if sink.end_window.len() > end_sig.len() {
                sink.end_window.pop_front();
            }
        }

        let end_matched = write_ok
            && sink.end_window.len() == end_sig.len()
            && sink.end_window.iter().zip(end_sig.iter()).all(|(x, y)| x == y);

        (write_ok, sink.size, end_matched)
    };

    if !write_ok || size_now > max_size as u64 {
        if let Some((_, s)) = disk.take() {
            let _ = fs::remove_file(&s.tmp_path);
        }
        return DiskFeed::Aborted;
    }

    if !end_matched {
        return DiskFeed::Ok;
    }

    // Phase 2: close the temp file and promote it to its final name.
    if let Some((mut writer, s)) = disk.take() {
        let mut ok = writer.flush().is_ok();
        drop(writer); // close the handle (required on Windows before rename)
        if ok {
            ok = fs::rename(&s.tmp_path, &s.final_path).is_ok();
        }
        if !ok {
            let _ = fs::remove_file(&s.tmp_path);
            return DiskFeed::Aborted;
        }
        let key = format!("{}:{}", s.size, digest(&s.sample));
        return DiskFeed::Complete(s.final_path, s.size, key);
    }

    DiskFeed::Aborted
}

/// Resolves where the recovered files go: the folder chosen by the user,
/// or the default `found` folder next to the app.
fn resolve_output_dir(output_dir: Option<&str>) -> Result<String, String> {
    match output_dir {
        Some(d) if !d.trim().is_empty() => {
            fs::create_dir_all(d).map_err(|e| {
                format!("Could not create the output folder '{}': {}", d, e)
            })?;
            Ok(d.to_string())
        }
        _ => {
            let _ = fs::create_dir_all(FOUND_DIR);
            Ok(FOUND_DIR.to_string())
        }
    }
}

/// Validates an image candidate, saves the full file to disk and returns a
/// small JPEG thumbnail (base64) for the frontend preview grid.
///
/// `min_dim` > 0 enables the "filter thumbnails" option: candidates smaller
/// than min_dim pixels on either side (or smaller than 10 KB) are skipped,
/// so only real images are recovered.
fn handle_image_candidate(
    mem: &[u8],
    extension: &str,
    count: i32,
    out_dir: &str,
    min_dim: u32,
) -> Option<ImageFound> {
    use image::imageops::FilterType;
    use image::ImageFormat;
    use std::io::Cursor;

    // Tiny candidates are almost certainly OS/browser thumbnails — skip them
    // early without spending time decoding.
    if min_dim > 0 && mem.len() < 10 * 1024 {
        return None;
    }

    // Decode validates the image; candidates capped at max_size keep this bounded.
    let img = image::load_from_memory(mem).ok()?;

    // Filter out small images (thumbnails) when the option is enabled.
    if min_dim > 0 && (img.width() < min_dim || img.height() < min_dim) {
        return None;
    }

    // Small thumbnail for the UI (full image goes to disk, not to the UI).
    let thumb = img.resize(256, 256, FilterType::Triangle).to_rgb8();
    let mut thumb_bytes: Vec<u8> = Vec::new();
    thumb
        .write_to(&mut Cursor::new(&mut thumb_bytes), ImageFormat::Jpeg)
        .ok()?;
    let base64 = base64::engine::general_purpose::STANDARD.encode(&thumb_bytes);

    let filename = format!("{out_dir}/{extension}_{count}.{extension}");
    fs::write(&filename, mem).ok()?;

    Some(ImageFound {
        base64,
        path: filename,
        size: mem.len() as f64 / 1024.0,
    })
}

impl<'s> MagicByte<'s> {
    pub fn extract(
        &self,
        app_handle: tauri::AppHandle,
        path: &str,
        max: i32,
        output_dir: Option<&str>,
        min_dim: u32,
        flag: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let open_path = normalize_device_path(path);
        let mut file = File::open(&open_path).map_err(|e| friendly_open_error(&e, path))?;

        let total_size = get_block_device_size_gb(path).map_err(|e| e.to_string())?;
        let out_dir = resolve_output_dir(output_dir)?;

        let mut buffer = vec![0u8; BLOCK_SIZE];
        let mut total_read: u64 = 0;

        let mut file_hash: HashSet<String> = HashSet::new();

        let mut searching_file = false;
        let mut sig_match_index = 0;

        // Image candidates are buffered in memory (capped), everything else
        // streams to a temp file on disk.
        let mut mem_buffer: Vec<u8> = Vec::new();
        let mut disk: Option<(BufWriter<File>, DiskSink)> = None;

        let mut count = 0;

        app_handle
            .emit(
                "file-progress",
                Progress {
                    current: 0.0,
                    total: total_size,
                },
            )
            .unwrap();

        loop {
            // Cancelled (new scan started, user left the page or pressed stop).
            if flag.load(Ordering::Relaxed) {
                break;
            }

            let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }
            total_read += bytes_read as u64;

            if count >= max {
                break;
            }

            let mut cancelled = false;
            for (idx, &b) in buffer[..bytes_read].iter().enumerate() {
                // Check the flag once every 64 KB (cheap, keeps the scan responsive).
                if idx % 65536 == 0 && flag.load(Ordering::Relaxed) {
                    cancelled = true;
                    break;
                }

                if searching_file {
                    if self.is_image {
                        mem_buffer.push(b);

                        if mem_buffer.len() > self.max_size {
                            searching_file = false;
                            mem_buffer.clear();
                            continue;
                        }

                        if mem_buffer.len() >= self.end.len()
                            && mem_buffer[mem_buffer.len() - self.end.len()..] == *self.end
                        {
                            let hash = digest(&mem_buffer);
                            if file_hash.insert(hash.clone()) {
                                if let Some(found) = handle_image_candidate(
                                    &mem_buffer,
                                    self.extension,
                                    count,
                                    &out_dir,
                                    min_dim,
                                ) {
                                    let _ = app_handle.emit("file-found", found);
                                    count += 1;
                                }
                            }

                            searching_file = false;
                            mem_buffer.clear();
                            sig_match_index = 0;
                        }
                        continue;
                    }

                    // ---- Disk-streamed candidate (PDF/ZIP) ----
                    match feed_disk(&mut disk, b, self.end, self.max_size) {
                        DiskFeed::Ok => {}
                        DiskFeed::Aborted => {
                            searching_file = false;
                            sig_match_index = 0;
                        }
                        DiskFeed::Complete(final_path, size, key) => {
                            if file_hash.insert(key) {
                                let _ = app_handle.emit(
                                    "file-found",
                                    FileFind {
                                        path: final_path,
                                        size: size as f64 / 1024.0,
                                    },
                                );
                                count += 1;
                            } else {
                                let _ = fs::remove_file(&final_path);
                            }
                            searching_file = false;
                            sig_match_index = 0;
                        }
                    }
                    continue;
                }

                if b == self.signature[sig_match_index] {
                    sig_match_index += 1;
                    if sig_match_index == self.signature.len() {
                        searching_file = true;
                        sig_match_index = 0;

                        if self.is_image {
                            mem_buffer.clear();
                            mem_buffer.extend_from_slice(self.signature);
                        } else {
                            let tmp_path =
                                format!("{out_dir}/.tmp_{}_{}", self.extension, count);
                            let final_path = format!(
                                "{out_dir}/{}_{}.{}",
                                self.extension, count, self.extension
                            );
                            match File::create(&tmp_path) {
                                Ok(f) => {
                                    let mut writer = BufWriter::new(f);
                                    let _ = writer.write_all(self.signature);
                                    let start_from = self
                                        .signature
                                        .len()
                                        .saturating_sub(self.end.len());
                                    disk = Some((
                                        writer,
                                        DiskSink {
                                            tmp_path,
                                            final_path,
                                            size: self.signature.len() as u64,
                                            end_window: self.signature[start_from..]
                                                .to_vec()
                                                .into(),
                                            sample: self.signature.to_vec(),
                                        },
                                    ));
                                }
                                Err(_) => {
                                    searching_file = false;
                                }
                            }
                        }
                    }
                } else {
                    sig_match_index = 0;
                }
            }

            if cancelled {
                break;
            }

            if !flag.load(Ordering::Relaxed) {
                app_handle
                    .emit(
                        "file-progress",
                        Progress {
                            current: total_read as f64 / 1024.0 / 1024.0,
                            total: total_size,
                        },
                    )
                    .unwrap();
            }
        }

        // Incomplete candidate at EOF: discard it.
        if let Some((writer, s)) = disk.take() {
            drop(writer);
            let _ = fs::remove_file(&s.tmp_path);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MP4 carving
// ---------------------------------------------------------------------------
//
// MP4 files don't have an end signature: they are a chain of "boxes"
// ([4-byte size][4-byte type][payload]). We locate the `ftyp` box, then walk
// the box chain with seeks until the headers stop making sense — that's the
// end of the video. The bytes are copied to disk in chunks, so RAM stays flat.

/// Measures a candidate MP4 starting at `start` by walking its top-level
/// boxes. Returns the total file length, or None if it isn't a valid MP4.
fn measure_mp4(file: &mut File, start: u64) -> Option<u64> {
    file.seek(SeekFrom::Start(start)).ok()?;

    let mut total: u64 = 0;
    let mut boxes = 0u32;
    let mut saw_media = false;
    let mut header = [0u8; 8];

    loop {
        // Device/file ended exactly after the last box: valid end.
        if file.read_exact(&mut header).is_err() {
            return if saw_media && total > 0 { Some(total) } else { None };
        }

        let mut size = u32::from_be_bytes(header[..4].try_into().unwrap()) as u64;
        let typ: [u8; 4] = header[4..].try_into().unwrap();
        let mut header_len: u64 = 8;

        if size == 1 {
            // 64-bit "largesize" box.
            let mut big = [0u8; 8];
            if file.read_exact(&mut big).is_err() {
                return if saw_media && total > 0 { Some(total) } else { None };
            }
            size = u64::from_be_bytes(big);
            header_len = 16;
        } else if size == 0 {
            // Box extends until EOF — we can't know where that is on a raw
            // device, so treat the video as ending before this box.
            return if saw_media && total > 0 { Some(total) } else { None };
        }

        let sane_size = size >= header_len && size <= MAX_MP4_SIZE && total + size <= MAX_MP4_SIZE;
        let sane_type = typ
            .iter()
            .all(|&c| c.is_ascii_alphanumeric() || c == b' ');

        if !sane_size || !sane_type || boxes > 10_000 {
            // Headers stopped making sense: the video ended right before
            // this box (the bytes after it belong to other data).
            return if saw_media && total > 0 { Some(total) } else { None };
        }

        // The first box must be ftyp (we matched it), and a real video needs
        // at least a moov or mdat box.
        if boxes == 0 && typ != *b"ftyp" {
            return None;
        }
        if typ == *b"moov" || typ == *b"mdat" {
            saw_media = true;
        }

        total += size;
        boxes += 1;

        if file.seek(SeekFrom::Current((size - header_len) as i64)).is_err() {
            return if saw_media && total > 0 { Some(total) } else { None };
        }
    }
}

/// Copies `len` bytes of the device starting at `start` into `dest`,
/// streaming in 8 MB chunks. Returns a dedup key (size + hash of first 2 MB).
fn copy_mp4(file: &mut File, start: u64, len: u64, dest: &str) -> std::io::Result<String> {
    file.seek(SeekFrom::Start(start))?;

    let mut out = BufWriter::new(File::create(dest)?);
    let mut buf = vec![0u8; 8 * 1024 * 1024];
    let mut sample: Vec<u8> = Vec::new();
    let mut left = len;

    while left > 0 {
        let want = (left as usize).min(buf.len());
        let r = file.read(&mut buf[..want])?;
        if r == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated video",
            ));
        }
        if sample.len() < SAMPLE_SIZE {
            let take = r.min(SAMPLE_SIZE - sample.len());
            sample.extend_from_slice(&buf[..take]);
        }
        out.write_all(&buf[..r])?;
        left -= r as u64;
    }

    out.flush()?;
    Ok(format!("{}:{}", len, digest(&sample)))
}

pub fn extract_mp4(
    app_handle: tauri::AppHandle,
    path: &str,
    max: i32,
    output_dir: Option<&str>,
    flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let open_path = normalize_device_path(path);
    let mut file = File::open(&open_path).map_err(|e| friendly_open_error(&e, path))?;

    let total_size = get_block_device_size_gb(path).map_err(|e| e.to_string())?;
    let out_dir = resolve_output_dir(output_dir)?;

    let mut buffer = vec![0u8; BLOCK_SIZE];
    let mut base: u64 = 0; // absolute offset of buffer[0]
    let mut count = 0i32;
    let mut found: HashSet<String> = HashSet::new();

    app_handle
        .emit(
            "file-progress",
            Progress {
                current: 0.0,
                total: total_size,
            },
        )
        .map_err(|e| e.to_string())?;

    'outer: loop {
        // Cancelled (new scan started, user left the page or pressed stop).
        if flag.load(Ordering::Relaxed) {
            break;
        }

        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }

        let mut resume: Option<u64> = None;
        let mut i = 0usize;

        while i + 4 <= n {
            if &buffer[i..i + 4] == b"ftyp" {
                let abs = base + i as u64; // absolute offset of the 'f'
                if abs >= 4 {
                    let start = abs - 4; // box header begins 4 bytes earlier
                    if let Some(len) = measure_mp4(&mut file, start) {
                        let out_path = format!("{out_dir}/mp4_{count}.mp4");
                        match copy_mp4(&mut file, start, len, &out_path) {
                            Ok(key) => {
                                if found.insert(key) {
                                    let _ = app_handle.emit(
                                        "file-found",
                                        FileFind {
                                            path: out_path,
                                            size: len as f64 / 1024.0,
                                        },
                                    );
                                    count += 1;
                                    if count >= max {
                                        break 'outer;
                                    }
                                    // Skip past the carved video and keep scanning.
                                    resume = Some(start + len);
                                    break;
                                } else {
                                    // Duplicate video: discard and move on.
                                    let _ = fs::remove_file(&out_path);
                                    i += 4;
                                    continue;
                                }
                            }
                            Err(_) => {
                                let _ = fs::remove_file(&out_path);
                                i += 4;
                                continue;
                            }
                        }
                    }
                }
                i += 1;
            } else {
                i += 1;
            }
        }

        // Position for the next read: right after a carved video, or slightly
        // overlapping so a signature spanning two buffers is not missed.
        let next_pos = resume.unwrap_or_else(|| (base + n as u64).saturating_sub(3));
        file.seek(SeekFrom::Start(next_pos))
            .map_err(|e| e.to_string())?;
        base = next_pos;

        app_handle
            .emit(
                "file-progress",
                Progress {
                    current: base as f64 / 1024.0 / 1024.0,
                    total: total_size,
                },
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn find_mp4(
    app_handle: tauri::AppHandle,
    path: String,
    output_dir: Option<String>,
) -> Result<(), String> {
    let (scan_id, flag) = crate::begin_scan(&app_handle);
    crate::announce_scan(&app_handle, scan_id);
    tauri::async_runtime::spawn_blocking(move || {
        extract_mp4(app_handle, &path, i32::MAX, output_dir.as_deref(), flag)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn find_txt(
    app_handle: tauri::AppHandle,
    path: String,
    wordlist: Vec<String>,
    blacklist: Vec<String>,
) -> Result<(), String> {
    println!("wordlist: {:?} \n blacklist:{:?}", wordlist, blacklist);
    let (scan_id, flag) = crate::begin_scan(&app_handle);
    crate::announce_scan(&app_handle, scan_id);
    tauri::async_runtime::spawn_blocking(move || {
        extract_txt(app_handle, &path, i32::MAX, wordlist, blacklist, flag)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize, Clone)]
struct TextFound {
    text: String,
}

pub fn extract_txt(
    app_handle: tauri::AppHandle,
    path: &str,
    max: i32,
    wordlist: Vec<String>,
    blacklist: Vec<String>,
    flag: Arc<AtomicBool>,
) -> Result<(), String> {
    let open_path = normalize_device_path(path);
    let mut file = File::open(&open_path).map_err(|e| friendly_open_error(&e, path))?;
    let total_size = get_block_device_size_gb(path).map_err(|e| e.to_string())?;

    let mut buffer = vec![0u8; BLOCK_SIZE]; // 32 MB buffer
    let mut total_read: u64 = 0;
    let mut text_buffer: Vec<u8> = Vec::new(); // incremental text buffer
    let mut count = 0;

    // lowercase wordlist/blacklist for case-insensitive search
    let wordlist: Vec<String> = wordlist.into_iter().map(|s| s.to_lowercase()).collect();
    let blacklist: Vec<String> = blacklist.into_iter().map(|s| s.to_lowercase()).collect();

    let mut found_hashes: HashSet<String> = HashSet::new();

    let _ = app_handle.emit(
        "file-progress",
        Progress {
            current: 0.0,
            total: total_size,
        },
    );

    loop {
        // Cancelled (new scan started, user left the page or pressed stop).
        if flag.load(Ordering::Relaxed) {
            break;
        }

        let bytes_read = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }
        total_read += bytes_read as u64;

        let mut cancelled = false;
        for (idx, &b) in buffer[..bytes_read].iter().enumerate() {
            if idx % 65536 == 0 && flag.load(Ordering::Relaxed) {
                cancelled = true;
                break;
            }

            if b == 0x09 || b == 0x0A || b == 0x0D || (0x20..=0x7E).contains(&b) {
                text_buffer.push(b);
                // 64 KB
                if text_buffer.len() > (64 * 1024) {
                    text_buffer.clear();
                }
            } else {
                if text_buffer.len() >= 32 {
                    let text = String::from_utf8_lossy(&text_buffer).to_string();
                    let text_lower = text.to_lowercase();

                    if !blacklist.iter().any(|b| text_lower.contains(b))
                        && wordlist.iter().any(|w| text_lower.contains(w))
                    {
                        let hash = digest(&text);
                        if !found_hashes.contains(&hash) {
                            found_hashes.insert(hash);

                            count += 1;
                            let _ = app_handle.emit("text-found", TextFound { text: text.clone() });
                            if count >= max {
                                return Ok(());
                            }
                        }
                    }
                }
                // reset buffer
                text_buffer.clear();
            }
        }

        if cancelled {
            break;
        }

        // emit progress
        if !flag.load(Ordering::Relaxed) {
            let _ = app_handle.emit(
                "file-progress",
                Progress {
                    current: total_read as f64 / 1024.0 / 1024.0,
                    total: total_size,
                },
            );
        }
    }

    if text_buffer.len() >= 32 {
        let text = String::from_utf8_lossy(&text_buffer).to_string();
        let text_lower = text.to_lowercase();
        if !blacklist.iter().any(|b| text_lower.contains(b))
            && wordlist.iter().any(|w| text_lower.contains(w))
        {
            let hash = digest(&text);
            if !found_hashes.contains(&hash) {
                found_hashes.insert(hash);
                count += 1;
                let _ = app_handle.emit("text-found", TextFound { text: text.clone() });
            }
        }
    }

    Ok(())
}
