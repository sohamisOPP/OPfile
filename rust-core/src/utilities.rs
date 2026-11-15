use crate::error::{OpResult, OpfileError};
use crate::task_manager::{TaskHandle, STREAM_BUF_SIZE};
use base64::Engine;
use md5::{Digest as Md5Digest, Md5};
use memmap2::MmapOptions;
use qrcode::QrCode;
use rand::RngCore;
use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Sha512};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Serialize)]
pub struct ChecksumReport {
    pub path: String,
    pub size: u64,
    pub blake3: String,
    pub md5: String,
    pub sha256: String,
    pub sha512: String,
}

pub fn compute_checksums(path: &str, task: Option<&TaskHandle>) -> OpResult<ChecksumReport> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();
    if let Some(t) = task {
        t.set_totals(size, 1);
        t.set_current_item(
            Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("checksum"),
        );
    }

    let mut reader = BufReader::with_capacity(STREAM_BUF_SIZE, file);
    let mut b3 = blake3::Hasher::new();
    let mut m5 = Md5::new();
    let mut s256 = Sha256::new();
    let mut s512 = Sha512::new();
    let mut buf = vec![0u8; STREAM_BUF_SIZE];

    loop {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let slice = &buf[..n];
        b3.update(slice);
        m5.update(slice);
        s256.update(slice);
        s512.update(slice);
        if let Some(t) = task {
            t.add_bytes(n as u64);
        }
    }
    if let Some(t) = task {
        t.inc_items();
    }

    Ok(ChecksumReport {
        path: path.to_string(),
        size,
        blake3: b3.finalize().to_hex().to_string(),
        md5: hex::encode(m5.finalize()),
        sha256: hex::encode(s256.finalize()),
        sha512: hex::encode(s512.finalize()),
    })
}

/// DoD 5220.22-M 3-pass file shredder (0x00, 0xFF, CSPRNG random) + name obfuscation + unlink.
pub fn shred_paths_dod(paths: &[String], task: Option<&TaskHandle>) -> OpResult<usize> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    for p_str in paths {
        let p = PathBuf::from(p_str);
        if p.is_dir() {
            for e in WalkDir::new(&p).contents_first(true).into_iter().filter_map(Result::ok) {
                if e.file_type().is_dir() {
                    dirs.push(e.path().to_path_buf());
                } else {
                    files.push(e.path().to_path_buf());
                }
            }
        } else if p.exists() {
            files.push(p);
        }
    }

    let total_bytes: u64 = files
        .iter()
        .map(|f| f.metadata().map(|m| m.len() * 3).unwrap_or(0))
        .sum();
    if let Some(t) = task {
        t.set_totals(total_bytes, (files.len() + dirs.len()) as u64);
    }

    let mut wiped = 0usize;
    for f in &files {
        shred_single_file(f, task)?;
        wiped += 1;
        if let Some(t) = task {
            t.inc_items();
        }
    }
    for d in &dirs {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let _ = fs::remove_dir(d);
        if let Some(t) = task {
            t.inc_items();
        }
    }
    Ok(wiped)
}

fn shred_single_file(path: &Path, task: Option<&TaskHandle>) -> OpResult<()> {
    if let Some(t) = task {
        t.set_current_item(path.file_name().and_then(|n| n.to_str()).unwrap_or("shred"));
    }
    let len = path.metadata()?.len();
    {
        let mut file = OpenOptions::new().write(true).open(path)?;
        let mut buf = vec![0u8; STREAM_BUF_SIZE];

        // Pass 1: 0x00, Pass 2: 0xFF, Pass 3: Random CSPRNG
        for pass in 0..3 {
            file.seek(SeekFrom::Start(0))?;
            let mut remaining = len;
            while remaining > 0 {
                if let Some(t) = task {
                    t.check_checkpoint()?;
                }
                let chunk = (remaining as usize).min(buf.len());
                match pass {
                    0 => buf[..chunk].fill(0x00),
                    1 => buf[..chunk].fill(0xFF),
                    _ => rand::rngs::OsRng.fill_bytes(&mut buf[..chunk]),
                }
                file.write_all(&buf[..chunk])?;
                remaining -= chunk as u64;
                if let Some(t) = task {
                    t.add_bytes(chunk as u64);
                }
            }
            file.flush()?;
            file.sync_all()?;
        }
        file.set_len(0)?;
        file.sync_all()?;
    }

    // Rename to random obfuscated filename before unlink
    let mut rnd = [0u8; 8];
    rand::rngs::OsRng.fill_bytes(&mut rnd);
    let obf = path.with_file_name(format!(".shred_{}", hex::encode(rnd)));
    if fs::rename(path, &obf).is_ok() {
        let _ = fs::remove_file(&obf);
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[derive(Serialize)]
pub struct HexWindow {
    pub path: String,
    pub total_size: u64,
    pub offset: u64,
    pub length: usize,
    pub bytes_base64: String,
}

pub fn read_hex_window(path: &str, offset: u64, length: usize) -> OpResult<HexWindow> {
    let file = File::open(path)?;
    let total_size = file.metadata()?.len();
    if offset >= total_size || length == 0 || total_size == 0 {
        return Ok(HexWindow {
            path: path.to_string(),
            total_size,
            offset,
            length: 0,
            bytes_base64: String::new(),
        });
    }
    let clamped_len = (length as u64).min(total_size - offset).min(64 * 1024) as usize;
    let mmap = unsafe { MmapOptions::new().map(&file) }
        .map_err(|e| OpfileError::io(format!("mmap failed: {e}")))?;
    let start = offset as usize;
    let end = start + clamped_len;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&mmap[start..end]);
    Ok(HexWindow {
        path: path.to_string(),
        total_size,
        offset,
        length: clamped_len,
        bytes_base64: b64,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct TreemapNode {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub depth: u32,
}

pub fn analyze_cushion_treemap(
    root: &str,
    width: f32,
    height: f32,
    task: Option<&TaskHandle>,
) -> OpResult<Vec<TreemapNode>> {
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(OpfileError::new(
            crate::error::ErrorCode::NotFound,
            "Root path not found",
        ));
    }

    let entries: Vec<PathBuf> = fs::read_dir(root_path)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();

    if let Some(t) = task {
        t.set_totals(0, entries.len() as u64);
    }

    let mut children: Vec<(String, String, u64, bool)> = entries
        .par_iter()
        .filter_map(|p| {
            if let Some(t) = task {
                if t.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                    return None;
                }
            }
            let name = p.file_name()?.to_string_lossy().into_owned();
            let meta = fs::symlink_metadata(p).ok()?;
            let size = if meta.is_dir() {
                WalkDir::new(p)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter_map(|e| e.metadata().ok())
                    .filter(|m| m.is_file())
                    .map(|m| m.len())
                    .sum::<u64>()
            } else {
                meta.len()
            };
            if let Some(t) = task {
                t.inc_items();
            }
            Some((name, p.to_string_lossy().into_owned(), size.max(1), meta.is_dir()))
        })
        .collect();

    children.sort_by(|a, b| b.2.cmp(&a.2));
    children.truncate(48);

    let total: u64 = children.iter().map(|c| c.2).sum::<u64>().max(1);
    let mut rects = Vec::with_capacity(children.len());
    squarify_layout(
        &children,
        total,
        0.0,
        0.0,
        width.max(1.0),
        height.max(1.0),
        1,
        &mut rects,
    );
    Ok(rects)
}

#[allow(clippy::too_many_arguments)]
fn squarify_layout(
    items: &[(String, String, u64, bool)],
    total_size: u64,
    mut x: f32,
    mut y: f32,
    mut w: f32,
    mut h: f32,
    depth: u32,
    out: &mut Vec<TreemapNode>,
) {
    let mut rem_size = total_size as f64;
    for (name, path, size, is_dir) in items {
        if w <= 2.0 || h <= 2.0 || rem_size <= 0.0 {
            break;
        }
        let frac = ((*size as f64) / rem_size).clamp(0.0, 1.0) as f32;
        if w >= h {
            let slice_w = (w * frac).max(1.0);
            out.push(TreemapNode {
                name: name.clone(),
                path: path.clone(),
                size: *size,
                is_dir: *is_dir,
                x,
                y,
                w: slice_w,
                h,
                depth,
            });
            x += slice_w;
            w = (w - slice_w).max(0.0);
        } else {
            let slice_h = (h * frac).max(1.0);
            out.push(TreemapNode {
                name: name.clone(),
                path: path.clone(),
                size: *size,
                is_dir: *is_dir,
                x,
                y,
                w,
                h: slice_h,
                depth,
            });
            y += slice_h;
            h = (h - slice_h).max(0.0);
        }
        rem_size -= *size as f64;
    }
}

#[derive(Deserialize)]
pub struct BatchRenameRequest {
    pub paths: Vec<String>,
    pub pattern: String,
    pub replacement: String,
    pub use_regex: bool,
    pub start_index: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RenameOp {
    pub old_path: String,
    pub new_path: String,
    pub old_name: String,
    pub new_name: String,
}

pub fn preview_batch_rename(req: &BatchRenameRequest) -> OpResult<Vec<RenameOp>> {
    let re = if req.use_regex {
        Some(Regex::new(&req.pattern).map_err(|e| OpfileError::invalid(format!("Invalid regex: {e}")))?)
    } else {
        None
    };

    let mut ops = Vec::with_capacity(req.paths.len());
    for (i, path_str) in req.paths.iter().enumerate() {
        let p = Path::new(path_str);
        let parent = p
            .parent()
            .ok_or_else(|| OpfileError::invalid("Missing parent path"))?;
        let old_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| OpfileError::invalid("Invalid filename"))?
            .to_string();
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
        let idx_str = (req.start_index + i).to_string();

        let templated_repl = req
            .replacement
            .replace("{n}", &idx_str)
            .replace("{name}", stem)
            .replace("{ext}", ext);

        let new_name = if let Some(ref regex) = re {
            regex.replace_all(&old_name, templated_repl.as_str()).into_owned()
        } else if req.pattern.is_empty() {
            templated_repl
        } else {
            old_name.replace(&req.pattern, &templated_repl)
        };

        let new_path = parent.join(&new_name).to_string_lossy().into_owned();
        ops.push(RenameOp {
            old_path: path_str.clone(),
            new_path,
            old_name,
            new_name,
        });
    }
    Ok(ops)
}

pub fn execute_batch_rename(ops: &[RenameOp]) -> OpResult<Vec<RenameOp>> {
    let mut applied = Vec::with_capacity(ops.len());
    for op in ops {
        if op.old_path != op.new_path {
            fs::rename(&op.old_path, &op.new_path)?;
            applied.push(op.clone());
        }
    }
    Ok(applied)
}

pub fn undo_batch_rename(ops: &[RenameOp]) -> OpResult<usize> {
    let mut count = 0usize;
    for op in ops.iter().rev() {
        if Path::new(&op.new_path).exists() {
            fs::rename(&op.new_path, &op.old_path)?;
            count += 1;
        }
    }
    Ok(count)
}

#[derive(Serialize)]
pub struct QrMatrix {
    pub width: usize,
    pub modules: Vec<u8>,
}

pub fn generate_qr_matrix(content: &str) -> OpResult<QrMatrix> {
    let code = QrCode::new(content.as_bytes())
        .map_err(|e| OpfileError::convert(format!("QR generation error: {e}")))?;
    let width = code.width();
    let modules = code
        .to_colors()
        .into_iter()
        .map(|c| match c {
            qrcode::Color::Dark => 1u8,
            qrcode::Color::Light => 0u8,
        })
        .collect();
    Ok(QrMatrix { width, modules })
}
