use crate::error::{OpResult, OpfileError};
use crate::task_manager::{
    stream_copy_with_task, tokio_rt, AtomicPartFile, TaskHandle, STREAM_BUF_SIZE,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fs::{self, Metadata};
use std::io::{BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    Name,
    Size,
    Date,
    Type,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub is_hidden: bool,
    pub size: u64,
    pub modified_ms: i64,
    pub permissions: String,
    pub extension: String,
    pub category: String,
    pub child_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryListing {
    pub path: String,
    pub current_path: String,
    pub parent: Option<String>,
    pub parent_path: Option<String>,
    pub entries: Vec<FileEntry>,
    pub skipped_restricted: usize,
}

pub fn format_rwx(meta: &Metadata) -> String {
    let mode = meta.permissions().mode();
    let kind = if meta.is_dir() {
        'd'
    } else if meta.file_type().is_symlink() {
        'l'
    } else {
        '-'
    };
    let flags = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let mut out = String::with_capacity(10);
    out.push(kind);
    for (mask, ch) in flags {
        out.push(if mode & mask != 0 { ch } else { '-' });
    }
    out
}

pub fn classify_extension(ext: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return "folder";
    }
    match ext {
        "zip" | "7z" | "rar" | "tar" | "gz" | "tgz" | "bz2" | "tbz2" | "xz" | "txz" | "zst"
        | "tzst" | "lz4" | "iso" | "img" | "dmg" | "qcow2" | "vdi" | "vmdk" | "cab" | "ar" => {
            "archive"
        }
        "apk" | "xapk" | "apks" | "apkm" | "obb" => "apk",
        "png" | "jpg" | "jpeg" | "webp" | "avif" | "heic" | "heif" | "jxl" | "tiff" | "tif"
        | "bmp" | "gif" | "apng" | "ico" | "icns" | "dds" | "ktx2" | "psd" | "svg" | "hdr"
        | "exr" | "cr2" | "nef" | "arw" | "dng" | "qoi" | "tga" => "image",
        "mp3" | "aac" | "m4a" | "ogg" | "opus" | "flac" | "alac" | "wav" | "aiff" | "mid"
        | "midi" | "mod" | "xm" | "it" | "s3m" | "wma" => "audio",
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "flv" | "m2ts" | "m4v" | "3gp" | "wmv" => "video",
        "pdf" | "docx" | "doc" | "odt" | "rtf" | "xlsx" | "xls" | "ods" | "csv" | "tsv"
        | "parquet" | "sqlite" | "db" | "pptx" | "ppt" | "odp" | "epub" | "mobi" | "cbz"
        | "tex" => "document",
        "rs" | "kt" | "kts" | "java" | "c" | "cpp" | "h" | "hpp" | "py" | "js" | "ts"
        | "json" | "yaml" | "yml" | "toml" | "xml" | "html" | "htm" | "css" | "md" | "txt"
        | "ini" | "conf" | "cfg" | "env" | "sh" | "gradle" | "sql" | "srt" | "vtt" | "ass"
        | "vcf" | "log" => "code",
        "obj" | "fbx" | "stl" | "gltf" | "glb" | "ply" | "dxf" | "ttf" | "otf" | "woff2"
        | "gpx" | "kml" | "geojson" | "ipynb" | "h5" | "hdf5" | "las" | "laz" | "pem" | "der"
        | "pfx" | "p12" | "crt" | "cer" => "specialized",
        _ => "file",
    }
}

pub fn list_directory(
    dir_path: &str,
    sort_field: SortField,
    descending: bool,
    show_hidden: bool,
    query: Option<&str>,
) -> OpResult<DirectoryListing> {
    let rt = tokio_rt()?;
    let target = PathBuf::from(dir_path);
    let query_lower = query.map(|q| q.trim().to_ascii_lowercase()).filter(|q| !q.is_empty());

    rt.block_on(async move {
        tokio::task::spawn_blocking(move || {
            list_directory_blocking(&target, sort_field, descending, show_hidden, query_lower.as_deref())
        })
        .await
        .map_err(|e| OpfileError::io(format!("Directory scan task failed: {e}")))?
    })
}

fn list_directory_blocking(
    target: &Path,
    sort_field: SortField,
    descending: bool,
    show_hidden: bool,
    query_lower: Option<&str>,
) -> OpResult<DirectoryListing> {
    if !target.exists() {
        return Err(OpfileError::new(
            crate::error::ErrorCode::NotFound,
            format!("Path does not exist: {}", target.display()),
        ));
    }

    let mut entries = Vec::new();
    let mut skipped_restricted = 0usize;

    let read_dir = match fs::read_dir(target) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let p_str = target.to_string_lossy().into_owned();
            let parent_opt = target.parent().map(|p| p.to_string_lossy().into_owned());
            return Ok(DirectoryListing {
                path: p_str.clone(),
                current_path: p_str,
                parent: parent_opt.clone(),
                parent_path: parent_opt,
                entries: Vec::new(),
                skipped_restricted: 1,
            });
        }
        Err(e) => return Err(e.into()),
    };

    for item in read_dir {
        let dir_entry = match item {
            Ok(de) => de,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                skipped_restricted += 1;
                continue;
            }
            Err(_) => {
                skipped_restricted += 1;
                continue;
            }
        };

        let file_name = dir_entry.file_name().to_string_lossy().into_owned();
        if file_name == "." || file_name == ".." {
            continue;
        }
        let is_hidden = file_name.starts_with('.');
        if is_hidden && !show_hidden {
            continue;
        }
        if let Some(q) = query_lower {
            if !file_name.to_ascii_lowercase().contains(q) {
                continue;
            }
        }

        let path = dir_entry.path();
        let sym_meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                skipped_restricted += 1;
                continue;
            }
            Err(_) => continue,
        };
        let is_symlink = sym_meta.file_type().is_symlink();
        let meta = if is_symlink {
            fs::metadata(&path).unwrap_or(sym_meta)
        } else {
            sym_meta
        };

        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let permissions = format_rwx(&meta);
        let extension = if is_dir {
            String::new()
        } else {
            path.extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default()
        };
        let category = classify_extension(&extension, is_dir).to_string();

        entries.push(FileEntry {
            name: file_name,
            path: path.to_string_lossy().into_owned(),
            is_dir,
            is_symlink,
            is_hidden,
            size,
            modified_ms,
            permissions,
            extension,
            category,
            child_count: None,
        });
    }

    // Strict directory-first natural sorting
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }
        let ord = match sort_field {
            SortField::Name => natord::compare_ignore_case(&a.name, &b.name),
            SortField::Size => a
                .size
                .cmp(&b.size)
                .then_with(|| natord::compare_ignore_case(&a.name, &b.name)),
            SortField::Date => a
                .modified_ms
                .cmp(&b.modified_ms)
                .then_with(|| natord::compare_ignore_case(&a.name, &b.name)),
            SortField::Type => a
                .extension
                .cmp(&b.extension)
                .then_with(|| natord::compare_ignore_case(&a.name, &b.name)),
        };
        if descending {
            ord.reverse()
        } else {
            ord
        }
    });

    let p_str = target.to_string_lossy().into_owned();
    let parent_opt = target.parent().map(|p| p.to_string_lossy().into_owned());
    Ok(DirectoryListing {
        path: p_str.clone(),
        current_path: p_str,
        parent: parent_opt.clone(),
        parent_path: parent_opt,
        entries,
        skipped_restricted,
    })
}

/// Fault-tolerant recursive search using walkdir that suppresses PermissionDenied.
pub fn search_recursive(root: &str, query: &str, max_results: usize) -> OpResult<Vec<FileEntry>> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    let mut results = Vec::new();
    let walker = WalkDir::new(root).follow_links(false).into_iter();
    for entry_res in walker {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.depth() == 0 {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !name.to_ascii_lowercase().contains(&q) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        let ext = if is_dir {
            String::new()
        } else {
            entry
                .path()
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default()
        };
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let is_hidden = name.starts_with('.');
        results.push(FileEntry {
            name: name.into_owned(),
            path: entry.path().to_string_lossy().into_owned(),
            is_dir,
            is_symlink: entry.path_is_symlink(),
            is_hidden,
            size: if is_dir { 0 } else { meta.len() },
            modified_ms,
            permissions: format_rwx(&meta),
            extension: ext.clone(),
            category: classify_extension(&ext, is_dir).to_string(),
            child_count: None,
        });
        if results.len() >= max_results {
            break;
        }
    }
    Ok(results)
}

pub fn create_folder(path: &str) -> OpResult<()> {
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn create_empty_file(path: &str) -> OpResult<()> {
    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let (guard, mut w) = AtomicPartFile::create(p)?;
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}

pub fn rename_entry(old_path: &str, new_name: &str) -> OpResult<String> {
    let src = Path::new(old_path);
    let parent = src
        .parent()
        .ok_or_else(|| OpfileError::invalid("Cannot determine parent folder"))?;
    let dst = parent.join(new_name);
    fs::rename(src, &dst)?;
    Ok(dst.to_string_lossy().into_owned())
}

pub fn copy_or_move_batch(
    sources: Vec<String>,
    dest_dir: &str,
    move_mode: bool,
    task: &TaskHandle,
) -> OpResult<Vec<String>> {
    let dst_root = PathBuf::from(dest_dir);
    fs::create_dir_all(&dst_root)?;

    let mut total_bytes = 0u64;
    let mut file_pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut dirs_to_create: Vec<PathBuf> = Vec::new();

    for src_str in &sources {
        let src = PathBuf::from(src_str);
        if !src.exists() {
            continue;
        }
        let base_name = src
            .file_name()
            .ok_or_else(|| OpfileError::invalid("Invalid source filename"))?;
        let target_base = unique_dest_path(&dst_root, base_name, &src);

        if move_mode && fs::rename(&src, &target_base).is_ok() {
            task.inc_items();
            continue;
        }

        if src.is_dir() {
            for entry in WalkDir::new(&src).into_iter().filter_map(Result::ok) {
                let rel = entry
                    .path()
                    .strip_prefix(&src)
                    .map_err(|e| OpfileError::io(e.to_string()))?;
                let out = target_base.join(rel);
                if entry.file_type().is_dir() {
                    dirs_to_create.push(out);
                } else {
                    total_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                    file_pairs.push((entry.path().to_path_buf(), out));
                }
            }
        } else {
            total_bytes += src.metadata().map(|m| m.len()).unwrap_or(0);
            file_pairs.push((src, target_base));
        }
    }

    task.set_totals(total_bytes, (dirs_to_create.len() + file_pairs.len()) as u64);

    for d in &dirs_to_create {
        task.check_checkpoint()?;
        fs::create_dir_all(d)?;
        task.inc_items();
    }

    let mut created_outputs = Vec::with_capacity(file_pairs.len());
    for (src, dst) in &file_pairs {
        task.check_checkpoint()?;
        task.set_current_item(src.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        stream_copy_with_task(src, dst, Some(task))?;
        created_outputs.push(dst.to_string_lossy().into_owned());
        task.inc_items();
    }

    if move_mode {
        for src_str in &sources {
            let p = Path::new(src_str);
            if p.exists() {
                if p.is_dir() {
                    let _ = fs::remove_dir_all(p);
                } else {
                    let _ = fs::remove_file(p);
                }
            }
        }
    }

    Ok(created_outputs)
}

fn unique_dest_path(dst_root: &Path, name: &std::ffi::OsStr, src: &Path) -> PathBuf {
    let candidate = dst_root.join(name);
    if !candidate.exists() && candidate != src {
        return candidate;
    }
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("copy");
    let ext = Path::new(name).extension().and_then(|e| e.to_str());
    for idx in 1..10_000 {
        let next_name = match ext {
            Some(e) if !e.is_empty() => format!("{stem} ({idx}).{e}"),
            _ => format!("{stem} ({idx})"),
        };
        let p = dst_root.join(next_name);
        if !p.exists() {
            return p;
        }
    }
    candidate
}

pub fn delete_paths_batch(paths: Vec<String>, task: &TaskHandle) -> OpResult<usize> {
    task.set_totals(0, paths.len() as u64);
    let mut count = 0usize;
    for path_str in paths {
        task.check_checkpoint()?;
        let p = Path::new(&path_str);
        task.set_current_item(p.file_name().and_then(|n| n.to_str()).unwrap_or(""));
        if p.is_dir() {
            fs::remove_dir_all(p)?;
        } else if p.exists() {
            fs::remove_file(p)?;
        }
        count += 1;
        task.inc_items();
    }
    Ok(count)
}

pub fn read_text_bounded(path: &str, max_bytes: usize) -> OpResult<String> {
    let f = fs::File::open(path)?;
    let mut reader = BufReader::with_capacity(STREAM_BUF_SIZE, f);
    let mut buf = Vec::with_capacity(max_bytes.min(64 * 1024));
    let mut chunk = [0u8; 8192];
    while buf.len() < max_bytes {
        let to_read = chunk.len().min(max_bytes - buf.len());
        let n = reader.read(&mut chunk[..to_read])?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

pub fn write_text_atomic(path: &str, content: &str) -> OpResult<()> {
    let (guard, mut w) = AtomicPartFile::create(Path::new(path))?;
    w.write_all(content.as_bytes())?;
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}
