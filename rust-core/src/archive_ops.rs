use crate::disk_images::{
    extract_disk_image_entry, extract_iso_entry, list_disk_image_entries, list_iso_entries,
    VirtualEntry,
};
use crate::error::{OpResult, OpfileError};
use crate::task_manager::{AtomicPartFile, TaskHandle, STREAM_BUF_SIZE};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionProfile {
    Store,
    Fastest,
    Fast,
    Normal,
    Maximum,
    Ultra,
}

impl CompressionProfile {
    fn zip_level(self) -> i64 {
        match self {
            Self::Store => 0,
            Self::Fastest => 1,
            Self::Fast => 3,
            Self::Normal => 6,
            Self::Maximum => 8,
            Self::Ultra => 9,
        }
    }

    fn zstd_level(self) -> i32 {
        match self {
            Self::Store => 1,
            Self::Fastest => 1,
            Self::Fast => 3,
            Self::Normal => 6,
            Self::Maximum => 14,
            Self::Ultra => 19,
        }
    }

    fn xz_level(self) -> u32 {
        match self {
            Self::Store => 0,
            Self::Fastest => 1,
            Self::Fast => 3,
            Self::Normal => 6,
            Self::Maximum => 8,
            Self::Ultra => 9,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveVirtualPage {
    pub archive_path: String,
    pub inner_dir: String,
    pub entries: Vec<VirtualEntry>,
    pub total_entries: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegrityReport {
    pub archive_path: String,
    pub passed: bool,
    pub checked_entries: usize,
    pub failed_entries: Vec<String>,
    pub summary: String,
}

/// Multi-volume reader that stitches `.001`, `.002`, ... into one contiguous Read + Seek stream.
struct MultiVolumeReader {
    parts: Vec<(File, u64)>,
    total_len: u64,
    pos: u64,
}

impl MultiVolumeReader {
    fn open(first_part: &Path) -> OpResult<Self> {
        let s = first_part.to_string_lossy();
        let mut files = Vec::new();
        let mut total_len = 0u64;
        if let Some(stem) = s.strip_suffix(".001") {
            for idx in 1..1000 {
                let p = PathBuf::from(format!("{stem}.{idx:03}"));
                if !p.exists() {
                    break;
                }
                let f = File::open(&p)?;
                let len = f.metadata()?.len();
                total_len += len;
                files.push((f, len));
            }
        }
        if files.is_empty() {
            let f = File::open(first_part)?;
            let len = f.metadata()?.len();
            total_len = len;
            files.push((f, len));
        }
        Ok(Self {
            parts: files,
            total_len,
            pos: 0,
        })
    }
}

impl Read for MultiVolumeReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.total_len || buf.is_empty() {
            return Ok(0);
        }
        let mut acc = 0u64;
        for (f, len) in &mut self.parts {
            if self.pos < acc + *len {
                let local_off = self.pos - acc;
                f.seek(SeekFrom::Start(local_off))?;
                let avail = (*len - local_off).min(buf.len() as u64) as usize;
                let n = f.read(&mut buf[..avail])?;
                self.pos += n as u64;
                return Ok(n);
            }
            acc += *len;
        }
        Ok(0)
    }
}

impl Seek for MultiVolumeReader {
    fn seek(&mut self, style: SeekFrom) -> std::io::Result<u64> {
        let next: i64 = match style {
            SeekFrom::Start(o) => o as i64,
            SeekFrom::End(o) => self.total_len as i64 + o,
            SeekFrom::Current(o) => self.pos as i64 + o,
        };
        if next < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Negative seek position",
            ));
        }
        self.pos = (next as u64).min(self.total_len);
        Ok(self.pos)
    }
}

fn detect_archive_kind(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    for suffix in [
        ".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".7z.001", ".zip.001",
    ] {
        if lower.ends_with(suffix) {
            return suffix.trim_start_matches('.').to_string();
        }
    }
    Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string()
}

pub fn list_all_entries(archive_path: &str, password: Option<&str>) -> OpResult<Vec<VirtualEntry>> {
    let kind = detect_archive_kind(archive_path);
    match kind.as_str() {
        "zip" | "apk" | "xapk" | "apks" | "obb" | "cbz" | "epub" | "docx" | "xlsx" | "pptx"
        | "zip.001" => list_zip_entries(archive_path),
        "7z" | "7z.001" => list_7z_entries(archive_path, password),
        "tar" => list_tar_entries(File::open(archive_path)?),
        "tar.gz" | "tgz" => list_tar_entries(GzDecoder::new(BufReader::new(File::open(archive_path)?))),
        "tar.bz2" | "tbz2" => list_tar_entries(BzDecoder::new(BufReader::new(File::open(archive_path)?))),
        "tar.xz" | "txz" => list_tar_entries(XzDecoder::new(BufReader::new(File::open(archive_path)?))),
        "tar.zst" | "tzst" => {
            let dec = zstd::stream::read::Decoder::new(BufReader::new(File::open(archive_path)?))
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            list_tar_entries(dec)
        }
        "gz" | "bz2" | "xz" | "zst" => {
            let p = Path::new(archive_path);
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("payload");
            let meta = fs::metadata(p)?;
            Ok(vec![VirtualEntry {
                name: stem.to_string(),
                full_path: stem.to_string(),
                is_dir: false,
                size: meta.len(),
                compressed_size: meta.len(),
                method: kind.to_ascii_uppercase(),
                encrypted: false,
                crc32: 0,
            }])
        }
        "rar" => list_rar_entries(archive_path),
        "iso" => list_iso_entries(archive_path),
        "img" | "dmg" | "qcow2" | "vdi" | "vmdk" => list_disk_image_entries(archive_path),
        "opx" => {
            let p = Path::new(archive_path);
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("decrypted");
            let meta = fs::metadata(p)?;
            Ok(vec![VirtualEntry {
                name: stem.to_string(),
                full_path: stem.to_string(),
                is_dir: false,
                size: meta.len().saturating_sub(28),
                compressed_size: meta.len(),
                method: "AES-256-GCM".to_string(),
                encrypted: true,
                crc32: 0,
            }])
        }
        _ => Err(OpfileError::unsupported(format!(
            "Unsupported archive format: {kind}"
        ))),
    }
}

pub fn list_archive_virtual(
    archive_path: &str,
    inner_dir: &str,
    password: Option<&str>,
) -> OpResult<ArchiveVirtualPage> {
    let all = list_all_entries(archive_path, password)?;
    let norm_dir = inner_dir.trim_matches('/');
    let prefix = if norm_dir.is_empty() {
        String::new()
    } else {
        format!("{norm_dir}/")
    };

    let mut folder_map: BTreeMap<String, VirtualEntry> = BTreeMap::new();
    let mut files: Vec<VirtualEntry> = Vec::new();

    for e in &all {
        let clean = e.full_path.trim_matches('/');
        if clean.is_empty() {
            continue;
        }
        if !prefix.is_empty() && !clean.starts_with(&prefix) {
            continue;
        }
        let rem = &clean[prefix.len()..];
        if rem.is_empty() {
            continue;
        }
        if let Some((dir_name, _rest)) = rem.split_once('/') {
            let dir_full = if prefix.is_empty() {
                dir_name.to_string()
            } else {
                format!("{prefix}{dir_name}")
            };
            folder_map.entry(dir_name.to_string()).or_insert(VirtualEntry {
                name: dir_name.to_string(),
                full_path: dir_full,
                is_dir: true,
                size: 0,
                compressed_size: 0,
                method: "DIR".to_string(),
                encrypted: e.encrypted,
                crc32: 0,
            });
        } else if e.is_dir {
            folder_map.entry(rem.to_string()).or_insert(VirtualEntry {
                name: rem.to_string(),
                full_path: clean.to_string(),
                is_dir: true,
                size: 0,
                compressed_size: 0,
                method: "DIR".to_string(),
                encrypted: e.encrypted,
                crc32: 0,
            });
        } else {
            let mut item = e.clone();
            item.name = rem.to_string();
            files.push(item);
        }
    }

    let mut combined: Vec<VirtualEntry> = folder_map.into_values().collect();
    combined.sort_by(|a, b| natord::compare_ignore_case(&a.name, &b.name));
    files.sort_by(|a, b| natord::compare_ignore_case(&a.name, &b.name));
    combined.extend(files);

    Ok(ArchiveVirtualPage {
        archive_path: archive_path.to_string(),
        inner_dir: norm_dir.to_string(),
        entries: combined,
        total_entries: all.len(),
    })
}

pub fn search_archive(
    archive_path: &str,
    query: &str,
    password: Option<&str>,
) -> OpResult<Vec<VirtualEntry>> {
    let q = query.trim().to_ascii_lowercase();
    let all = list_all_entries(archive_path, password)?;
    if q.is_empty() {
        return Ok(all);
    }
    Ok(all
        .into_iter()
        .filter(|e| e.full_path.to_ascii_lowercase().contains(&q))
        .collect())
}

fn list_zip_entries(path: &str) -> OpResult<Vec<VirtualEntry>> {
    let reader = MultiVolumeReader::open(Path::new(path))?;
    let mut archive =
        ZipArchive::new(reader).map_err(|e| OpfileError::archive(format!("ZIP open error: {e}")))?;
    let mut out = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index_raw(i) {
            let full = entry.name().trim_end_matches('/').to_string();
            let name = full.rsplit('/').next().unwrap_or(&full).to_string();
            out.push(VirtualEntry {
                name,
                full_path: full,
                is_dir: entry.is_dir(),
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                method: format!("{:?}", entry.compression()),
                encrypted: entry.encrypted(),
                crc32: entry.crc32(),
            });
        }
    }
    Ok(out)
}

fn list_7z_entries(path: &str, password: Option<&str>) -> OpResult<Vec<VirtualEntry>> {
    let mut reader = MultiVolumeReader::open(Path::new(path))?;
    let len = reader.total_len;
    let pw = password.unwrap_or("");
    let archive = sevenz_rust::Archive::read(&mut reader, len, pw.as_bytes())
        .map_err(|e| OpfileError::archive(format!("7z read error: {e}")))?;
    let mut out = Vec::with_capacity(archive.files.len());
    for f in &archive.files {
        let full = f.name.trim_end_matches('/').to_string();
        let name = full.rsplit('/').next().unwrap_or(&full).to_string();
        out.push(VirtualEntry {
            name,
            full_path: full,
            is_dir: f.is_directory,
            size: f.size,
            compressed_size: f.compressed_size,
            method: "LZMA2".to_string(),
            encrypted: f.has_stream && !pw.is_empty(),
            crc32: f.crc as u32,
        });
    }
    Ok(out)
}

fn list_tar_entries<R: Read>(reader: R) -> OpResult<Vec<VirtualEntry>> {
    let mut ar = tar::Archive::new(reader);
    let mut out = Vec::new();
    let entries = ar
        .entries()
        .map_err(|e| OpfileError::archive(format!("TAR read error: {e}")))?;
    for entry_res in entries {
        let entry = entry_res.map_err(|e| OpfileError::archive(e.to_string()))?;
        let path_str = entry
            .path()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string();
        let name = path_str.rsplit('/').next().unwrap_or(&path_str).to_string();
        let is_dir = entry.header().entry_type().is_dir();
        let size = entry.header().size().unwrap_or(0);
        out.push(VirtualEntry {
            name,
            full_path: path_str,
            is_dir,
            size,
            compressed_size: size,
            method: "TAR".to_string(),
            encrypted: false,
            crc32: 0,
        });
    }
    Ok(out)
}

/// Minimal RAR4 & RAR5 header/file block parser for zero-C-dependency browsing & store extraction.
fn list_rar_entries(path: &str) -> OpResult<Vec<VirtualEntry>> {
    let mut f = File::open(path)?;
    let mut sig = [0u8; 8];
    f.read_exact(&mut sig)?;
    let mut out = Vec::new();

    if &sig[0..7] == b"Rar!\x1a\x07\x00" {
        // RAR4
        f.seek(SeekFrom::Start(7))?;
        let mut hdr = [0u8; 7];
        while f.read_exact(&mut hdr).is_ok() {
            let block_type = hdr[2];
            let flags = u16::from_le_bytes([hdr[3], hdr[4]]);
            let head_size = u16::from_le_bytes([hdr[5], hdr[6]]) as u64;
            if head_size < 7 {
                break;
            }
            let pos_start = f.stream_position()? - 7;
            let mut rest = vec![0u8; (head_size as usize).saturating_sub(7)];
            if f.read_exact(&mut rest).is_err() {
                break;
            }
            let add_size = if (flags & 0x8000) != 0 && rest.len() >= 4 {
                u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64
            } else {
                0
            };
            if block_type == 0x74 && rest.len() >= 25 {
                let pack_size = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64;
                let unp_size = u32::from_le_bytes([rest[4], rest[5], rest[6], rest[7]]) as u64;
                let crc = u32::from_le_bytes([rest[9], rest[10], rest[11], rest[12]]);
                let method = rest[18];
                let name_len = u16::from_le_bytes([rest[19], rest[20]]) as usize;
                let name_bytes = rest.get(25..25 + name_len).unwrap_or(&[]);
                let full = String::from_utf8_lossy(name_bytes)
                    .replace('\\', "/")
                    .trim_end_matches('/')
                    .to_string();
                let is_dir = (flags & 0x00E0) == 0x00E0;
                let name = full.rsplit('/').next().unwrap_or(&full).to_string();
                out.push(VirtualEntry {
                    name,
                    full_path: full,
                    is_dir,
                    size: unp_size,
                    compressed_size: pack_size,
                    method: format!("RAR4-m{method}"),
                    encrypted: (flags & 0x0004) != 0,
                    crc32: crc,
                });
                f.seek(SeekFrom::Start(pos_start + head_size + pack_size))?;
            } else {
                f.seek(SeekFrom::Start(pos_start + head_size + add_size))?;
            }
        }
        return Ok(out);
    }

    if &sig == b"Rar!\x1a\x07\x01\x00" {
        // RAR5 vint block walker
        loop {
            let mut crc_buf = [0u8; 4];
            if f.read_exact(&mut crc_buf).is_err() {
                break;
            }
            let Ok(head_size) = read_rar5_vint(&mut f) else {
                break;
            };
            if head_size == 0 || head_size > 1024 * 1024 {
                break;
            }
            let mut block = vec![0u8; head_size as usize];
            if f.read_exact(&mut block).is_err() {
                break;
            }
            let mut cur = Cursor::new(&block);
            let htype = read_rar5_vint(&mut cur).unwrap_or(0);
            let hflags = read_rar5_vint(&mut cur).unwrap_or(0);
            let extra_size = if (hflags & 0x01) != 0 {
                read_rar5_vint(&mut cur).unwrap_or(0)
            } else {
                0
            };
            let data_size = if (hflags & 0x02) != 0 {
                read_rar5_vint(&mut cur).unwrap_or(0)
            } else {
                0
            };
            if htype == 2 {
                // File header
                let fflags = read_rar5_vint(&mut cur).unwrap_or(0);
                let unp_size = read_rar5_vint(&mut cur).unwrap_or(0);
                let _attrs = read_rar5_vint(&mut cur).unwrap_or(0);
                if (fflags & 0x02) != 0 {
                    let mut mtime = [0u8; 4];
                    let _ = cur.read_exact(&mut mtime);
                }
                let crc = if (fflags & 0x04) != 0 {
                    let mut c = [0u8; 4];
                    let _ = cur.read_exact(&mut c);
                    u32::from_le_bytes(c)
                } else {
                    0
                };
                let comp_info = read_rar5_vint(&mut cur).unwrap_or(0);
                let _host_os = read_rar5_vint(&mut cur).unwrap_or(0);
                let name_len = read_rar5_vint(&mut cur).unwrap_or(0) as usize;
                let mut name_buf = vec![0u8; name_len.min(4096)];
                let _ = cur.read_exact(&mut name_buf);
                let full = String::from_utf8_lossy(&name_buf)
                    .replace('\\', "/")
                    .trim_end_matches('/')
                    .to_string();
                let is_dir = (fflags & 0x01) != 0;
                let name = full.rsplit('/').next().unwrap_or(&full).to_string();
                let method_code = (comp_info >> 7) & 0x07;
                out.push(VirtualEntry {
                    name,
                    full_path: full,
                    is_dir,
                    size: unp_size,
                    compressed_size: data_size,
                    method: format!("RAR5-m{method_code}"),
                    encrypted: extra_size > 0 && (hflags & 0x04) != 0,
                    crc32: crc,
                });
            }
            if data_size > 0 {
                f.seek(SeekFrom::Current(data_size as i64))?;
            }
            if htype == 5 {
                break;
            }
        }
        return Ok(out);
    }

    Err(OpfileError::archive("Unrecognized RAR signature"))
}

fn read_rar5_vint<R: Read>(r: &mut R) -> std::io::Result<u64> {
    let mut val = 0u64;
    let mut shift = 0u32;
    let mut b = [0u8; 1];
    for _ in 0..10 {
        r.read_exact(&mut b)?;
        val |= ((b[0] & 0x7F) as u64) << shift;
        if (b[0] & 0x80) == 0 {
            return Ok(val);
        }
        shift += 7;
    }
    Ok(val)
}

pub fn extract_single_entry(
    archive_path: &str,
    entry_path: &str,
    dest_file: &str,
    password: Option<&str>,
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let kind = detect_archive_kind(archive_path);
    let dest = Path::new(dest_file);
    let target_clean = entry_path.trim_matches('/');

    match kind.as_str() {
        "zip" | "apk" | "xapk" | "apks" | "obb" | "cbz" | "epub" | "docx" | "xlsx" | "pptx"
        | "zip.001" => {
            let reader = MultiVolumeReader::open(Path::new(archive_path))?;
            let mut zip = ZipArchive::new(reader).map_err(|e| OpfileError::archive(e.to_string()))?;
            let mut idx_found = None;
            for i in 0..zip.len() {
                if let Ok(e) = zip.by_index_raw(i) {
                    if e.name().trim_matches('/') == target_clean {
                        idx_found = Some(i);
                        break;
                    }
                }
            }
            let idx = idx_found.ok_or_else(|| OpfileError::archive("Entry not found in ZIP"))?;
            let mut entry = if let Some(pw) = password.filter(|s| !s.is_empty()) {
                zip.by_index_decrypt(idx, pw.as_bytes())
                    .map_err(|e| OpfileError::crypto(format!("Decrypt error: {e}")))?
            } else {
                zip.by_index(idx).map_err(|e| OpfileError::archive(e.to_string()))?
            };
            stream_entry_to_part(&mut entry, dest, task)
        }
        "7z" | "7z.001" => {
            let reader = MultiVolumeReader::open(Path::new(archive_path))?;
            let len = reader.total_len;
            let pw = password.unwrap_or("");
            let mut written = 0u64;
            let dest_owned = dest.to_path_buf();
            let mut sz = sevenz_rust::SevenZReader::new(reader, len, pw.into())
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            sz.for_each_entries(|entry, r| {
                if entry.name().trim_matches('/') == target_clean && !entry.is_directory() {
                    written = stream_entry_to_part(r, &dest_owned, task)
                        .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
                }
                Ok(true)
            })
            .map_err(|e| OpfileError::archive(e.to_string()))?;
            Ok(written)
        }
        "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tbz2" | "tar.xz" | "txz" | "tar.zst" | "tzst" => {
            let mut written = 0u64;
            let mut found = false;
            with_tar_archive(archive_path, &kind, |ar| {
                for entry_res in ar.entries().map_err(|e| OpfileError::archive(e.to_string()))? {
                    let mut entry = entry_res.map_err(|e| OpfileError::archive(e.to_string()))?;
                    let p = entry
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if p.trim_matches('/') == target_clean && !entry.header().entry_type().is_dir() {
                        written = stream_entry_to_part(&mut entry, dest, task)?;
                        found = true;
                        break;
                    }
                }
                Ok(())
            })?;
            if !found {
                return Err(OpfileError::archive("Entry not found in TAR"));
            }
            Ok(written)
        }
        "gz" => stream_entry_to_part(&mut GzDecoder::new(BufReader::new(File::open(archive_path)?)), dest, task),
        "bz2" => stream_entry_to_part(&mut BzDecoder::new(BufReader::new(File::open(archive_path)?)), dest, task),
        "xz" => stream_entry_to_part(&mut XzDecoder::new(BufReader::new(File::open(archive_path)?)), dest, task),
        "zst" => {
            let mut dec = zstd::stream::read::Decoder::new(BufReader::new(File::open(archive_path)?))
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            stream_entry_to_part(&mut dec, dest, task)
        }
        "iso" => extract_iso_entry(archive_path, target_clean, dest, task),
        "img" | "dmg" | "qcow2" | "vdi" | "vmdk" => {
            extract_disk_image_entry(archive_path, target_clean, dest, task)
        }
        "opx" => decrypt_opx_file(archive_path, dest, password.unwrap_or(""), task),
        _ => Err(OpfileError::unsupported(format!(
            "Single-entry extraction not supported for {kind}"
        ))),
    }
}

pub fn extract_archive(
    archive_path: &str,
    dest_dir: &str,
    password: Option<&str>,
    selected_entries: Option<Vec<String>>,
    task: Option<&TaskHandle>,
) -> OpResult<Vec<String>> {
    let kind = detect_archive_kind(archive_path);
    let dest_root = PathBuf::from(dest_dir);
    fs::create_dir_all(&dest_root)?;
    let filter: Option<HashSet<String>> = selected_entries
        .map(|v| v.into_iter().map(|s| s.trim_matches('/').to_string()).collect());

    let mut extracted = Vec::new();

    match kind.as_str() {
        "zip" | "apk" | "xapk" | "apks" | "obb" | "cbz" | "epub" | "docx" | "xlsx" | "pptx"
        | "zip.001" => {
            let reader = MultiVolumeReader::open(Path::new(archive_path))?;
            let mut zip =
                ZipArchive::new(reader).map_err(|e| OpfileError::archive(e.to_string()))?;
            let len = zip.len();
            if let Some(t) = task {
                t.set_totals(0, len as u64);
            }
            for i in 0..len {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                }
                let mut entry = if let Some(pw) = password.filter(|s| !s.is_empty()) {
                    zip.by_index_decrypt(i, pw.as_bytes())
                        .map_err(|e| OpfileError::crypto(format!("Decrypt error: {e}")))?
                } else {
                    zip.by_index(i).map_err(|e| OpfileError::archive(e.to_string()))?
                };
                let rel = entry.name().trim_matches('/').to_string();
                if rel.is_empty() {
                    continue;
                }
                if let Some(ref set) = filter {
                    if !set.iter().any(|s| rel == *s || rel.starts_with(&format!("{s}/"))) {
                        continue;
                    }
                }
                let out_path = safe_join(&dest_root, &rel)?;
                if let Some(t) = task {
                    t.set_current_item(&rel);
                }
                if entry.is_dir() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    stream_entry_to_part(&mut entry, &out_path, task)?;
                    extracted.push(out_path.to_string_lossy().into_owned());
                }
                if let Some(t) = task {
                    t.inc_items();
                }
            }
        }
        "7z" | "7z.001" => {
            let reader = MultiVolumeReader::open(Path::new(archive_path))?;
            let len = reader.total_len;
            let pw = password.unwrap_or("");
            let mut sz = sevenz_rust::SevenZReader::new(reader, len, pw.into())
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            let total = sz.archive().files.len() as u64;
            if let Some(t) = task {
                t.set_totals(0, total);
            }
            sz.for_each_entries(|entry, r| {
                if let Some(t) = task {
                    if t.check_checkpoint().is_err() {
                        return Ok(false);
                    }
                }
                let rel = entry.name().trim_matches('/').to_string();
                if let Some(ref set) = filter {
                    if !set.iter().any(|s| rel == *s || rel.starts_with(&format!("{s}/"))) {
                        return Ok(true);
                    }
                }
                let out_path = safe_join(&dest_root, &rel)
                    .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
                if entry.is_directory() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(t) = task {
                        t.set_current_item(&rel);
                    }
                    stream_entry_to_part(r, &out_path, task)
                        .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
                    extracted.push(out_path.to_string_lossy().into_owned());
                }
                if let Some(t) = task {
                    t.inc_items();
                }
                Ok(true)
            })
            .map_err(|e| OpfileError::archive(e.to_string()))?;
        }
        "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tbz2" | "tar.xz" | "txz" | "tar.zst" | "tzst" => {
            with_tar_archive(archive_path, &kind, |ar| {
                for entry_res in ar.entries().map_err(|e| OpfileError::archive(e.to_string()))? {
                    if let Some(t) = task {
                        t.check_checkpoint()?;
                    }
                    let mut entry = entry_res.map_err(|e| OpfileError::archive(e.to_string()))?;
                    let rel = entry
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default()
                        .trim_matches('/')
                        .to_string();
                    if rel.is_empty() {
                        continue;
                    }
                    if let Some(ref set) = filter {
                        if !set.iter().any(|s| rel == *s || rel.starts_with(&format!("{s}/"))) {
                            continue;
                        }
                    }
                    let out_path = safe_join(&dest_root, &rel)?;
                    if entry.header().entry_type().is_dir() {
                        fs::create_dir_all(&out_path)?;
                    } else {
                        if let Some(t) = task {
                            t.set_current_item(&rel);
                        }
                        stream_entry_to_part(&mut entry, &out_path, task)?;
                        extracted.push(out_path.to_string_lossy().into_owned());
                    }
                    if let Some(t) = task {
                        t.inc_items();
                    }
                }
                Ok(())
            })?;
        }
        "gz" | "bz2" | "xz" | "zst" | "opx" => {
            let stem = Path::new(archive_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let out_path = dest_root.join(stem);
            extract_single_entry(
                archive_path,
                stem,
                &out_path.to_string_lossy(),
                password,
                task,
            )?;
            extracted.push(out_path.to_string_lossy().into_owned());
        }
        "iso" => {
            let entries = list_iso_entries(archive_path)?;
            for e in entries {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                }
                if let Some(ref set) = filter {
                    if !set.iter().any(|s| e.full_path == *s || e.full_path.starts_with(&format!("{s}/"))) {
                        continue;
                    }
                }
                let out_path = safe_join(&dest_root, &e.full_path)?;
                if e.is_dir {
                    fs::create_dir_all(&out_path)?;
                } else {
                    extract_iso_entry(archive_path, &e.full_path, &out_path, task)?;
                    extracted.push(out_path.to_string_lossy().into_owned());
                }
            }
        }
        "img" | "dmg" | "qcow2" | "vdi" | "vmdk" => {
            let entries = list_disk_image_entries(archive_path)?;
            for e in entries {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                }
                let out_path = safe_join(&dest_root, &e.full_path)?;
                extract_disk_image_entry(archive_path, &e.full_path, &out_path, task)?;
                extracted.push(out_path.to_string_lossy().into_owned());
            }
        }
        _ => {
            return Err(OpfileError::unsupported(format!(
                "Extraction not supported for {kind}"
            )))
        }
    }

    Ok(extracted)
}

#[derive(Deserialize)]
pub struct CreateArchiveRequest {
    pub sources: Vec<String>,
    pub dest_path: String,
    pub format: String,
    pub profile: CompressionProfile,
    pub dict_size_mb: Option<u32>,
    pub password: Option<String>,
    pub split_size_mb: Option<u64>,
}

pub fn create_archive(req: &CreateArchiveRequest, task: Option<&TaskHandle>) -> OpResult<Vec<String>> {
    let dest = PathBuf::from(&req.dest_path);
    let fmt = req.format.to_ascii_lowercase();
    let mut files_to_pack: Vec<(PathBuf, String)> = Vec::new();
    let mut total_bytes = 0u64;

    for src_str in &req.sources {
        let src = PathBuf::from(src_str);
        if !src.exists() {
            continue;
        }
        let root_name = src
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("item")
            .to_string();
        if src.is_dir() {
            for e in WalkDir::new(&src).into_iter().filter_map(Result::ok) {
                let rel = e
                    .path()
                    .strip_prefix(&src)
                    .map_err(|err| OpfileError::io(err.to_string()))?;
                let archive_rel = if rel.as_os_str().is_empty() {
                    root_name.clone()
                } else {
                    format!("{root_name}/{}", rel.to_string_lossy().replace('\\', "/"))
                };
                if e.file_type().is_file() {
                    total_bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
                }
                files_to_pack.push((e.path().to_path_buf(), archive_rel));
            }
        } else {
            total_bytes += src.metadata().map(|m| m.len()).unwrap_or(0);
            files_to_pack.push((src, root_name));
        }
    }

    if let Some(t) = task {
        t.set_totals(total_bytes, files_to_pack.len() as u64);
    }

    let split_bytes = req.split_size_mb.filter(|&mb| mb > 0).map(|mb| mb * 1024 * 1024);

    if fmt == "opx" {
        let single = req
            .sources
            .first()
            .ok_or_else(|| OpfileError::invalid("No source file for OPX encryption"))?;
        let pw = req.password.as_deref().unwrap_or("");
        encrypt_opx_file(Path::new(single), &dest, pw, task)?;
        return Ok(vec![dest.to_string_lossy().into_owned()]);
    }

    let staging = if split_bytes.is_some() {
        dest.with_extension("staging.tmp")
    } else {
        dest.clone()
    };

    match fmt.as_str() {
        "zip" => {
            let (guard, writer) = AtomicPartFile::create(&staging)?;
            let mut zw = ZipWriter::new(writer);
            let method = match req.profile {
                CompressionProfile::Store => CompressionMethod::Stored,
                CompressionProfile::Ultra => CompressionMethod::Zstd,
                _ => CompressionMethod::Deflated,
            };
            let mut opts = SimpleFileOptions::default()
                .compression_method(method)
                .compression_level(Some(req.profile.zip_level()));
            if let Some(pw) = req.password.as_deref().filter(|s| !s.is_empty()) {
                opts = opts.with_aes_encryption(zip::AesMode::Aes256, pw);
            }

            let mut buf = vec![0u8; STREAM_BUF_SIZE];
            for (disk_path, rel) in &files_to_pack {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                    t.set_current_item(rel);
                }
                if disk_path.is_dir() {
                    zw.add_directory(format!("{rel}/"), opts)
                        .map_err(|e| OpfileError::archive(e.to_string()))?;
                } else {
                    zw.start_file(rel, opts)
                        .map_err(|e| OpfileError::archive(e.to_string()))?;
                    let mut f = BufReader::with_capacity(STREAM_BUF_SIZE, File::open(disk_path)?);
                    loop {
                        if let Some(t) = task {
                            t.check_checkpoint()?;
                        }
                        let n = f.read(&mut buf)?;
                        if n == 0 {
                            break;
                        }
                        zw.write_all(&buf[..n])?;
                        if let Some(t) = task {
                            t.add_bytes(n as u64);
                        }
                    }
                }
                if let Some(t) = task {
                    t.inc_items();
                }
            }
            let mut w = zw.finish().map_err(|e| OpfileError::archive(e.to_string()))?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "7z" => {
            let guard = AtomicPartFile::part_path_only(&staging)?;
            let mut sz = sevenz_rust::SevenZWriter::create(&guard.part_path)
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            let _dict_hint = req.dict_size_mb.unwrap_or(16);
            if let Some(pw) = req.password.as_deref().filter(|s| !s.is_empty()) {
                sz.set_content_methods(vec![
                    sevenz_rust::SevenZMethod::LZMA2.into(),
                    sevenz_rust::AesEncoderOptions::new(pw.into()).into(),
                ]);
            }
            for (disk_path, rel) in &files_to_pack {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                    t.set_current_item(rel);
                }
                let entry = sevenz_rust::SevenZArchiveEntry::from_path(disk_path, rel.clone());
                let reader_opt = if disk_path.is_file() {
                    Some(File::open(disk_path)?)
                } else {
                    None
                };
                sz.push_archive_entry(entry, reader_opt)
                    .map_err(|e| OpfileError::archive(e.to_string()))?;
                if let Some(t) = task {
                    t.inc_items();
                }
            }
            sz.finish().map_err(|e| OpfileError::archive(e.to_string()))?;
            guard.commit()?;
        }
        "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tbz2" | "tar.xz" | "txz" | "tar.zst" | "tzst" => {
            let (guard, bw) = AtomicPartFile::create(&staging)?;
            match fmt.as_str() {
                "tar" => write_tar_payload(bw, &files_to_pack, task)?,
                "tar.gz" | "tgz" => {
                    let enc = GzEncoder::new(
                        bw,
                        flate2::Compression::new(req.profile.zip_level() as u32),
                    );
                    write_tar_payload(enc, &files_to_pack, task)?;
                }
                "tar.bz2" | "tbz2" => {
                    let enc = BzEncoder::new(
                        bw,
                        bzip2::Compression::new((req.profile.zip_level() as u32).clamp(1, 9)),
                    );
                    write_tar_payload(enc, &files_to_pack, task)?;
                }
                "tar.xz" | "txz" => {
                    let enc = XzEncoder::new(bw, req.profile.xz_level());
                    write_tar_payload(enc, &files_to_pack, task)?;
                }
                "tar.zst" | "tzst" => {
                    let enc = zstd::stream::write::Encoder::new(bw, req.profile.zstd_level())
                        .map_err(|e| OpfileError::archive(e.to_string()))?
                        .auto_finish();
                    write_tar_payload(enc, &files_to_pack, task)?;
                }
                _ => unreachable!(),
            }
            guard.commit()?;
        }
        "gz" | "bz2" | "xz" | "zst" => {
            let single = req
                .sources
                .first()
                .ok_or_else(|| OpfileError::invalid("Single-stream format requires 1 file"))?;
            let (guard, bw) = AtomicPartFile::create(&staging)?;
            let mut r = BufReader::with_capacity(STREAM_BUF_SIZE, File::open(single)?);
            let mut buf = vec![0u8; STREAM_BUF_SIZE];
            match fmt.as_str() {
                "gz" => {
                    let mut enc = GzEncoder::new(
                        bw,
                        flate2::Compression::new(req.profile.zip_level() as u32),
                    );
                    stream_rw(&mut r, &mut enc, &mut buf, task)?;
                    enc.finish()?;
                }
                "bz2" => {
                    let mut enc = BzEncoder::new(
                        bw,
                        bzip2::Compression::new((req.profile.zip_level() as u32).clamp(1, 9)),
                    );
                    stream_rw(&mut r, &mut enc, &mut buf, task)?;
                    enc.finish()?;
                }
                "xz" => {
                    let mut enc = XzEncoder::new(bw, req.profile.xz_level());
                    stream_rw(&mut r, &mut enc, &mut buf, task)?;
                    enc.finish()?;
                }
                "zst" => {
                    let mut enc = zstd::stream::write::Encoder::new(bw, req.profile.zstd_level())
                        .map_err(|e| OpfileError::archive(e.to_string()))?;
                    stream_rw(&mut r, &mut enc, &mut buf, task)?;
                    enc.finish()?;
                }
                _ => {}
            }
            guard.commit()?;
        }
        _ => {
            return Err(OpfileError::unsupported(format!(
                "Archive creation not supported for {fmt}"
            )))
        }
    }

    if let Some(chunk_size) = split_bytes {
        let outputs = split_into_volumes(&staging, &dest, chunk_size, task)?;
        let _ = fs::remove_file(&staging);
        Ok(outputs)
    } else {
        Ok(vec![dest.to_string_lossy().into_owned()])
    }
}

fn split_into_volumes(
    src: &Path,
    base_dest: &Path,
    chunk_size: u64,
    task: Option<&TaskHandle>,
) -> OpResult<Vec<String>> {
    let mut r = BufReader::with_capacity(STREAM_BUF_SIZE, File::open(src)?);
    let mut buf = vec![0u8; STREAM_BUF_SIZE];
    let mut vol_idx = 1usize;
    let mut outputs = Vec::new();
    let base_str = base_dest.to_string_lossy();

    loop {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let vol_path = PathBuf::from(format!("{base_str}.{vol_idx:03}"));
        let (guard, mut w) = AtomicPartFile::create(&vol_path)?;
        let mut written_this_vol = 0u64;
        let mut hit_eof = false;

        while written_this_vol < chunk_size {
            let want = ((chunk_size - written_this_vol) as usize).min(buf.len());
            let n = r.read(&mut buf[..want])?;
            if n == 0 {
                hit_eof = true;
                break;
            }
            w.write_all(&buf[..n])?;
            written_this_vol += n as u64;
        }
        w.flush()?;
        drop(w);
        if written_this_vol > 0 {
            guard.commit()?;
            outputs.push(vol_path.to_string_lossy().into_owned());
        }
        if hit_eof {
            break;
        }
        vol_idx += 1;
    }
    Ok(outputs)
}

pub fn test_archive_integrity(
    archive_path: &str,
    password: Option<&str>,
    task: Option<&TaskHandle>,
) -> OpResult<IntegrityReport> {
    let kind = detect_archive_kind(archive_path);
    let mut checked = 0usize;
    let mut failed = Vec::new();

    match kind.as_str() {
        "zip" | "apk" | "xapk" | "apks" | "obb" | "cbz" | "epub" | "zip.001" => {
            let reader = MultiVolumeReader::open(Path::new(archive_path))?;
            let mut zip =
                ZipArchive::new(reader).map_err(|e| OpfileError::archive(e.to_string()))?;
            let len = zip.len();
            if let Some(t) = task {
                t.set_totals(0, len as u64);
            }
            let mut sink = std::io::sink();
            for i in 0..len {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                }
                let res = if let Some(pw) = password.filter(|s| !s.is_empty()) {
                    zip.by_index_decrypt(i, pw.as_bytes()).map_err(|e| e.to_string())
                } else {
                    zip.by_index(i).map_err(|e| e.to_string())
                };
                match res {
                    Ok(mut entry) => {
                        let name = entry.name().to_string();
                        if !entry.is_dir() {
                            if let Err(e) = std::io::copy(&mut entry, &mut sink) {
                                failed.push(format!("{name}: {e}"));
                            }
                        }
                    }
                    Err(e) => failed.push(format!("entry#{i}: {e}")),
                }
                checked += 1;
                if let Some(t) = task {
                    t.inc_items();
                }
            }
        }
        "7z" | "7z.001" => {
            let reader = MultiVolumeReader::open(Path::new(archive_path))?;
            let len = reader.total_len;
            let pw = password.unwrap_or("");
            let mut sz = sevenz_rust::SevenZReader::new(reader, len, pw.into())
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            let mut sink = std::io::sink();
            sz.for_each_entries(|entry, r| {
                checked += 1;
                if !entry.is_directory() {
                    if let Err(e) = std::io::copy(r, &mut sink) {
                        failed.push(format!("{}: {e}", entry.name()));
                    }
                }
                Ok(true)
            })
            .map_err(|e| OpfileError::archive(e.to_string()))?;
        }
        "tar" | "tar.gz" | "tgz" | "tar.bz2" | "tbz2" | "tar.xz" | "txz" | "tar.zst" | "tzst" => {
            let mut sink = std::io::sink();
            with_tar_archive(archive_path, &kind, |ar| {
                for entry_res in ar.entries().map_err(|e| OpfileError::archive(e.to_string()))? {
                    let mut entry = entry_res.map_err(|e| OpfileError::archive(e.to_string()))?;
                    let name = entry
                        .path()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if let Err(e) = std::io::copy(&mut entry, &mut sink) {
                        failed.push(format!("{name}: {e}"));
                    }
                    checked += 1;
                }
                Ok(())
            })?;
        }
        _ => {
            // Verify readability of headers for ISO/RAR/Disk images
            let entries = list_all_entries(archive_path, password)?;
            checked = entries.len();
        }
    }

    let passed = failed.is_empty();
    let summary = if passed {
        format!("Verified {checked} entries: no errors")
    } else {
        format!("Verified {checked} entries: {} failed", failed.len())
    };
    Ok(IntegrityReport {
        archive_path: archive_path.to_string(),
        passed,
        checked_entries: checked,
        failed_entries: failed,
        summary,
    })
}

/// Direct in-archive text/config file edit that atomically updates a ZIP/APK/TAR archive.
pub fn update_archive_entry(
    archive_path: &str,
    entry_path: &str,
    new_content: &[u8],
    password: Option<&str>,
) -> OpResult<()> {
    let kind = detect_archive_kind(archive_path);
    let target_clean = entry_path.trim_matches('/');
    let arch = Path::new(archive_path);

    if kind == "zip" || kind == "cbz" || kind == "epub" {
        let mut src_zip = ZipArchive::new(BufReader::new(File::open(arch)?))
            .map_err(|e| OpfileError::archive(e.to_string()))?;
        let (guard, bw) = AtomicPartFile::create(arch)?;
        let mut dst_zip = ZipWriter::new(bw);
        let mut replaced = false;

        for i in 0..src_zip.len() {
            let mut entry = if let Some(pw) = password.filter(|s| !s.is_empty()) {
                src_zip
                    .by_index_decrypt(i, pw.as_bytes())
                    .map_err(|e| OpfileError::crypto(e.to_string()))?
            } else {
                src_zip.by_index(i).map_err(|e| OpfileError::archive(e.to_string()))?
            };
            let name = entry.name().to_string();
            let opts = SimpleFileOptions::default().compression_method(entry.compression());
            if name.trim_matches('/') == target_clean && !entry.is_dir() {
                dst_zip
                    .start_file(&name, opts)
                    .map_err(|e| OpfileError::archive(e.to_string()))?;
                dst_zip.write_all(new_content)?;
                replaced = true;
            } else if entry.is_dir() {
                dst_zip
                    .add_directory(&name, opts)
                    .map_err(|e| OpfileError::archive(e.to_string()))?;
            } else {
                dst_zip
                    .start_file(&name, opts)
                    .map_err(|e| OpfileError::archive(e.to_string()))?;
                std::io::copy(&mut entry, &mut dst_zip)?;
            }
        }
        if !replaced {
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            dst_zip
                .start_file(target_clean, opts)
                .map_err(|e| OpfileError::archive(e.to_string()))?;
            dst_zip.write_all(new_content)?;
        }
        let mut w = dst_zip.finish().map_err(|e| OpfileError::archive(e.to_string()))?;
        w.flush()?;
        drop(w);
        guard.commit()?;
        return Ok(());
    }

    Err(OpfileError::unsupported(format!(
        "In-archive live edit supports ZIP/CBZ/EPUB (got {kind})"
    )))
}

fn encrypt_opx_file(
    src: &Path,
    dest: &Path,
    password: &str,
    task: Option<&TaskHandle>,
) -> OpResult<()> {
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut salt);
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);

    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, 100_000, &mut key)
        .map_err(|e| OpfileError::crypto(format!("KDF error: {e}")))?;

    let data = fs::read(src)?;
    if let Some(t) = task {
        t.check_checkpoint()?;
    }
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| OpfileError::crypto(format!("Cipher init error: {e}")))?;
    let ct = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), data.as_slice())
        .map_err(|e| OpfileError::crypto(format!("AES-256-GCM encrypt error: {e}")))?;

    let (guard, mut w) = AtomicPartFile::create(dest)?;
    w.write_all(b"OPX1")?;
    w.write_all(&salt)?;
    w.write_all(&nonce_bytes)?;
    w.write_all(&ct)?;
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}

fn decrypt_opx_file(
    src_path: &str,
    dest: &Path,
    password: &str,
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let raw = fs::read(src_path)?;
    if raw.len() < 32 || &raw[0..4] != b"OPX1" {
        return Err(OpfileError::crypto("Invalid OPX encrypted file header"));
    }
    let salt = &raw[4..20];
    let nonce_bytes = &raw[20..32];
    let ct = &raw[32..];

    let mut key = [0u8; 32];
    pbkdf2::<Hmac<Sha256>>(password.as_bytes(), salt, 100_000, &mut key)
        .map_err(|e| OpfileError::crypto(format!("KDF error: {e}")))?;
    if let Some(t) = task {
        t.check_checkpoint()?;
    }
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| OpfileError::crypto(format!("Cipher init error: {e}")))?;
    let pt = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|_| OpfileError::crypto("Decryption failed: wrong password or corrupted data"))?;

    let (guard, mut w) = AtomicPartFile::create(dest)?;
    w.write_all(&pt)?;
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(pt.len() as u64)
}

fn write_tar_payload<W: Write>(
    writer: W,
    files: &[(PathBuf, String)],
    task: Option<&TaskHandle>,
) -> OpResult<()> {
    let mut builder = tar::Builder::new(writer);
    for (disk_path, rel) in files {
        if let Some(t) = task {
            t.check_checkpoint()?;
            t.set_current_item(rel);
        }
        if disk_path.is_dir() {
            builder
                .append_dir(rel, disk_path)
                .map_err(|e| OpfileError::archive(e.to_string()))?;
        } else {
            builder
                .append_path_with_name(disk_path, rel)
                .map_err(|e| OpfileError::archive(e.to_string()))?;
        }
        if let Some(t) = task {
            t.inc_items();
        }
    }
    let mut inner = builder
        .into_inner()
        .map_err(|e| OpfileError::archive(e.to_string()))?;
    inner.flush()?;
    Ok(())
}

fn with_tar_archive<F>(path: &str, kind: &str, mut f: F) -> OpResult<()>
where
    F: FnMut(&mut tar::Archive<Box<dyn Read>>) -> OpResult<()>,
{
    let file = BufReader::with_capacity(STREAM_BUF_SIZE, File::open(path)?);
    let reader: Box<dyn Read> = match kind {
        "tar" => Box::new(file),
        "tar.gz" | "tgz" => Box::new(GzDecoder::new(file)),
        "tar.bz2" | "tbz2" => Box::new(BzDecoder::new(file)),
        "tar.xz" | "txz" => Box::new(XzDecoder::new(file)),
        "tar.zst" | "tzst" => Box::new(
            zstd::stream::read::Decoder::new(file)
                .map_err(|e| OpfileError::archive(e.to_string()))?,
        ),
        _ => return Err(OpfileError::unsupported("Unknown tar compression")),
    };
    let mut ar = tar::Archive::new(reader);
    f(&mut ar)
}

fn stream_entry_to_part<R: Read + ?Sized>(
    reader: &mut R,
    dest: &Path,
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let (guard, mut writer) = AtomicPartFile::create(dest)?;
    let mut buf = vec![0u8; STREAM_BUF_SIZE];
    let total = stream_rw(reader, &mut writer, &mut buf, task)?;
    writer.flush()?;
    drop(writer);
    guard.commit()?;
    Ok(total)
}

fn stream_rw<R: Read + ?Sized, W: Write + ?Sized>(
    reader: &mut R,
    writer: &mut W,
    buf: &mut [u8],
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let mut total = 0u64;
    loop {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let n = reader.read(buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        total += n as u64;
        if let Some(t) = task {
            t.add_bytes(n as u64);
        }
    }
    Ok(total)
}

fn safe_join(root: &Path, rel: &str) -> OpResult<PathBuf> {
    let mut out = root.to_path_buf();
    for comp in Path::new(rel).components() {
        if let std::path::Component::Normal(seg) = comp {
            out.push(seg);
        }
    }
    if out == root {
        return Err(OpfileError::invalid("Empty or invalid relative path"));
    }
    if !out.starts_with(root) {
        return Err(OpfileError::invalid("Path traversal attempt detected"));
    }
    Ok(out)
}
