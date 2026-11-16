use crate::error::{OpResult, OpfileError};
use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

#[derive(Serialize)]
pub struct ApkManifestInfo {
    pub package_name: String,
    pub version_name: String,
    pub version_code: String,
    pub min_sdk: String,
    pub target_sdk: String,
    pub permissions: Vec<String>,
    pub activities: Vec<String>,
    pub services: Vec<String>,
    pub is_xapk: bool,
    pub split_apks: Vec<String>,
    pub decoded_xml: String,
}

pub fn inspect_apk(path: &str) -> OpResult<ApkManifestInfo> {
    let file = File::open(path)?;
    let mut zip = ZipArchive::new(file).map_err(|e| OpfileError::archive(format!("Invalid APK/XAPK: {e}")))?;

    let mut split_apks = Vec::new();
    let mut is_xapk = false;
    let mut manifest_bytes = Vec::new();

    for i in 0..zip.len() {
        let name = zip
            .by_index(i)
            .map(|f| f.name().to_string())
            .unwrap_or_default();
        if name.ends_with(".apk") {
            is_xapk = true;
            split_apks.push(name.clone());
        }
    }

    let has_direct_manifest = match zip.by_name("AndroidManifest.xml") {
        Ok(mut mf) => {
            mf.read_to_end(&mut manifest_bytes)?;
            true
        }
        Err(_) => false,
    };

    if !has_direct_manifest && is_xapk {
        // Look inside base.apk or first embedded APK
        let base_name = split_apks
            .iter()
            .find(|n| n.ends_with("base.apk") || n.contains("base"))
            .or_else(|| split_apks.first())
            .cloned();
        if let Some(bn) = base_name {
            let mut inner_bytes = Vec::new();
            zip.by_name(&bn)
                .map_err(|e| OpfileError::archive(e.to_string()))?
                .read_to_end(&mut inner_bytes)?;
            let cursor = std::io::Cursor::new(inner_bytes);
            if let Ok(mut inner_zip) = ZipArchive::new(cursor) {
                if let Ok(mut mf) = inner_zip.by_name("AndroidManifest.xml") {
                    mf.read_to_end(&mut manifest_bytes)?;
                }
            }
        }
    }

    if manifest_bytes.is_empty() {
        // Check XAPK manifest.json fallback
        if let Ok(mut mj) = zip.by_name("manifest.json") {
            let mut s = String::new();
            mj.read_to_string(&mut s)?;
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
            return Ok(ApkManifestInfo {
                package_name: v["package_name"].as_str().unwrap_or("unknown").to_string(),
                version_name: v["version_name"].as_str().unwrap_or("").to_string(),
                version_code: v["version_code"].to_string(),
                min_sdk: v["min_sdk_version"].as_str().unwrap_or("").to_string(),
                target_sdk: v["target_sdk_version"].as_str().unwrap_or("").to_string(),
                permissions: v["permissions"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                    .unwrap_or_default(),
                activities: Vec::new(),
                services: Vec::new(),
                is_xapk: true,
                split_apks,
                decoded_xml: s,
            });
        }
        return Err(OpfileError::archive("AndroidManifest.xml not found in package"));
    }

    parse_axml(&manifest_bytes, is_xapk, split_apks)
}

fn read_u16(b: &[u8], off: usize) -> u16 {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .unwrap_or(0)
}

fn read_u32(b: &[u8], off: usize) -> u32 {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .unwrap_or(0)
}

fn parse_axml(data: &[u8], is_xapk: bool, split_apks: Vec<String>) -> OpResult<ApkManifestInfo> {
    if data.len() < 8 {
        return Err(OpfileError::archive("Truncated AXML header"));
    }
    let magic = read_u16(data, 0);
    if magic != 0x0003 {
        // Might already be plain XML
        let text = String::from_utf8_lossy(data).into_owned();
        return Ok(ApkManifestInfo {
            package_name: String::new(),
            version_name: String::new(),
            version_code: String::new(),
            min_sdk: String::new(),
            target_sdk: String::new(),
            permissions: Vec::new(),
            activities: Vec::new(),
            services: Vec::new(),
            is_xapk,
            split_apks,
            decoded_xml: text,
        });
    }

    let mut pos = 8usize;
    let mut strings: Vec<String> = Vec::new();
    let mut xml_out = String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    let mut indent = 0usize;

    let mut package_name = String::new();
    let mut version_name = String::new();
    let mut version_code = String::new();
    let mut min_sdk = String::new();
    let mut target_sdk = String::new();
    let mut permissions = Vec::new();
    let mut activities = Vec::new();
    let mut services = Vec::new();
    let mut seen_perms = HashSet::new();

    while pos + 8 <= data.len() {
        let chunk_type = read_u16(data, pos);
        let header_size = read_u16(data, pos + 2) as usize;
        let chunk_size = read_u32(data, pos + 4) as usize;
        if chunk_size < 8 || pos + chunk_size > data.len() {
            break;
        }
        let chunk = &data[pos..pos + chunk_size];

        match chunk_type {
            // RES_STRING_POOL_TYPE = 0x0001
            0x0001 if chunk.len() >= 28 => {
                let string_count = read_u32(chunk, 8) as usize;
                let flags = read_u32(chunk, 16);
                let strings_start = read_u32(chunk, 20) as usize;
                let is_utf8 = (flags & (1 << 8)) != 0;
                strings.clear();
                for i in 0..string_count {
                    let off_pos = 28 + i * 4;
                    if off_pos + 4 > chunk.len() {
                        break;
                    }
                    let str_off = strings_start + read_u32(chunk, off_pos) as usize;
                    if str_off >= chunk.len() {
                        strings.push(String::new());
                        continue;
                    }
                    if is_utf8 {
                        let mut p = str_off;
                        let (_c_len, step1) = read_len8(&chunk[p..]);
                        p += step1;
                        let (b_len, step2) = read_len8(&chunk[p..]);
                        p += step2;
                        let end = (p + b_len).min(chunk.len());
                        strings.push(String::from_utf8_lossy(&chunk[p..end]).into_owned());
                    } else {
                        let (c_len, step) = read_len16(&chunk[str_off..]);
                        let p = str_off + step;
                        let byte_len = c_len * 2;
                        let end = (p + byte_len).min(chunk.len());
                        let (chunks, _) = chunk[p..end].as_chunks::<2>();
                        let u16s: Vec<u16> = chunks
                            .iter()
                            .map(|c| u16::from_le_bytes(*c))
                            .collect();
                        strings.push(String::from_utf16_lossy(&u16s));
                    }
                }
            }
            // RES_XML_START_ELEMENT_TYPE = 0x0102
            0x0102 if chunk.len() >= 36 => {
                let name_idx = read_u32(chunk, 20) as usize;
                let attr_start = read_u16(chunk, 24) as usize;
                let attr_size = read_u16(chunk, 26) as usize;
                let attr_count = read_u16(chunk, 28) as usize;
                let tag = strings.get(name_idx).cloned().unwrap_or_else(|| "tag".to_string());

                for _ in 0..indent {
                    xml_out.push_str("  ");
                }
                xml_out.push('<');
                xml_out.push_str(&tag);

                let base = header_size.max(16) + attr_start.saturating_sub(20);
                let mut attr_pos = if base < chunk.len() { base } else { 36 };
                let step = attr_size.max(20);

                for _ in 0..attr_count {
                    if attr_pos + 20 > chunk.len() {
                        break;
                    }
                    let a_name_idx = read_u32(chunk, attr_pos + 4) as usize;
                    let a_raw_idx = read_u32(chunk, attr_pos + 8) as usize;
                    let val_type = chunk.get(attr_pos + 15).copied().unwrap_or(0);
                    let val_data = read_u32(chunk, attr_pos + 16);

                    let attr_name = strings
                        .get(a_name_idx)
                        .cloned()
                        .unwrap_or_else(|| format!("attr_{a_name_idx}"));

                    let attr_val = if a_raw_idx != u32::MAX as usize {
                        strings.get(a_raw_idx).cloned().unwrap_or_default()
                    } else {
                        match val_type {
                            0x03 => strings.get(val_data as usize).cloned().unwrap_or_default(),
                            0x10 | 0x11 => (val_data as i32).to_string(),
                            0x12 => if val_data != 0 { "true" } else { "false" }.to_string(),
                            _ => format!("0x{val_data:x}"),
                        }
                    };

                    xml_out.push_str(&format!(" {attr_name}=\"{attr_val}\""));

                    match (tag.as_str(), attr_name.as_str()) {
                        ("manifest", "package") => package_name = attr_val,
                        ("manifest", "versionName") => version_name = attr_val,
                        ("manifest", "versionCode") => version_code = attr_val,
                        ("uses-sdk", "minSdkVersion") => min_sdk = attr_val,
                        ("uses-sdk", "targetSdkVersion") => target_sdk = attr_val,
                        ("uses-permission", "name") => {
                            if seen_perms.insert(attr_val.clone()) {
                                permissions.push(attr_val);
                            }
                        }
                        ("activity", "name") | ("activity-alias", "name") => activities.push(attr_val),
                        ("service", "name") => services.push(attr_val),
                        _ => {}
                    }

                    attr_pos += step;
                }
                xml_out.push_str(">\n");
                indent += 1;
            }
            // RES_XML_END_ELEMENT_TYPE = 0x0103
            0x0103 if chunk.len() >= 24 => {
                indent = indent.saturating_sub(1);
                let name_idx = read_u32(chunk, 20) as usize;
                let tag = strings.get(name_idx).cloned().unwrap_or_else(|| "tag".to_string());
                for _ in 0..indent {
                    xml_out.push_str("  ");
                }
                xml_out.push_str(&format!("</{tag}>\n"));
            }
            _ => {}
        }

        pos += chunk_size;
    }

    Ok(ApkManifestInfo {
        package_name,
        version_name,
        version_code,
        min_sdk,
        target_sdk,
        permissions,
        activities,
        services,
        is_xapk,
        split_apks,
        decoded_xml: xml_out,
    })
}

fn read_len8(b: &[u8]) -> (usize, usize) {
    let first = b.first().copied().unwrap_or(0) as usize;
    if first & 0x80 != 0 {
        let second = b.get(1).copied().unwrap_or(0) as usize;
        (((first & 0x7F) << 8) | second, 2)
    } else {
        (first, 1)
    }
}

fn read_len16(b: &[u8]) -> (usize, usize) {
    let first = read_u16(b, 0) as usize;
    if first & 0x8000 != 0 {
        let second = read_u16(b, 2) as usize;
        (((first & 0x7FFF) << 16) | second, 4)
    } else {
        (first, 2)
    }
}
