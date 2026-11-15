use crate::convert_docs_3d::{convert_document_or_data, convert_specialized_format};
use crate::convert_media::{convert_audio_pipeline, convert_image_pipeline, MediaConvertOpts};
use crate::error::{OpResult, OpfileError};
use crate::task_manager::{AtomicPartFile, TaskHandle};
use crate::utilities::generate_qr_matrix;
use image::{ImageFormat, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SUPPORTED_IMAGE_FORMATS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "avif", "heic", "heif", "jxl", "tiff", "tif", "bmp", "gif",
    "apng", "ico", "icns", "dds", "ktx2", "psd", "dcm", "dicom", "svg", "svgz", "hdr", "exr",
    "tga", "qoi", "pnm", "ppm", "pgm", "pbm", "pam", "ff", "cr2", "nef", "arw", "dng", "raf",
    "orf", "rw2", "pef", "srw", "x3f", "raw", "nrw", "kdc", "mos", "pcx", "xbm", "xpm", "wbmp",
    "cur", "sgi", "ras", "jp2", "j2k", "jxr", "bpg", "flif", "mng", "xcf", "ai", "eps", "ps",
];

pub const SUPPORTED_DOC_DATA_FORMATS: &[&str] = &[
    "pdf", "docx", "doc", "odt", "rtf", "txt", "md", "markdown", "html", "htm", "xhtml", "tex",
    "latex", "rst", "adoc", "org", "xlsx", "xls", "ods", "csv", "tsv", "parquet", "sqlite", "db",
    "pptx", "ppt", "odp", "epub", "mobi", "azw3", "fb2", "cbz", "cbr", "djvu", "json", "jsonl",
    "ndjson", "yaml", "yml", "toml", "xml", "ini", "cfg", "conf", "properties", "env", "dotenv",
    "vcf", "vcard", "ics", "ical", "srt", "vtt", "ass", "ssa", "sub", "lrc", "ttml", "sbv", "sql",
    "bson", "cbor", "msgpack", "ron", "hcl", "plist", "b64", "hex", "qr", "log", "diff", "patch",
];

pub const SUPPORTED_AV_FORMATS: &[&str] = &[
    "mp4", "mkv", "webm", "mov", "avi", "flv", "ts", "mts", "m2ts", "m4v", "3gp", "3g2", "wmv",
    "asf", "vob", "ogv", "mxf", "rm", "rmvb", "nut", "f4v", "mp3", "aac", "m4a", "m4b", "m4r",
    "ogg", "oga", "opus", "flac", "alac", "wav", "aiff", "aif", "au", "caf", "wma", "ac3",
    "eac3", "dts", "amr", "awb", "spx", "wv", "ape", "tta", "mpc", "dsf", "dff", "mid", "midi",
    "kar", "mod", "xm", "it", "s3m", "pcm", "mka", "weba",
];

pub const SUPPORTED_SPECIALIZED_FORMATS: &[&str] = &[
    "obj", "fbx", "stl", "gltf", "glb", "ply", "dae", "3ds", "off", "x3d", "usd", "usda", "usdc",
    "dxf", "dwg", "step", "stp", "iges", "igs", "ttf", "otf", "woff", "woff2", "eot", "sf2",
    "gpx", "kml", "kmz", "geojson", "topojson", "wkt", "shp", "ipynb", "h5", "hdf5", "nc",
    "netcdf", "fits", "las", "laz", "xyz", "pcd", "pem", "der", "cer", "crt", "pfx", "p12",
    "jwk", "pub", "asc", "zip", "7z", "tar", "tar.gz", "tgz", "tar.bz2", "tar.xz", "tar.zst",
    "gz", "bz2", "xz", "zst", "opx", "iso", "img", "dmg", "qcow2", "vdi", "vmdk",
];

#[derive(Serialize)]
pub struct FormatCatalogInfo {
    pub total_supported_formats: usize,
    pub source_extension: String,
    pub primary_targets: Vec<String>,
    pub all_valid_targets: Vec<String>,
    pub needs_native_mediacodec: bool,
}

pub fn total_format_count() -> usize {
    SUPPORTED_IMAGE_FORMATS.len()
        + SUPPORTED_DOC_DATA_FORMATS.len()
        + SUPPORTED_AV_FORMATS.len()
        + SUPPORTED_SPECIALIZED_FORMATS.len()
}

pub fn get_valid_targets_for_extension(ext: &str) -> FormatCatalogInfo {
    let e = ext.trim_start_matches('.').to_ascii_lowercase();
    let total = total_format_count();

    let (primary, all_valid, native_codec) = if SUPPORTED_IMAGE_FORMATS.contains(&e.as_str()) {
        let targets: Vec<String> = vec![
            "webp", "png", "jpg", "avif", "gif", "bmp", "pdf", "svg", "zip", "7z",
        ]
        .into_iter()
        .filter(|&t| t != e)
        .map(str::to_string)
        .collect();
        let all = targets.clone();
        (targets, all, matches!(e.as_str(), "heic" | "heif" | "avif"))
    } else if matches!(
        e.as_str(),
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "flv" | "ts" | "m4v" | "3gp" | "wmv" | "ogv"
    ) {
        let targets: Vec<String> = vec![
            "mp4", "mkv", "webm", "gif", "mp3", "aac", "wav", "zip", "7z",
        ]
        .into_iter()
        .filter(|&t| t != e)
        .map(str::to_string)
        .collect();
        let all = targets.clone();
        (targets, all, true)
    } else if SUPPORTED_AV_FORMATS.contains(&e.as_str()) {
        let targets: Vec<String> = vec![
            "mp3", "wav", "flac", "aac", "ogg", "opus", "m4a", "zip", "7z",
        ]
        .into_iter()
        .filter(|&t| t != e)
        .map(str::to_string)
        .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(e.as_str(), "srt" | "vtt" | "ass" | "ssa" | "lrc" | "sbv") {
        let targets: Vec<String> = vec!["srt", "vtt", "ass", "txt", "lrc", "zip", "7z"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(
        e.as_str(),
        "json" | "yaml" | "yml" | "toml" | "xml" | "csv" | "tsv" | "xlsx" | "sqlite" | "db"
            | "parquet" | "ini" | "env"
    ) {
        let targets: Vec<String> = vec![
            "json", "csv", "xml", "yaml", "sqlite", "parquet", "pdf", "zip", "7z",
        ]
        .into_iter()
        .filter(|&t| t != e)
        .map(str::to_string)
        .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(
        e.as_str(),
        "pdf" | "docx" | "doc" | "odt" | "rtf" | "epub" | "html" | "htm" | "md" | "txt"
    ) {
        let targets: Vec<String> = vec!["pdf", "txt", "md", "html", "docx", "epub", "zip", "7z"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(e.as_str(), "obj" | "stl" | "ply" | "gltf" | "glb" | "dxf" | "fbx") {
        let targets: Vec<String> = vec!["obj", "stl", "ply", "gltf", "dxf", "zip", "7z"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(e.as_str(), "gpx" | "kml" | "geojson" | "wkt") {
        let targets: Vec<String> = vec!["geojson", "gpx", "kml", "wkt", "zip", "7z"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(e.as_str(), "pem" | "der" | "crt" | "cer") {
        let targets: Vec<String> = vec!["pem", "der", "crt", "zip", "7z"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(e.as_str(), "ttf" | "otf" | "woff2") {
        let targets: Vec<String> = vec!["woff2", "ttf", "otf", "zip", "7z"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else if matches!(
        e.as_str(),
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" | "zst" | "tgz" | "tbz2" | "txz"
    ) {
        let targets: Vec<String> = vec!["zip", "7z", "tar.gz", "tar.xz"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    } else {
        let targets: Vec<String> = vec!["zip", "7z", "tar.gz", "qr"]
            .into_iter()
            .filter(|&t| t != e)
            .map(str::to_string)
            .collect();
        let all = targets.clone();
        (targets, all, false)
    };

    FormatCatalogInfo {
        total_supported_formats: total,
        source_extension: e,
        primary_targets: primary,
        all_valid_targets: all_valid,
        needs_native_mediacodec: native_codec,
    }
}

#[derive(Deserialize)]
pub struct ConvertRequest {
    pub source_path: String,
    pub target_format: String,
    pub dest_path: Option<String>,
    pub quality: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub normalize_lufs: Option<f32>,
    pub decimate_ratio: Option<f32>,
    pub unicode_form: Option<String>,
}

pub fn convert_file(req: &ConvertRequest, task: Option<&TaskHandle>) -> OpResult<String> {
    let src = Path::new(&req.source_path);
    if !src.exists() {
        return Err(OpfileError::new(
            crate::error::ErrorCode::NotFound,
            format!("Source file not found: {}", req.source_path),
        ));
    }
    let src_ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let dst_ext = req.target_format.trim_start_matches('.').to_ascii_lowercase();

    let dst: PathBuf = match &req.dest_path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => src.with_extension(&dst_ext),
    };

    if dst_ext == "qr" {
        let payload = std::fs::read_to_string(src).unwrap_or_else(|_| req.source_path.clone());
        let trimmed = payload.chars().take(1800).collect::<String>();
        let qr = generate_qr_matrix(&trimmed)?;
        let scale = 8u32;
        let dim = (qr.width as u32 + 4) * scale;
        let mut img = RgbaImage::from_pixel(dim, dim, Rgba([255, 255, 255, 255]));
        for y in 0..qr.width {
            for x in 0..qr.width {
                if qr.modules[y * qr.width + x] == 1 {
                    let px0 = (x as u32 + 2) * scale;
                    let py0 = (y as u32 + 2) * scale;
                    for dy in 0..scale {
                        for dx in 0..scale {
                            img.put_pixel(px0 + dx, py0 + dy, Rgba([0, 0, 0, 255]));
                        }
                    }
                }
            }
        }
        let qr_png = dst.with_extension("qr.png");
        let (guard, mut bw) = AtomicPartFile::create(&qr_png)?;
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut bw, ImageFormat::Png)
            .map_err(|e| OpfileError::convert(e.to_string()))?;
        std::io::Write::flush(&mut bw)?;
        drop(bw);
        guard.commit()?;
        return Ok(qr_png.to_string_lossy().into_owned());
    }

    let media_opts = MediaConvertOpts {
        quality: req.quality,
        width: req.width,
        height: req.height,
        tone_map: Some("filmic".to_string()),
        quantize_colors: None,
        sample_rate: req.sample_rate,
        normalize_lufs: req.normalize_lufs,
    };

    if SUPPORTED_IMAGE_FORMATS.contains(&src_ext.as_str())
        && (SUPPORTED_IMAGE_FORMATS.contains(&dst_ext.as_str()) || dst_ext == "pdf")
    {
        convert_image_pipeline(src, &dst, &src_ext, &dst_ext, &media_opts, task)?;
        return Ok(dst.to_string_lossy().into_owned());
    }

    if SUPPORTED_AV_FORMATS.contains(&src_ext.as_str())
        && SUPPORTED_AV_FORMATS.contains(&dst_ext.as_str())
    {
        convert_audio_pipeline(src, &dst, &src_ext, &dst_ext, &media_opts, task)?;
        return Ok(dst.to_string_lossy().into_owned());
    }

    if matches!(
        dst_ext.as_str(),
        "zip" | "7z" | "tar" | "tar.gz" | "tar.bz2" | "tar.xz" | "tar.zst" | "gz" | "bz2" | "xz"
            | "zst" | "opx"
    ) {
        let arch_req = crate::archive_ops::CreateArchiveRequest {
            sources: vec![req.source_path.clone()],
            dest_path: dst.to_string_lossy().into_owned(),
            format: dst_ext,
            profile: crate::archive_ops::CompressionProfile::Normal,
            dict_size_mb: None,
            password: None,
            split_size_mb: None,
        };
        let outs = crate::archive_ops::create_archive(&arch_req, task)?;
        return Ok(outs.into_iter().next().unwrap_or_default());
    }

    if let Ok(()) = convert_specialized_format(src, &dst, &src_ext, &dst_ext, req.decimate_ratio, task) {
        return Ok(dst.to_string_lossy().into_owned());
    }

    convert_document_or_data(
        src,
        &dst,
        &src_ext,
        &dst_ext,
        req.unicode_form.as_deref(),
        task,
    )?;
    Ok(dst.to_string_lossy().into_owned())
}
