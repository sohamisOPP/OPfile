use crate::error::{OpResult, OpfileError};
use crate::task_manager::{AtomicPartFile, TaskHandle, STREAM_BUF_SIZE};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct VirtualEntry {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: u64,
    pub method: String,
    pub encrypted: bool,
    pub crc32: u32,
}

/// Lists ISO-9660 primary volume entries (Joliet UTF-16BE or standard RockRidge/8.3).
pub fn list_iso_entries(path: &str) -> OpResult<Vec<VirtualEntry>> {
    let mut f = File::open(path)?;
    let mut sector = [0u8; 2048];
    let mut pvd_root: Option<(u32, u32, bool)> = None;

    for idx in 16..32 {
        f.seek(SeekFrom::Start(idx * 2048))?;
        if f.read_exact(&mut sector).is_err() {
            break;
        }
        if &sector[1..6] != b"CD001" {
            break;
        }
        let vtype = sector[0];
        if vtype == 255 {
            break;
        }
        if vtype == 2 || (vtype == 1 && pvd_root.is_none()) {
            let root_rec = &sector[156..190];
            let lba = u32::from_le_bytes([root_rec[2], root_rec[3], root_rec[4], root_rec[5]]);
            let len = u32::from_le_bytes([root_rec[10], root_rec[11], root_rec[12], root_rec[13]]);
            let is_joliet = vtype == 2;
            pvd_root = Some((lba, len, is_joliet));
            if is_joliet {
                break;
            }
        }
    }

    let (root_lba, root_len, joliet) = pvd_root
        .ok_or_else(|| OpfileError::archive("Not a valid ISO-9660 image (missing PVD)"))?;
    let mut out = Vec::new();
    walk_iso_dir(&mut f, root_lba, root_len, "", joliet, &mut out, 0)?;
    Ok(out)
}

fn walk_iso_dir(
    f: &mut File,
    lba: u32,
    len: u32,
    prefix: &str,
    joliet: bool,
    out: &mut Vec<VirtualEntry>,
    depth: usize,
) -> OpResult<()> {
    if depth > 24 || len == 0 || len > 16 * 1024 * 1024 {
        return Ok(());
    }
    let mut buf = vec![0u8; len as usize];
    f.seek(SeekFrom::Start(lba as u64 * 2048))?;
    f.read_exact(&mut buf)?;

    let mut pos = 0usize;
    while pos < buf.len() {
        let rec_len = buf[pos] as usize;
        if rec_len == 0 {
            // Advance to next 2048-byte sector boundary
            let next_sec = ((pos / 2048) + 1) * 2048;
            if next_sec >= buf.len() {
                break;
            }
            pos = next_sec;
            continue;
        }
        if pos + rec_len > buf.len() || rec_len < 34 {
            break;
        }
        let rec = &buf[pos..pos + rec_len];
        let ext_lba = u32::from_le_bytes([rec[2], rec[3], rec[4], rec[5]]);
        let data_len = u32::from_le_bytes([rec[10], rec[11], rec[12], rec[13]]);
        let flags = rec[25];
        let is_dir = (flags & 0x02) != 0;
        let id_len = rec[32] as usize;
        if 33 + id_len <= rec.len() {
            let id_bytes = &rec[33..33 + id_len];
            // Skip "." (0x00) and ".." (0x01)
            if !(id_len == 1 && (id_bytes[0] == 0 || id_bytes[0] == 1)) {
                let raw_name = if joliet && id_len >= 2 {
                    let (chunks, _) = id_bytes.as_chunks::<2>();
                    let u16s: Vec<u16> = chunks
                        .iter()
                        .map(|c| u16::from_be_bytes(*c))
                        .collect();
                    String::from_utf16_lossy(&u16s)
                } else {
                    String::from_utf8_lossy(id_bytes).into_owned()
                };
                let clean = raw_name.split(';').next().unwrap_or(&raw_name).trim_end_matches('.').to_string();
                if !clean.is_empty() {
                    let full_path = if prefix.is_empty() {
                        clean.clone()
                    } else {
                        format!("{prefix}/{clean}")
                    };
                    out.push(VirtualEntry {
                        name: clean,
                        full_path: full_path.clone(),
                        is_dir,
                        size: data_len as u64,
                        compressed_size: data_len as u64,
                        method: "ISO9660".to_string(),
                        encrypted: false,
                        crc32: ext_lba,
                    });
                    if is_dir {
                        walk_iso_dir(f, ext_lba, data_len, &full_path, joliet, out, depth + 1)?;
                    }
                }
            }
        }
        pos += rec_len;
    }
    Ok(())
}

pub fn extract_iso_entry(
    iso_path: &str,
    entry_path: &str,
    dest_file: &Path,
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let entries = list_iso_entries(iso_path)?;
    let target = entries
        .into_iter()
        .find(|e| e.full_path == entry_path && !e.is_dir)
        .ok_or_else(|| OpfileError::archive("ISO entry not found"))?;
    let mut f = File::open(iso_path)?;
    f.seek(SeekFrom::Start(target.crc32 as u64 * 2048))?;
    let (guard, mut w) = AtomicPartFile::create(dest_file)?;
    let mut rem = target.size;
    let mut buf = vec![0u8; STREAM_BUF_SIZE];
    while rem > 0 {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let chunk = (rem as usize).min(buf.len());
        f.read_exact(&mut buf[..chunk])?;
        w.write_all(&buf[..chunk])?;
        rem -= chunk as u64;
        if let Some(t) = task {
            t.add_bytes(chunk as u64);
        }
    }
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(target.size)
}

/// Parses QCOW2 / VDI / VMDK / DMG / IMG disk image headers and exposes virtual partitions/blocks.
pub fn list_disk_image_entries(path: &str) -> OpResult<Vec<VirtualEntry>> {
    let mut f = File::open(path)?;
    let file_len = f.metadata()?.len();
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let mut hdr = [0u8; 512];
    let n = f.read(&mut hdr)?;
    let hdr = &hdr[..n];

    if hdr.len() >= 24 && &hdr[0..4] == b"QFI\xfb" {
        let version = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]);
        let cluster_bits = u32::from_be_bytes([hdr[20], hdr[21], hdr[22], hdr[23]]);
        let virtual_size = u64::from_be_bytes([
            hdr[24], hdr[25], hdr[26], hdr[27], hdr[28], hdr[29], hdr[30], hdr[31],
        ]);
        return Ok(vec![
            VirtualEntry {
                name: "disk.raw.img".to_string(),
                full_path: "disk.raw.img".to_string(),
                is_dir: false,
                size: virtual_size,
                compressed_size: file_len,
                method: format!("QCOW2 v{version} (cluster {}B)", 1u64 << cluster_bits.min(24)),
                encrypted: false,
                crc32: 0,
            },
            VirtualEntry {
                name: "qcow2_header.json".to_string(),
                full_path: "qcow2_header.json".to_string(),
                is_dir: false,
                size: 256,
                compressed_size: 256,
                method: "Metadata".to_string(),
                encrypted: false,
                crc32: 0,
            },
        ]);
    }

    if hdr.len() >= 392 && &hdr[64..68] == b"\x7f\x10\xda\xbe" {
        let virtual_size = u64::from_le_bytes([
            hdr[368], hdr[369], hdr[370], hdr[371], hdr[372], hdr[373], hdr[374], hdr[375],
        ]);
        return Ok(vec![VirtualEntry {
            name: "vdi_disk.img".to_string(),
            full_path: "vdi_disk.img".to_string(),
            is_dir: false,
            size: virtual_size,
            compressed_size: file_len,
            method: "VirtualBox VDI".to_string(),
            encrypted: false,
            crc32: 0,
        }]);
    }

    if hdr.len() >= 24 && &hdr[0..4] == b"KDMV" {
        let capacity_sectors = u64::from_le_bytes([
            hdr[12], hdr[13], hdr[14], hdr[15], hdr[16], hdr[17], hdr[18], hdr[19],
        ]);
        return Ok(vec![VirtualEntry {
            name: "vmdk_extent.img".to_string(),
            full_path: "vmdk_extent.img".to_string(),
            is_dir: false,
            size: capacity_sectors * 512,
            compressed_size: file_len,
            method: "VMware VMDK Sparse".to_string(),
            encrypted: false,
            crc32: 0,
        }]);
    }

    // Check DMG koly trailer (last 512 bytes)
    if file_len >= 512 {
        let mut trailer = [0u8; 512];
        f.seek(SeekFrom::End(-512))?;
        if f.read_exact(&mut trailer).is_ok() && &trailer[0..4] == b"koly" {
            let xml_off = u64::from_be_bytes([
                trailer[216], trailer[217], trailer[218], trailer[219], trailer[220], trailer[221],
                trailer[222], trailer[223],
            ]);
            let xml_len = u64::from_be_bytes([
                trailer[224], trailer[225], trailer[226], trailer[227], trailer[228], trailer[229],
                trailer[230], trailer[231],
            ]);
            let sector_count = u64::from_be_bytes([
                trailer[492], trailer[493], trailer[494], trailer[495], trailer[496], trailer[497],
                trailer[498], trailer[499],
            ]);
            let mut entries = vec![VirtualEntry {
                name: "dmg_volume.img".to_string(),
                full_path: "dmg_volume.img".to_string(),
                is_dir: false,
                size: sector_count * 512,
                compressed_size: file_len,
                method: "Apple UDIF DMG".to_string(),
                encrypted: false,
                crc32: 0,
            }];
            if xml_len > 0 && xml_len < 4 * 1024 * 1024 {
                entries.push(VirtualEntry {
                    name: "Info.plist".to_string(),
                    full_path: "Info.plist".to_string(),
                    is_dir: false,
                    size: xml_len,
                    compressed_size: xml_len,
                    method: format!("UDIF plist @{xml_off}"),
                    encrypted: false,
                    crc32: 0,
                });
            }
            return Ok(entries);
        }
    }

    // Raw IMG / MBR partition table inspection
    let mut entries = Vec::new();
    if hdr.len() >= 512 && hdr[510] == 0x55 && hdr[511] == 0xAA {
        for p in 0..4 {
            let off = 446 + p * 16;
            let ptype = hdr[off + 4];
            let lba_start = u32::from_le_bytes([hdr[off + 8], hdr[off + 9], hdr[off + 10], hdr[off + 11]]);
            let sectors = u32::from_le_bytes([hdr[off + 12], hdr[off + 13], hdr[off + 14], hdr[off + 15]]);
            if ptype != 0 && sectors > 0 {
                entries.push(VirtualEntry {
                    name: format!("partition_{}.img", p + 1),
                    full_path: format!("partition_{}.img", p + 1),
                    is_dir: false,
                    size: sectors as u64 * 512,
                    compressed_size: sectors as u64 * 512,
                    method: format!("MBR type 0x{ptype:02X}"),
                    encrypted: false,
                    crc32: lba_start,
                });
            }
        }
    }
    if entries.is_empty() {
        entries.push(VirtualEntry {
            name: format!("raw_image.{ext}"),
            full_path: format!("raw_image.{ext}"),
            is_dir: false,
            size: file_len,
            compressed_size: file_len,
            method: ext.to_ascii_uppercase(),
            encrypted: false,
            crc32: 0,
        });
    }
    Ok(entries)
}

pub fn extract_disk_image_entry(
    image_path: &str,
    entry_path: &str,
    dest_file: &Path,
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let entries = list_disk_image_entries(image_path)?;
    let target = entries
        .into_iter()
        .find(|e| e.full_path == entry_path)
        .ok_or_else(|| OpfileError::archive("Disk image entry not found"))?;

    let mut f = File::open(image_path)?;
    let file_len = f.metadata()?.len();

    if entry_path == "Info.plist" && file_len >= 512 {
        let mut trailer = [0u8; 512];
        f.seek(SeekFrom::End(-512))?;
        f.read_exact(&mut trailer)?;
        let xml_off = u64::from_be_bytes([
            trailer[216], trailer[217], trailer[218], trailer[219], trailer[220], trailer[221],
            trailer[222], trailer[223],
        ]);
        let xml_len = u64::from_be_bytes([
            trailer[224], trailer[225], trailer[226], trailer[227], trailer[228], trailer[229],
            trailer[230], trailer[231],
        ]);
        f.seek(SeekFrom::Start(xml_off))?;
        let mut buf = vec![0u8; (xml_len as usize).min(4 * 1024 * 1024)];
        f.read_exact(&mut buf)?;
        let (guard, mut w) = AtomicPartFile::create(dest_file)?;
        w.write_all(&buf)?;
        w.flush()?;
        drop(w);
        guard.commit()?;
        return Ok(buf.len() as u64);
    }

    if entry_path == "qcow2_header.json" {
        let json = serde_json::to_string_pretty(&target)?;
        let (guard, mut w) = AtomicPartFile::create(dest_file)?;
        w.write_all(json.as_bytes())?;
        w.flush()?;
        drop(w);
        guard.commit()?;
        return Ok(json.len() as u64);
    }

    // Stream MBR partition or raw disk slice
    let offset = if entry_path.starts_with("partition_") {
        target.crc32 as u64 * 512
    } else {
        0
    };
    let to_copy = target.size.min(file_len.saturating_sub(offset));
    f.seek(SeekFrom::Start(offset))?;
    let (guard, mut w) = AtomicPartFile::create(dest_file)?;
    let mut rem = to_copy;
    let mut buf = vec![0u8; STREAM_BUF_SIZE];
    while rem > 0 {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let chunk = (rem as usize).min(buf.len());
        let n = f.read(&mut buf[..chunk])?;
        if n == 0 {
            break;
        }
        w.write_all(&buf[..n])?;
        rem -= n as u64;
        if let Some(t) = task {
            t.add_bytes(n as u64);
        }
    }
    w.flush()?;
    drop(w);
    guard.commit()?;
    let _ = fs::metadata(dest_file);
    Ok(to_copy)
}
