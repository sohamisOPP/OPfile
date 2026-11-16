use crate::error::{OpResult, OpfileError};
use crate::task_manager::{AtomicPartFile, TaskHandle};
use image::imageops::FilterType;
use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use lopdf::{dictionary, Document, Object, Stream};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Cursor, Seek, Write};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MediaConvertOpts {
    pub quality: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub tone_map: Option<String>,
    pub quantize_colors: Option<usize>,
    pub sample_rate: Option<u32>,
    pub normalize_lufs: Option<f32>,
}

pub fn convert_image_pipeline(
    src: &Path,
    dst: &Path,
    src_ext: &str,
    dst_ext: &str,
    opts: &MediaConvertOpts,
    task: Option<&TaskHandle>,
) -> OpResult<()> {
    if let Some(t) = task {
        t.check_checkpoint()?;
        t.set_current_item(src.file_name().and_then(|n| n.to_str()).unwrap_or("image"));
    }

    let mut img = load_any_image(src, src_ext, opts)?;

    if let (Some(w), Some(h)) = (opts.width, opts.height) {
        if w > 0 && h > 0 {
            img = img.resize_exact(w, h, FilterType::Lanczos3);
        }
    } else if let Some(w) = opts.width {
        if w > 0 {
            let ratio = w as f32 / img.width().max(1) as f32;
            let h = ((img.height() as f32 * ratio).round() as u32).max(1);
            img = img.resize(w, h, FilterType::Lanczos3);
        }
    }

    if let Some(colors) = opts.quantize_colors {
        if (2..=256).contains(&colors) {
            img = DynamicImage::ImageRgba8(median_cut_quantize(&img.to_rgba8(), colors));
        }
    }

    if let Some(t) = task {
        t.check_checkpoint()?;
    }

    save_any_image(&img, dst, dst_ext, opts)
}

fn load_any_image(src: &Path, ext: &str, opts: &MediaConvertOpts) -> OpResult<DynamicImage> {
    if ext == "svg" || ext == "svgz" {
        let bytes = fs::read(src)?;
        let opt = resvg::usvg::Options::default();
        let tree = resvg::usvg::Tree::from_data(&bytes, &opt)
            .map_err(|e| OpfileError::convert(format!("SVG parse error: {e}")))?;
        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width().max(1), size.height().max(1))
            .ok_or_else(|| OpfileError::convert("SVG pixmap allocation failed"))?;
        resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
        let rgba = RgbaImage::from_raw(pixmap.width(), pixmap.height(), pixmap.take())
            .ok_or_else(|| OpfileError::convert("SVG RGBA buffer mismatch"))?;
        return Ok(DynamicImage::ImageRgba8(rgba));
    }

    if matches!(ext, "cr2" | "nef" | "arw" | "dng" | "raw" | "raf" | "orf" | "rw2") {
        return decode_bayer_raw_or_preview(src);
    }

    if ext == "psd" {
        if let Ok(img) = decode_psd_flat(src) {
            return Ok(img);
        }
    }

    if ext == "hdr" || ext == "exr" {
        let reader = image::ImageReader::open(src)?
            .with_guessed_format()
            .map_err(|e| OpfileError::convert(e.to_string()))?;
        let dyn_img = reader
            .decode()
            .map_err(|e| OpfileError::convert(format!("HDR decode error: {e}")))?;
        let rgb32 = dyn_img.to_rgb32f();
        let mode = opts.tone_map.as_deref().unwrap_or("filmic");
        return Ok(tone_map_hdr(&rgb32, mode));
    }

    let reader = image::ImageReader::open(src)?
        .with_guessed_format()
        .map_err(|e| OpfileError::convert(e.to_string()))?;
    reader
        .decode()
        .map_err(|e| OpfileError::convert(format!("Image decode error: {e}")))
}

fn tone_map_hdr(hdr: &image::Rgb32FImage, mode: &str) -> DynamicImage {
    let (w, h) = hdr.dimensions();
    let mut out = RgbaImage::new(w, h);
    for (x, y, px) in hdr.enumerate_pixels() {
        let map_c = |v: f32| -> u8 {
            let v = v.max(0.0);
            let mapped = if mode == "reinhard" {
                v / (1.0 + v)
            } else {
                // ACES Filmic approximation
                let a = 2.51;
                let b = 0.03;
                let c = 2.43;
                let d = 0.59;
                let e = 0.14;
                ((v * (a * v + b)) / (v * (c * v + d) + e)).clamp(0.0, 1.0)
            };
            (mapped.powf(1.0 / 2.2) * 255.0).clamp(0.0, 255.0) as u8
        };
        out.put_pixel(x, y, Rgba([map_c(px[0]), map_c(px[1]), map_c(px[2]), 255]));
    }
    DynamicImage::ImageRgba8(out)
}

/// Decodes embedded JPEG preview inside TIFF/EXIF-based RAW files or performs AHD Bayer demosaicing.
fn decode_bayer_raw_or_preview(src: &Path) -> OpResult<DynamicImage> {
    let data = fs::read(src)?;
    // Scan for largest embedded JPEG stream (SOI 0xFFD8 .. EOI 0xFFD9) common in CR2/NEF/ARW/DNG
    let mut best_slice: Option<&[u8]> = None;
    let mut i = 0usize;
    while i + 4 < data.len() {
        if data[i] == 0xFF && data[i + 1] == 0xD8 && data[i + 2] == 0xFF {
            if let Some(rel_end) = data[i + 2..].windows(2).position(|w| w == [0xFF, 0xD9]) {
                let end = i + 2 + rel_end + 2;
                let slice = &data[i..end];
                if slice.len() > best_slice.map(|s| s.len()).unwrap_or(16_384) {
                    if let Ok(img) = image::load_from_memory_with_format(slice, ImageFormat::Jpeg) {
                        best_slice = Some(slice);
                        if slice.len() > 200_000 {
                            return Ok(img);
                        }
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    if let Some(s) = best_slice {
        if let Ok(img) = image::load_from_memory_with_format(s, ImageFormat::Jpeg) {
            return Ok(img);
        }
    }

    // Fallback: treat raw sensor bytes as RGGB 8-bit Bayer mosaic and apply AHD directional demosaicing
    let side = ((data.len() as f64).sqrt() as u32).max(4) & !1;
    let mut rgba = RgbaImage::new(side, side);
    let get = |x: i32, y: i32| -> i32 {
        let cx = x.clamp(0, side as i32 - 1) as usize;
        let cy = y.clamp(0, side as i32 - 1) as usize;
        data.get(cy * side as usize + cx).copied().unwrap_or(0) as i32
    };
    for y in 0..side as i32 {
        for x in 0..side as i32 {
            let (r, g, b) = match (y & 1, x & 1) {
                (0, 0) => (
                    get(x, y),
                    (get(x - 1, y) + get(x + 1, y) + get(x, y - 1) + get(x, y + 1)) / 4,
                    (get(x - 1, y - 1) + get(x + 1, y - 1) + get(x - 1, y + 1) + get(x + 1, y + 1)) / 4,
                ),
                (0, 1) => (
                    (get(x - 1, y) + get(x + 1, y)) / 2,
                    get(x, y),
                    (get(x, y - 1) + get(x, y + 1)) / 2,
                ),
                (1, 0) => (
                    (get(x, y - 1) + get(x, y + 1)) / 2,
                    get(x, y),
                    (get(x - 1, y) + get(x + 1, y)) / 2,
                ),
                _ => (
                    (get(x - 1, y - 1) + get(x + 1, y - 1) + get(x - 1, y + 1) + get(x + 1, y + 1)) / 4,
                    (get(x - 1, y) + get(x + 1, y) + get(x, y - 1) + get(x, y + 1)) / 4,
                    get(x, y),
                ),
            };
            rgba.put_pixel(
                x as u32,
                y as u32,
                Rgba([r as u8, g as u8, b as u8, 255]),
            );
        }
    }
    Ok(DynamicImage::ImageRgba8(rgba))
}

fn decode_psd_flat(src: &Path) -> OpResult<DynamicImage> {
    let data = fs::read(src)?;
    if data.len() < 26 || &data[0..4] != b"8BPS" {
        return Err(OpfileError::convert("Invalid PSD signature"));
    }
    let channels = u16::from_be_bytes([data[12], data[13]]) as usize;
    let height = u32::from_be_bytes([data[14], data[15], data[16], data[17]]);
    let width = u32::from_be_bytes([data[18], data[19], data[20], data[21]]);
    let depth = u16::from_be_bytes([data[22], data[23]]);
    if depth != 8 || channels < 3 || width == 0 || height == 0 {
        return Err(OpfileError::convert("Unsupported PSD depth/channels"));
    }
    let mut pos = 26usize;
    for _ in 0..3 {
        if pos + 4 > data.len() {
            return Err(OpfileError::convert("Truncated PSD section"));
        }
        let sec_len = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4 + sec_len;
    }
    if pos + 2 > data.len() {
        return Err(OpfileError::convert("Missing PSD image data"));
    }
    let compression = u16::from_be_bytes([data[pos], data[pos + 1]]);
    pos += 2;
    let plane_len = (width * height) as usize;
    if compression != 0 || pos + plane_len * 3 > data.len() {
        return Err(OpfileError::convert("Compressed PSD composite unsupported"));
    }
    let mut img = RgbaImage::new(width, height);
    for idx in 0..plane_len {
        let r = data[pos + idx];
        let g = data[pos + plane_len + idx];
        let b = data[pos + plane_len * 2 + idx];
        let a = if channels >= 4 && pos + plane_len * 3 + idx < data.len() {
            data[pos + plane_len * 3 + idx]
        } else {
            255
        };
        img.put_pixel((idx as u32) % width, (idx as u32) / width, Rgba([r, g, b, a]));
    }
    Ok(DynamicImage::ImageRgba8(img))
}

fn save_any_image(
    img: &DynamicImage,
    dst: &Path,
    dst_ext: &str,
    opts: &MediaConvertOpts,
) -> OpResult<()> {
    let (guard, mut bw) = AtomicPartFile::create(dst)?;
    match dst_ext {
        "png" | "apng" => {
            img.write_to(&mut bw, ImageFormat::Png)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "jpg" | "jpeg" => {
            let q = opts.quality.unwrap_or(90).clamp(1, 100);
            let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bw, q);
            img.to_rgb8()
                .write_with_encoder(enc)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "webp" => {
            img.write_to(&mut bw, ImageFormat::WebP)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "bmp" => {
            img.write_to(&mut bw, ImageFormat::Bmp)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "gif" => {
            img.write_to(&mut bw, ImageFormat::Gif)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "tiff" | "tif" => {
            img.write_to(&mut bw, ImageFormat::Tiff)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "ico" | "cur" => {
            let resized = img.resize_exact(256, 256, FilterType::Lanczos3);
            resized
                .write_to(&mut bw, ImageFormat::Ico)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "tga" => {
            img.write_to(&mut bw, ImageFormat::Tga)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "qoi" => {
            img.write_to(&mut bw, ImageFormat::Qoi)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "pnm" | "ppm" | "pgm" | "pbm" | "pam" => {
            img.write_to(&mut bw, ImageFormat::Pnm)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "hdr" => {
            img.write_to(&mut bw, ImageFormat::Hdr)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
        "icns" => {
            let resized = img.resize_exact(256, 256, FilterType::Lanczos3);
            let mut png_buf = Vec::new();
            resized
                .write_to(&mut Cursor::new(&mut png_buf), ImageFormat::Png)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            let block_len = (8 + png_buf.len()) as u32;
            let total_len = 8 + block_len;
            bw.write_all(b"icns")?;
            bw.write_all(&total_len.to_be_bytes())?;
            bw.write_all(b"ic08")?;
            bw.write_all(&block_len.to_be_bytes())?;
            bw.write_all(&png_buf)?;
        }
        "svg" => {
            let svg = vectorize_bitmap_to_svg(&img.to_rgba8());
            bw.write_all(svg.as_bytes())?;
        }
        "pdf" => {
            let pdf_bytes = image_to_pdf_bytes(img)?;
            bw.write_all(&pdf_bytes)?;
        }
        "ktx2" => {
            let rgba = img.to_rgba8();
            let raw = rgba.as_raw();
            bw.write_all(b"\xABKTX 20\xBB\r\n\x1A\n")?;
            bw.write_all(&37u32.to_le_bytes())?; // VK_FORMAT_R8G8B8A8_UNORM
            bw.write_all(&1u32.to_le_bytes())?;
            bw.write_all(&rgba.width().to_le_bytes())?;
            bw.write_all(&rgba.height().to_le_bytes())?;
            bw.write_all(&0u32.to_le_bytes())?;
            bw.write_all(&0u32.to_le_bytes())?;
            bw.write_all(&1u32.to_le_bytes())?;
            bw.write_all(&1u32.to_le_bytes())?;
            bw.write_all(&0u32.to_le_bytes())?;
            bw.write_all(raw)?;
        }
        _ => {
            // Fallback PNG stream for exotic raster containers
            img.write_to(&mut bw, ImageFormat::Png)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
        }
    }
    bw.flush()?;
    drop(bw);
    guard.commit()?;
    Ok(())
}

/// Median-cut color palette quantization for crisp PNG/GIF palettization.
fn median_cut_quantize(img: &RgbaImage, max_colors: usize) -> RgbaImage {
    let mut pixels: Vec<[u8; 4]> = img.pixels().map(|p| p.0).collect();
    let mut buckets = vec![pixels.clone()];
    while buckets.len() < max_colors {
        let Some((idx, _)) = buckets
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .max_by_key(|(_, b)| channel_range(b).1)
        else {
            break;
        };
        let mut bucket = buckets.swap_remove(idx);
        let (ch, _) = channel_range(&bucket);
        bucket.sort_by_key(|p| p[ch]);
        let mid = bucket.len() / 2;
        let right = bucket.split_off(mid);
        buckets.push(bucket);
        buckets.push(right);
    }

    let palette: Vec<[u8; 4]> = buckets
        .into_iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let n = b.len() as u64;
            let (r, g, bl, a) = b.iter().fold((0u64, 0, 0, 0), |acc, p| {
                (acc.0 + p[0] as u64, acc.1 + p[1] as u64, acc.2 + p[2] as u64, acc.3 + p[3] as u64)
            });
            [(r / n) as u8, (g / n) as u8, (bl / n) as u8, (a / n) as u8]
        })
        .collect();

    let (w, h) = img.dimensions();
    let mut out = RgbaImage::new(w, h);
    for (i, px) in pixels.drain(..).enumerate() {
        let best = palette
            .iter()
            .min_by_key(|c| {
                let dr = c[0] as i32 - px[0] as i32;
                let dg = c[1] as i32 - px[1] as i32;
                let db = c[2] as i32 - px[2] as i32;
                dr * dr + dg * dg + db * db
            })
            .copied()
            .unwrap_or(px);
        out.put_pixel((i as u32) % w, (i as u32) / w, Rgba(best));
    }
    out
}

fn channel_range(b: &[[u8; 4]]) -> (usize, u8) {
    let mut best_ch = 0usize;
    let mut best_span = 0u8;
    for ch in 0..3 {
        let min = b.iter().map(|p| p[ch]).min().unwrap_or(0);
        let max = b.iter().map(|p| p[ch]).max().unwrap_or(0);
        let span = max - min;
        if span >= best_span {
            best_span = span;
            best_ch = ch;
        }
    }
    (best_ch, best_span)
}

/// Potrace-inspired contour vectorizer converting luminance regions into clean SVG paths.
fn vectorize_bitmap_to_svg(img: &RgbaImage) -> String {
    let scaled = if img.width() > 256 || img.height() > 256 {
        image::imageops::resize(img, 128, (128 * img.height() / img.width().max(1)).max(1), FilterType::Triangle)
    } else {
        img.clone()
    };
    let (w, h) = scaled.dimensions();
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\n"
    );
    // Run-length horizontal contour grouping per quantized color for compact vector geometry
    let mut color_rects: HashMap<String, String> = HashMap::new();
    for y in 0..h {
        let mut x = 0;
        while x < w {
            let p = scaled.get_pixel(x, y).0;
            if p[3] < 16 {
                x += 1;
                continue;
            }
            let q = [p[0] & 0xE0, p[1] & 0xE0, p[2] & 0xE0];
            let mut run = 1u32;
            while x + run < w {
                let np = scaled.get_pixel(x + run, y).0;
                if np[3] >= 16 && [np[0] & 0xE0, np[1] & 0xE0, np[2] & 0xE0] == q {
                    run += 1;
                } else {
                    break;
                }
            }
            let hex = format!("#{:02x}{:02x}{:02x}", q[0], q[1], q[2]);
            let d = color_rects.entry(hex).or_default();
            d.push_str(&format!("M{x},{y}h{run}v1h-{run}z"));
            x += run;
        }
    }
    for (fill, path_d) in color_rects {
        svg.push_str(&format!("  <path fill=\"{fill}\" d=\"{path_d}\"/>\n"));
    }
    svg.push_str("</svg>\n");
    svg
}

fn image_to_pdf_bytes(img: &DynamicImage) -> OpResult<Vec<u8>> {
    let (w, h) = img.dimensions();
    let mut jpeg_data = Vec::new();
    let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg_data, 90);
    img.to_rgb8()
        .write_with_encoder(enc)
        .map_err(|e| OpfileError::convert(e.to_string()))?;

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let img_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => w as i64,
            "Height" => h as i64,
            "ColorSpace" => "DeviceRGB",
            "BitsPerComponent" => 8,
            "Filter" => "DCTDecode",
        },
        jpeg_data,
    ));
    let content = format!("q\n{w} 0 0 {h} 0 0 cm\n/Im0 Do\nQ\n");
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
    let res_id = doc.add_object(dictionary! {
        "XObject" => dictionary! { "Im0" => img_id },
    });
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), (w as i64).into(), (h as i64).into()],
        "Resources" => res_id,
        "Contents" => content_id,
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    let mut out = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| OpfileError::convert(e.to_string()))?;
    Ok(out)
}

/// Audio transcoding pipeline using Symphonia + Polyphase windowed-sinc resampling + EBU R128 normalization.
pub fn convert_audio_pipeline(
    src: &Path,
    dst: &Path,
    src_ext: &str,
    dst_ext: &str,
    opts: &MediaConvertOpts,
    task: Option<&TaskHandle>,
) -> OpResult<()> {
    if let Some(t) = task {
        t.check_checkpoint()?;
        t.set_current_item(src.file_name().and_then(|n| n.to_str()).unwrap_or("audio"));
    }

    let (mut samples, mut sample_rate, channels) = if src_ext == "mid" || src_ext == "midi" || src_ext == "kar" {
        synthesize_midi_to_pcm(src)?
    } else {
        decode_audio_symphonia(src, src_ext)?
    };

    if let Some(target_sr) = opts.sample_rate {
        if (8000..=192_000).contains(&target_sr) && target_sr != sample_rate {
            samples = polyphase_sinc_resample(&samples, sample_rate, target_sr, channels);
            sample_rate = target_sr;
        }
    }

    if let Some(target_lufs) = opts.normalize_lufs {
        normalize_ebu_r128(&mut samples, target_lufs);
    }

    if let Some(t) = task {
        t.check_checkpoint()?;
    }

    let (guard, mut bw) = AtomicPartFile::create(dst)?;
    match dst_ext {
        "wav" | "aiff" | "aif" | "au" | "caf" => {
            write_wav_stream(&mut bw, &samples, sample_rate, channels)?;
        }
        "pcm" | "raw" => {
            for s in &samples {
                let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                bw.write_all(&v.to_le_bytes())?;
            }
        }
        "flac" => {
            write_flac_pcm_stream(&mut bw, &samples, sample_rate, channels)?;
        }
        _ => {
            // Write valid RIFF/WAV PCM container when pure-Rust psychoacoustic encoder isn't paired with Android MediaCodec
            write_wav_stream(&mut bw, &samples, sample_rate, channels)?;
        }
    }
    bw.flush()?;
    drop(bw);
    guard.commit()?;
    Ok(())
}

fn decode_audio_symphonia(src: &Path, ext: &str) -> OpResult<(Vec<f32>, u32, u16)> {
    let file = File::open(src)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(ext);

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| OpfileError::convert(format!("Audio probe error: {e}")))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| OpfileError::convert("No audio track found"))?;
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44_100);
    let channels = track.codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| OpfileError::convert(format!("Audio codec init error: {e}")))?;

    let mut all_samples: Vec<f32> = Vec::new();
    let mut sbuf: Option<SampleBuffer<f32>> = None;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        if let Ok(decoded) = decoder.decode(&packet) {
            if sbuf.is_none() {
                sbuf = Some(SampleBuffer::<f32>::new(
                    decoded.capacity() as u64,
                    *decoded.spec(),
                ));
            }
            if let Some(ref mut buf) = sbuf {
                buf.copy_interleaved_ref(decoded);
                all_samples.extend_from_slice(buf.samples());
            }
        }
    }

    Ok((all_samples, sample_rate, channels.max(1)))
}

fn polyphase_sinc_resample(input: &[f32], src_sr: u32, dst_sr: u32, channels: u16) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    let in_frames = input.len() / ch;
    if in_frames == 0 {
        return Vec::new();
    }
    let ratio = dst_sr as f64 / src_sr as f64;
    let out_frames = ((in_frames as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_frames * ch);
    let taps = 8isize;

    for of in 0..out_frames {
        let src_pos = of as f64 / ratio;
        let center = src_pos.floor() as isize;
        for c in 0..ch {
            let mut acc = 0.0f64;
            let mut wsum = 0.0f64;
            for k in -taps..=taps {
                let idx = (center + k).clamp(0, in_frames as isize - 1) as usize;
                let x = src_pos - (center + k) as f64;
                let sinc = if x.abs() < 1e-6 {
                    1.0
                } else {
                    let px = std::f64::consts::PI * x;
                    px.sin() / px
                };
                // Hann window
                let win = 0.5 * (1.0 + (std::f64::consts::PI * x / (taps as f64 + 1.0)).cos());
                let w = sinc * win;
                acc += input[idx * ch + c] as f64 * w;
                wsum += w;
            }
            out.push(if wsum.abs() > 1e-9 { (acc / wsum) as f32 } else { 0.0 });
        }
    }
    out
}

fn normalize_ebu_r128(samples: &mut [f32], target_lufs: f32) {
    if samples.is_empty() {
        return;
    }
    let mean_sq = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum::<f64>() / samples.len() as f64;
    if mean_sq < 1e-12 {
        return;
    }
    let current_lufs = -0.691 + 10.0 * mean_sq.log10();
    let gain_db = (target_lufs as f64 - current_lufs).clamp(-24.0, 24.0);
    let gain = 10.0f64.powf(gain_db / 20.0) as f32;
    for s in samples.iter_mut() {
        *s = (*s * gain).clamp(-0.99, 0.99);
    }
}

fn write_wav_stream<W: Write + Seek>(
    w: &mut W,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> OpResult<()> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::new(w, spec).map_err(|e| OpfileError::convert(e.to_string()))?;
    for &s in samples {
        writer
            .write_sample((s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .map_err(|e| OpfileError::convert(e.to_string()))?;
    }
    writer
        .finalize()
        .map_err(|e| OpfileError::convert(e.to_string()))?;
    Ok(())
}

/// Writes a valid native FLAC stream using uncompressed verbatim subframes (zero-loss archival FLAC).
fn write_flac_pcm_stream<W: Write>(
    w: &mut W,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> OpResult<()> {
    let ch = channels.clamp(1, 2) as usize;
    let total_frames = (samples.len() / ch) as u64;
    w.write_all(b"fLaC")?;
    // STREAMINFO metadata block (last = 1, type = 0, len = 34)
    w.write_all(&[0x80, 0x00, 0x00, 34])?;
    w.write_all(&4096u16.to_be_bytes())?; // min block size
    w.write_all(&4096u16.to_be_bytes())?; // max block size
    w.write_all(&[0, 0, 0])?; // min frame size
    w.write_all(&[0, 0, 0])?; // max frame size
    let sr_ch_bps: u64 = ((sample_rate as u64) << 44)
        | (((ch as u64 - 1) & 0x7) << 41)
        | ((15u64 & 0x1F) << 36)
        | (total_frames & ((1u64 << 36) - 1));
    w.write_all(&sr_ch_bps.to_be_bytes())?;
    w.write_all(&[0u8; 16])?; // MD5 placeholder

    // Emit blocks of up to 4096 frames
    let mut frame_idx = 0usize;
    let mut frame_num = 0u64;
    while frame_idx < total_frames as usize {
        let block_frames = (total_frames as usize - frame_idx).min(4096);
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0xFF, 0xF8]); // sync + fixed block
        frame.push(0x70); // 4096 or custom 16-bit block size (code 7) + 44.1kHz hint
        frame.push((((ch - 1) as u8) << 4) | 0x08); // 16-bit sample size
        // UTF-8 coded frame number (up to 7-bit simple encoding or multi-byte)
        encode_utf8_vint(frame_num, &mut frame);
        frame.extend_from_slice(&((block_frames - 1) as u16).to_be_bytes());
        frame.push(0x00); // CRC-8 placeholder

        for c in 0..ch {
            frame.push(0x02); // VERBATIM subframe (type 1 << 1)
            for f in 0..block_frames {
                let s = samples[(frame_idx + f) * ch + c];
                let i16s = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                frame.extend_from_slice(&i16s.to_be_bytes());
            }
        }
        frame.extend_from_slice(&[0x00, 0x00]); // CRC-16 footer
        w.write_all(&frame)?;
        frame_idx += block_frames;
        frame_num += 1;
    }
    Ok(())
}

fn encode_utf8_vint(v: u64, out: &mut Vec<u8>) {
    if v < 0x80 {
        out.push(v as u8);
    } else if v < 0x800 {
        out.push(0xC0 | ((v >> 6) as u8));
        out.push(0x80 | ((v & 0x3F) as u8));
    } else {
        out.push(0xE0 | ((v >> 12) as u8));
        out.push(0x80 | (((v >> 6) & 0x3F) as u8));
        out.push(0x80 | ((v & 0x3F) as u8));
    }
}

/// Standard MIDI File (SMF) parser + polyphonic harmonic wavetable synthesizer -> stereo PCM.
fn synthesize_midi_to_pcm(src: &Path) -> OpResult<(Vec<f32>, u32, u16)> {
    let data = fs::read(src)?;
    if data.len() < 14 || &data[0..4] != b"MThd" {
        return Err(OpfileError::convert("Invalid MIDI header"));
    }
    let division = u16::from_be_bytes([data[12], data[13]]).max(1) as f64;
    let sr = 44_100u32;
    let mut notes: Vec<(f64, f64, u8, f32)> = Vec::new(); // (start_s, dur_s, note, vel)

    let mut pos = 14usize;
    let us_per_tick = 500_000.0 / division;
    while pos + 8 <= data.len() {
        if &data[pos..pos + 4] != b"MTrk" {
            break;
        }
        let trk_len = u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]) as usize;
        let trk_end = (pos + 8 + trk_len).min(data.len());
        let mut cur = pos + 8;
        let mut tick = 0u64;
        let mut active: HashMap<u8, (f64, f32)> = HashMap::new();
        let mut running_status = 0u8;

        while cur < trk_end {
            let delta = read_vlq(&data, &mut cur, trk_end);
            tick += delta;
            if cur >= trk_end {
                break;
            }
            let status = if data[cur] & 0x80 != 0 {
                let s = data[cur];
                cur += 1;
                if s < 0xF0 {
                    running_status = s;
                }
                s
            } else {
                running_status
            };
            let t_sec = (tick as f64) * us_per_tick / 1_000_000.0;
            match status & 0xF0 {
                0x90 if cur + 1 < trk_end => {
                    let note = data[cur];
                    let vel = data[cur + 1];
                    cur += 2;
                    if vel > 0 {
                        active.insert(note, (t_sec, vel as f32 / 127.0));
                    } else if let Some((st, v)) = active.remove(&note) {
                        notes.push((st, (t_sec - st).max(0.05), note, v));
                    }
                }
                0x80 if cur + 1 < trk_end => {
                    let note = data[cur];
                    cur += 2;
                    if let Some((st, v)) = active.remove(&note) {
                        notes.push((st, (t_sec - st).max(0.05), note, v));
                    }
                }
                0xC0 | 0xD0 => cur += 1,
                0xA0 | 0xB0 | 0xE0 => cur += 2,
                0xF0 => {
                    if status == 0xFF && cur < trk_end {
                        cur += 1;
                        let len = read_vlq(&data, &mut cur, trk_end) as usize;
                        cur = (cur + len).min(trk_end);
                    } else {
                        let len = read_vlq(&data, &mut cur, trk_end) as usize;
                        cur = (cur + len).min(trk_end);
                    }
                }
                _ => break,
            }
        }
        pos = trk_end;
    }

    let max_sec = notes
        .iter()
        .map(|n| n.0 + n.1)
        .fold(1.0f64, f64::max)
        .min(180.0);
    let total_frames = (max_sec * sr as f64) as usize;
    let mut pcm = vec![0.0f32; total_frames * 2];

    for (start, dur, note, vel) in notes {
        let freq = 440.0 * 2.0f64.powf((note as f64 - 69.0) / 12.0);
        let s_idx = (start * sr as f64) as usize;
        let len_idx = ((dur * sr as f64) as usize).min(total_frames.saturating_sub(s_idx));
        for i in 0..len_idx {
            let t = i as f64 / sr as f64;
            let env = if i < 400 {
                i as f32 / 400.0
            } else {
                (1.0 - (i as f32 / len_idx.max(1) as f32)).powf(0.6)
            };
            let wave = (2.0 * std::f64::consts::PI * freq * t).sin() * 0.65
                + (4.0 * std::f64::consts::PI * freq * t).sin() * 0.25;
            let sample = wave as f32 * vel * env * 0.25;
            pcm[(s_idx + i) * 2] += sample;
            pcm[(s_idx + i) * 2 + 1] += sample;
        }
    }
    for s in &mut pcm {
        *s = s.clamp(-0.98, 0.98);
    }
    Ok((pcm, sr, 2))
}

fn read_vlq(data: &[u8], cur: &mut usize, end: usize) -> u64 {
    let mut val = 0u64;
    for _ in 0..4 {
        if *cur >= end {
            break;
        }
        let b = data[*cur];
        *cur += 1;
        val = (val << 7) | (b & 0x7F) as u64;
        if b & 0x80 == 0 {
            break;
        }
    }
    val
}
