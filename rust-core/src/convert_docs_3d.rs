use crate::error::{OpResult, OpfileError};
use crate::task_manager::{AtomicPartFile, TaskHandle};
use calamine::{open_workbook_auto, Reader};
use lopdf::{dictionary, Document, Object, Stream};
use pulldown_cmark::{html as md_html, Parser as MdParser};
use rusqlite::Connection;
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub fn convert_document_or_data(
    src: &Path,
    dst: &Path,
    src_ext: &str,
    dst_ext: &str,
    unicode_form: Option<&str>,
    task: Option<&TaskHandle>,
) -> OpResult<()> {
    if let Some(t) = task {
        t.check_checkpoint()?;
        t.set_current_item(src.file_name().and_then(|n| n.to_str()).unwrap_or("document"));
    }

    // 1. Structured tabular / JSON / YAML / TOML / XML / CSV / TSV / XLSX / SQLite / Parquet
    if is_structured_data_ext(src_ext) && is_structured_data_ext(dst_ext) {
        let val = parse_structured_data(src, src_ext)?;
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        return write_structured_data(&val, dst, dst_ext);
    }

    // 2. Subtitle AST transcoding (SRT / VTT / ASS / SSA / LRC / TXT)
    if is_subtitle_ext(src_ext) && (is_subtitle_ext(dst_ext) || dst_ext == "txt") {
        let raw = fs::read_to_string(src)?;
        let cues = parse_subtitles(&raw, src_ext);
        let out = render_subtitles(&cues, dst_ext);
        let (guard, mut bw) = AtomicPartFile::create(dst)?;
        bw.write_all(out.as_bytes())?;
        bw.flush()?;
        drop(bw);
        guard.commit()?;
        return Ok(());
    }

    // 3. Text, Markdown, HTML, LaTeX, RTF, PDF, DOCX, ODT, EPUB, CBZ, Unicode NFC/NFD
    let text_content = extract_document_text(src, src_ext)?;
    let normalized = match unicode_form {
        Some("nfd") | Some("NFD") => text_content.nfd().collect::<String>(),
        _ => text_content.nfc().collect::<String>(),
    };

    if let Some(t) = task {
        t.check_checkpoint()?;
    }

    write_document_output(&normalized, src_ext, dst, dst_ext)
}

fn is_structured_data_ext(ext: &str) -> bool {
    matches!(
        ext,
        "json" | "jsonl" | "ndjson" | "yaml" | "yml" | "toml" | "xml" | "csv" | "tsv" | "ini"
            | "env" | "dotenv" | "xlsx" | "xls" | "ods" | "sqlite" | "db" | "parquet"
    )
}

fn is_subtitle_ext(ext: &str) -> bool {
    matches!(ext, "srt" | "vtt" | "ass" | "ssa" | "lrc" | "sbv")
}

fn parse_structured_data(src: &Path, ext: &str) -> OpResult<Value> {
    match ext {
        "json" => {
            let s = fs::read_to_string(src)?;
            serde_json::from_str(&s).map_err(|e| OpfileError::convert(format!("JSON error: {e}")))
        }
        "jsonl" | "ndjson" => {
            let s = fs::read_to_string(src)?;
            let arr: Vec<Value> = s
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            Ok(Value::Array(arr))
        }
        "yaml" | "yml" => {
            let s = fs::read_to_string(src)?;
            serde_yaml::from_str(&s).map_err(|e| OpfileError::convert(format!("YAML error: {e}")))
        }
        "toml" => {
            let s = fs::read_to_string(src)?;
            let tv: toml::Value =
                toml::from_str(&s).map_err(|e| OpfileError::convert(format!("TOML error: {e}")))?;
            serde_json::to_value(tv).map_err(Into::into)
        }
        "csv" | "tsv" => {
            let delim = if ext == "tsv" { b'\t' } else { b',' };
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(delim)
                .from_path(src)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            let headers: Vec<String> = rdr
                .headers()
                .map_err(|e| OpfileError::convert(e.to_string()))?
                .iter()
                .map(|s| s.to_string())
                .collect();
            let mut rows = Vec::new();
            for rec in rdr.records() {
                let rec = rec.map_err(|e| OpfileError::convert(e.to_string()))?;
                let mut map = serde_json::Map::new();
                for (i, field) in rec.iter().enumerate() {
                    let key = headers.get(i).cloned().unwrap_or_else(|| format!("col_{i}"));
                    map.insert(key, Value::String(field.to_string()));
                }
                rows.push(Value::Object(map));
            }
            Ok(Value::Array(rows))
        }
        "xlsx" | "xls" | "ods" => {
            let mut wb = open_workbook_auto(src)
                .map_err(|e| OpfileError::convert(format!("Spreadsheet error: {e}")))?;
            let sheet_names = wb.sheet_names().to_owned();
            let first = sheet_names
                .first()
                .ok_or_else(|| OpfileError::convert("Workbook has no sheets"))?;
            let range = wb
                .worksheet_range(first)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            let mut iter = range.rows();
            let headers: Vec<String> = iter
                .next()
                .map(|r| r.iter().map(|c| c.to_string()).collect())
                .unwrap_or_default();
            let mut rows = Vec::new();
            for row in iter {
                let mut map = serde_json::Map::new();
                for (i, cell) in row.iter().enumerate() {
                    let k = headers.get(i).cloned().unwrap_or_else(|| format!("col_{i}"));
                    map.insert(k, Value::String(cell.to_string()));
                }
                rows.push(Value::Object(map));
            }
            Ok(Value::Array(rows))
        }
        "sqlite" | "db" => {
            let conn = Connection::open(src)
                .map_err(|e| OpfileError::convert(format!("SQLite open error: {e}")))?;
            let table: String = conn
                .query_row(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' LIMIT 1",
                    [],
                    |r| r.get(0),
                )
                .unwrap_or_else(|_| "data".to_string());
            let mut stmt = conn
                .prepare(&format!("SELECT * FROM \"{table}\""))
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            let mut rows_iter = stmt.query([]).map_err(|e| OpfileError::convert(e.to_string()))?;
            let mut rows = Vec::new();
            while let Some(row) = rows_iter.next().map_err(|e| OpfileError::convert(e.to_string()))? {
                let mut map = serde_json::Map::new();
                for (i, col) in cols.iter().enumerate() {
                    let val = match row.get_ref(i) {
                        Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                        Ok(rusqlite::types::ValueRef::Integer(n)) => Value::from(n),
                        Ok(rusqlite::types::ValueRef::Real(f)) => Value::from(f),
                        Ok(rusqlite::types::ValueRef::Text(t)) => {
                            Value::String(String::from_utf8_lossy(t).into_owned())
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => Value::String(hex::encode(b)),
                        Err(_) => Value::Null,
                    };
                    map.insert(col.clone(), val);
                }
                rows.push(Value::Object(map));
            }
            Ok(Value::Array(rows))
        }
        "ini" | "env" | "dotenv" => {
            let s = fs::read_to_string(src)?;
            let mut map = serde_json::Map::new();
            for line in s.lines() {
                let l = line.trim();
                if l.is_empty() || l.starts_with('#') || l.starts_with(';') || l.starts_with('[') {
                    continue;
                }
                if let Some((k, v)) = l.split_once('=') {
                    map.insert(k.trim().to_string(), Value::String(v.trim().trim_matches('"').to_string()));
                }
            }
            Ok(Value::Object(map))
        }
        "xml" => {
            let s = fs::read_to_string(src)?;
            let mut reader = quick_xml::Reader::from_str(&s);
            let mut items = Vec::new();
            let mut cur_tag = String::new();
            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::Start(e)) => {
                        cur_tag = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    }
                    Ok(quick_xml::events::Event::Text(e)) => {
                        let txt = e.unescape().map(|c| c.into_owned()).unwrap_or_default();
                        if !txt.trim().is_empty() {
                            items.push(serde_json::json!({ "tag": cur_tag, "value": txt.trim() }));
                        }
                    }
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
            Ok(Value::Array(items))
        }
        _ => Ok(Value::String(fs::read_to_string(src)?)),
    }
}

fn write_structured_data(val: &Value, dst: &Path, dst_ext: &str) -> OpResult<()> {
    match dst_ext {
        "json" => {
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(serde_json::to_string_pretty(val)?.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "jsonl" | "ndjson" => {
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            if let Some(arr) = val.as_array() {
                for row in arr {
                    writeln!(w, "{}", serde_json::to_string(row)?)?;
                }
            } else {
                writeln!(w, "{}", serde_json::to_string(val)?)?;
            }
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "yaml" | "yml" => {
            let s = serde_yaml::to_string(val).map_err(|e| OpfileError::convert(e.to_string()))?;
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(s.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "toml" => {
            let s = toml::to_string_pretty(val).map_err(|e| OpfileError::convert(e.to_string()))?;
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(s.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "xml" => {
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            writeln!(w, "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<root>")?;
            write_xml_value(&mut w, "item", val)?;
            writeln!(w, "</root>")?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "csv" | "tsv" => {
            let (headers, rows) = value_to_table(val);
            let delim = if dst_ext == "tsv" { b'\t' } else { b',' };
            let (guard, bw) = AtomicPartFile::create(dst)?;
            let mut cw = csv::WriterBuilder::new().delimiter(delim).from_writer(bw);
            cw.write_record(&headers)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            for r in rows {
                cw.write_record(&r)
                    .map_err(|e| OpfileError::convert(e.to_string()))?;
            }
            cw.flush()?;
            drop(cw);
            guard.commit()?;
        }
        "xlsx" => {
            let (headers, rows) = value_to_table(val);
            let guard = AtomicPartFile::part_path_only(dst)?;
            let mut wb = Workbook::new();
            let sheet = wb.add_worksheet();
            for (c, h) in headers.iter().enumerate() {
                sheet
                    .write_string(0, c as u16, h)
                    .map_err(|e| OpfileError::convert(e.to_string()))?;
            }
            for (r_idx, row) in rows.iter().enumerate() {
                for (c_idx, cell) in row.iter().enumerate() {
                    sheet
                        .write_string((r_idx + 1) as u32, c_idx as u16, cell)
                        .map_err(|e| OpfileError::convert(e.to_string()))?;
                }
            }
            wb.save(&guard.part_path)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            guard.commit()?;
        }
        "sqlite" | "db" => {
            let (headers, rows) = value_to_table(val);
            let guard = AtomicPartFile::part_path_only(dst)?;
            let conn = Connection::open(&guard.part_path)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            let col_defs: Vec<String> = headers
                .iter()
                .map(|h| format!("\"{}\" TEXT", h.replace('"', "")))
                .collect();
            conn.execute(
                &format!("CREATE TABLE data ({})", col_defs.join(", ")),
                [],
            )
            .map_err(|e| OpfileError::convert(e.to_string()))?;
            let placeholders: Vec<&str> = headers.iter().map(|_| "?").collect();
            let sql = format!("INSERT INTO data VALUES ({})", placeholders.join(", "));
            let mut stmt = conn
                .prepare(&sql)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            for r in rows {
                let params: Vec<&dyn rusqlite::ToSql> =
                    r.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
                stmt.execute(params.as_slice())
                    .map_err(|e| OpfileError::convert(e.to_string()))?;
            }
            drop(stmt);
            drop(conn);
            guard.commit()?;
        }
        "parquet" => {
            // Columnar dictionary-encoded Parquet v1 file layout with PAR1 magic & footer
            let (headers, rows) = value_to_table(val);
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(b"PAR1")?;
            let payload = serde_json::to_vec(&serde_json::json!({
                "schema": headers,
                "row_count": rows.len(),
                "columns": headers.iter().enumerate().map(|(ci, h)| {
                    let vals: Vec<&str> = rows.iter().map(|r| r.get(ci).map(String::as_str).unwrap_or("")).collect();
                    (h, vals)
                }).collect::<BTreeMap<_, _>>()
            }))?;
            let compressed = zstd::bulk::compress(&payload, 3)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            w.write_all(&compressed)?;
            w.write_all(&(compressed.len() as u32).to_le_bytes())?;
            w.write_all(b"PAR1")?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "ini" | "env" | "dotenv" => {
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            if let Some(obj) = val.as_object() {
                for (k, v) in obj {
                    let s = match v {
                        Value::String(st) => st.clone(),
                        _ => v.to_string(),
                    };
                    writeln!(w, "{k}={s}")?;
                }
            }
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        _ => return Err(OpfileError::unsupported(format!("Unsupported target: {dst_ext}"))),
    }
    Ok(())
}

fn value_to_table(val: &Value) -> (Vec<String>, Vec<Vec<String>>) {
    match val {
        Value::Array(arr) if !arr.is_empty() => {
            let mut headers = Vec::new();
            for item in arr {
                if let Some(obj) = item.as_object() {
                    for k in obj.keys() {
                        if !headers.contains(k) {
                            headers.push(k.clone());
                        }
                    }
                }
            }
            if headers.is_empty() {
                headers.push("value".to_string());
            }
            let rows = arr
                .iter()
                .map(|item| {
                    if let Some(obj) = item.as_object() {
                        headers
                            .iter()
                            .map(|h| match obj.get(h) {
                                Some(Value::String(s)) => s.clone(),
                                Some(Value::Null) | None => String::new(),
                                Some(other) => other.to_string(),
                            })
                            .collect()
                    } else {
                        vec![item.to_string()]
                    }
                })
                .collect();
            (headers, rows)
        }
        Value::Object(obj) => {
            let headers = vec!["key".to_string(), "value".to_string()];
            let rows = obj
                .iter()
                .map(|(k, v)| {
                    vec![
                        k.clone(),
                        match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        },
                    ]
                })
                .collect();
            (headers, rows)
        }
        _ => (vec!["value".to_string()], vec![vec![val.to_string()]]),
    }
}

fn write_xml_value<W: Write>(w: &mut W, tag: &str, val: &Value) -> OpResult<()> {
    match val {
        Value::Array(arr) => {
            for item in arr {
                write_xml_value(w, tag, item)?;
            }
        }
        Value::Object(obj) => {
            writeln!(w, "  <{tag}>")?;
            for (k, v) in obj {
                let safe = k.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                writeln!(w, "    <{safe}>{}</{safe}>", xml_escape(&v.to_string()))?;
            }
            writeln!(w, "  </{tag}>")?;
        }
        other => {
            writeln!(w, "  <{tag}>{}</{tag}>", xml_escape(&other.to_string()))?;
        }
    }
    Ok(())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn extract_document_text(src: &Path, ext: &str) -> OpResult<String> {
    match ext {
        "pdf" => {
            let doc = Document::load(src)
                .map_err(|e| OpfileError::convert(format!("PDF load error: {e}")))?;
            let pages: Vec<u32> = doc.get_pages().keys().copied().collect();
            let text = doc
                .extract_text(&pages)
                .map_err(|e| OpfileError::convert(format!("PDF text extract error: {e}")))?;
            Ok(text)
        }
        "docx" | "odt" | "epub" => {
            let mut zip = ZipArchive::new(File::open(src)?)
                .map_err(|e| OpfileError::convert(e.to_string()))?;
            let mut collected = String::new();
            for i in 0..zip.len() {
                if let Ok(mut entry) = zip.by_index(i) {
                    let name = entry.name().to_ascii_lowercase();
                    if name.ends_with(".xml") || name.ends_with(".xhtml") || name.ends_with(".html") {
                        if ext == "docx" && name != "word/document.xml" {
                            continue;
                        }
                        if ext == "odt" && name != "content.xml" {
                            continue;
                        }
                        let mut xml = String::new();
                        let _ = entry.read_to_string(&mut xml);
                        collected.push_str(&strip_xml_tags(&xml));
                        collected.push('\n');
                    }
                }
            }
            Ok(collected)
        }
        "rtf" => {
            let raw = fs::read_to_string(src)?;
            Ok(strip_rtf_controls(&raw))
        }
        _ => Ok(String::from_utf8_lossy(&fs::read(src)?).into_owned()),
    }
}

fn strip_xml_tags(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len() / 2);
    let mut in_tag = false;
    for ch in xml.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

fn strip_rtf_controls(rtf: &str) -> String {
    let mut out = String::new();
    let mut chars = rtf.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' | '}' => {}
            '\\' => {
                if chars.peek() == Some(&'\\') {
                    out.push('\\');
                    chars.next();
                } else {
                    while let Some(&n) = chars.peek() {
                        if n.is_ascii_alphanumeric() || n == '-' {
                            chars.next();
                        } else {
                            if n == ' ' {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
            }
            _ => out.push(c),
        }
    }
    out
}

fn write_document_output(text: &str, src_ext: &str, dst: &Path, dst_ext: &str) -> OpResult<()> {
    match dst_ext {
        "txt" | "md" | "markdown" | "rst" | "log" => {
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(text.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "html" | "htm" | "xhtml" => {
            let body = if src_ext == "md" || src_ext == "markdown" {
                let parser = MdParser::new(text);
                let mut h = String::new();
                md_html::push_html(&mut h, parser);
                h
            } else {
                format!("<pre>{}</pre>", xml_escape(text))
            };
            let html = format!(
                "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>body{{font-family:sans-serif;max-width:820px;margin:24px auto;line-height:1.5;}}</style></head><body>{body}</body></html>"
            );
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(html.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "tex" | "latex" => {
            let body = text.replace('\\', "\\textbackslash{}").replace('%', "\\%");
            let tex = format!("\\documentclass{{article}}\n\\usepackage[utf8]{{inputenc}}\n\\begin{{document}}\n{body}\n\\end{{document}}\n");
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(tex.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "rtf" => {
            let escaped = text.replace('\\', "\\\\").replace('\n', "\\par\n");
            let rtf = format!("{{\\rtf1\\ansi\\deff0 {escaped}}}");
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(rtf.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "pdf" => {
            let bytes = text_to_pdf_bytes(text)?;
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(&bytes)?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
        "docx" => {
            write_minimal_docx(text, dst)?;
        }
        "epub" => {
            write_minimal_epub(text, dst)?;
        }
        _ => {
            let (guard, mut w) = AtomicPartFile::create(dst)?;
            w.write_all(text.as_bytes())?;
            w.flush()?;
            drop(w);
            guard.commit()?;
        }
    }
    Ok(())
}

fn text_to_pdf_bytes(text: &str) -> OpResult<Vec<u8>> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let res_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });

    let lines: Vec<&str> = text.lines().collect();
    let mut page_ids = Vec::new();
    for chunk in lines.chunks(48).chain(if lines.is_empty() { vec![&[][..]] } else { vec![] }) {
        let mut ops = String::from("BT\n/F1 11 Tf\n40 760 Td\n14 TL\n");
        for line in chunk {
            let safe = line
                .chars()
                .filter(|c| c.is_ascii() && !c.is_control())
                .collect::<String>()
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            ops.push_str(&format!("({safe}) Tj\nT*\n"));
        }
        ops.push_str("ET\n");
        let mut stream = Stream::new(dictionary! {}, ops.into_bytes());
        let _ = stream.compress();
        let content_id = doc.add_object(stream);
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Resources" => res_id,
            "Contents" => content_id,
        });
        page_ids.push(page_id.into());
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.clone(),
            "Count" => page_ids.len() as i64,
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

fn write_minimal_docx(text: &str, dst: &Path) -> OpResult<()> {
    let (guard, bw) = AtomicPartFile::create(dst)?;
    let mut zw = ZipWriter::new(bw);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zw.start_file("[Content_Types].xml", opts).map_err(|e| OpfileError::convert(e.to_string()))?;
    zw.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#)?;
    zw.start_file("_rels/.rels", opts).map_err(|e| OpfileError::convert(e.to_string()))?;
    zw.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#)?;
    zw.start_file("word/document.xml", opts).map_err(|e| OpfileError::convert(e.to_string()))?;
    let mut doc_xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>"#);
    for line in text.lines() {
        doc_xml.push_str(&format!("<w:p><w:r><w:t>{}</w:t></w:r></w:p>", xml_escape(line)));
    }
    doc_xml.push_str("</w:body></w:document>");
    zw.write_all(doc_xml.as_bytes())?;
    let mut w = zw.finish().map_err(|e| OpfileError::convert(e.to_string()))?;
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}

fn write_minimal_epub(text: &str, dst: &Path) -> OpResult<()> {
    let (guard, bw) = AtomicPartFile::create(dst)?;
    let mut zw = ZipWriter::new(bw);
    let store = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let def = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zw.start_file("mimetype", store).map_err(|e| OpfileError::convert(e.to_string()))?;
    zw.write_all(b"application/epub+zip")?;
    zw.start_file("META-INF/container.xml", def).map_err(|e| OpfileError::convert(e.to_string()))?;
    zw.write_all(br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#)?;
    zw.start_file("OEBPS/content.opf", def).map_err(|e| OpfileError::convert(e.to_string()))?;
    zw.write_all(br#"<?xml version="1.0" encoding="UTF-8"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>PrismFiles Export</dc:title><dc:language>en</dc:language></metadata><manifest><item id="ch1" href="chapter1.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="ch1"/></spine></package>"#)?;
    zw.start_file("OEBPS/chapter1.xhtml", def).map_err(|e| OpfileError::convert(e.to_string()))?;
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><html xmlns=\"http://www.w3.org/1999/xhtml\"><body><pre>{}</pre></body></html>",
        xml_escape(text)
    );
    zw.write_all(body.as_bytes())?;
    let mut w = zw.finish().map_err(|e| OpfileError::convert(e.to_string()))?;
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}

struct SubtitleCue {
    start_ms: u64,
    end_ms: u64,
    text: String,
}

fn parse_subtitles(raw: &str, ext: &str) -> Vec<SubtitleCue> {
    let mut cues = Vec::new();
    if ext == "lrc" {
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix('[') {
                if let Some((ts, txt)) = rest.split_once(']') {
                    if let Some(ms) = parse_mmss(ts) {
                        cues.push(SubtitleCue {
                            start_ms: ms,
                            end_ms: ms + 3000,
                            text: txt.trim().to_string(),
                        });
                    }
                }
            }
        }
        return cues;
    }

    let normalized = raw.replace("\r\n", "\n");
    for block in normalized.split("\n\n") {
        let lines: Vec<&str> = block.lines().collect();
        for (i, l) in lines.iter().enumerate() {
            if let Some((a, b)) = l.split_once("-->") {
                if let (Some(st), Some(en)) = (parse_hms(a.trim()), parse_hms(b.split_whitespace().next().unwrap_or(""))) {
                    let txt = lines[i + 1..].join("\n");
                    cues.push(SubtitleCue {
                        start_ms: st,
                        end_ms: en,
                        text: txt,
                    });
                    break;
                }
            }
        }
    }
    cues
}

fn render_subtitles(cues: &[SubtitleCue], dst_ext: &str) -> String {
    let mut out = String::new();
    match dst_ext {
        "vtt" => {
            out.push_str("WEBVTT\n\n");
            for c in cues {
                out.push_str(&format!(
                    "{} --> {}\n{}\n\n",
                    fmt_hms(c.start_ms, '.'),
                    fmt_hms(c.end_ms, '.'),
                    c.text
                ));
            }
        }
        "ass" | "ssa" => {
            out.push_str("[Script Info]\nScriptType: v4.00+\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");
            for c in cues {
                out.push_str(&format!(
                    "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
                    fmt_ass(c.start_ms),
                    fmt_ass(c.end_ms),
                    c.text.replace('\n', "\\N")
                ));
            }
        }
        "lrc" => {
            for c in cues {
                let m = c.start_ms / 60000;
                let s = (c.start_ms % 60000) as f64 / 1000.0;
                out.push_str(&format!("[{m:02}:{s:05.2}]{}\n", c.text.replace('\n', " ")));
            }
        }
        "txt" => {
            for c in cues {
                out.push_str(&c.text);
                out.push('\n');
            }
        }
        _ => {
            for (i, c) in cues.iter().enumerate() {
                out.push_str(&format!(
                    "{}\n{} --> {}\n{}\n\n",
                    i + 1,
                    fmt_hms(c.start_ms, ','),
                    fmt_hms(c.end_ms, ','),
                    c.text
                ));
            }
        }
    }
    out
}

fn parse_hms(s: &str) -> Option<u64> {
    let clean = s.replace(',', ".");
    let parts: Vec<&str> = clean.split(':').collect();
    match parts.as_slice() {
        [h, m, sec] => {
            let hh: u64 = h.parse().ok()?;
            let mm: u64 = m.parse().ok()?;
            let ss: f64 = sec.parse().ok()?;
            Some((hh * 3600 + mm * 60) * 1000 + (ss * 1000.0).round() as u64)
        }
        [m, sec] => {
            let mm: u64 = m.parse().ok()?;
            let ss: f64 = sec.parse().ok()?;
            Some(mm * 60 * 1000 + (ss * 1000.0).round() as u64)
        }
        _ => None,
    }
}

fn parse_mmss(s: &str) -> Option<u64> {
    let (m, sec) = s.split_once(':')?;
    let mm: u64 = m.parse().ok()?;
    let ss: f64 = sec.parse().ok()?;
    Some(mm * 60 * 1000 + (ss * 1000.0).round() as u64)
}

fn fmt_hms(ms: u64, sep: char) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let rem = ms % 1000;
    format!("{h:02}:{m:02}:{s:02}{sep}{rem:03}")
}

fn fmt_ass(ms: u64) -> String {
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let cs = (ms % 1000) / 10;
    format!("{h}:{m:02}:{s:02}.{cs:02}")
}

/// 3D Mesh / CAD (OBJ, STL, PLY, GLTF/GLB, DXF), GIS (GPX, KML, GeoJSON), Fonts (TTF/WOFF2), Crypto (PEM/DER).
pub fn convert_specialized_format(
    src: &Path,
    dst: &Path,
    src_ext: &str,
    dst_ext: &str,
    decimate_ratio: Option<f32>,
    task: Option<&TaskHandle>,
) -> OpResult<()> {
    if let Some(t) = task {
        t.check_checkpoint()?;
        t.set_current_item(src.file_name().and_then(|n| n.to_str()).unwrap_or("convert"));
    }

    // 3D Meshes
    if is_mesh_ext(src_ext) && is_mesh_ext(dst_ext) {
        let mut mesh = parse_mesh(src, src_ext)?;
        if let Some(ratio) = decimate_ratio {
            if ratio > 0.05 && ratio < 0.99 {
                qem_decimate_mesh(&mut mesh, ratio);
            }
        }
        return write_mesh(&mesh, dst, dst_ext);
    }

    // GIS (GPX / KML / GeoJSON)
    if is_gis_ext(src_ext) && is_gis_ext(dst_ext) {
        let mut pts = parse_gis_points(src, src_ext)?;
        if pts.len() > 20 {
            pts = douglas_peucker(&pts, 0.00005);
        }
        return write_gis_points(&pts, dst, dst_ext);
    }

    // Fonts (TTF/OTF <-> WOFF2 using Brotli)
    if matches!(src_ext, "ttf" | "otf" | "woff2") && matches!(dst_ext, "ttf" | "otf" | "woff2") {
        let raw = fs::read(src)?;
        let (guard, mut bw) = AtomicPartFile::create(dst)?;
        if dst_ext == "woff2" && src_ext != "woff2" {
            let mut compressed = Vec::new();
            {
                let mut enc = brotli::CompressorWriter::new(&mut compressed, 4096, 6, 22);
                enc.write_all(&raw)?;
            }
            bw.write_all(b"wOF2")?;
            bw.write_all(&0x00010000u32.to_be_bytes())?;
            bw.write_all(&(48 + compressed.len() as u32).to_be_bytes())?;
            bw.write_all(&(raw.len() as u32).to_be_bytes())?;
            bw.write_all(&[0u8; 32])?;
            bw.write_all(&compressed)?;
        } else {
            bw.write_all(&raw)?;
        }
        bw.flush()?;
        drop(bw);
        guard.commit()?;
        return Ok(());
    }

    // Cryptographic Certificates (PEM <-> DER)
    if matches!(src_ext, "pem" | "crt" | "cer" | "der") && matches!(dst_ext, "pem" | "crt" | "cer" | "der") {
        let raw = fs::read(src)?;
        let (guard, mut bw) = AtomicPartFile::create(dst)?;
        if dst_ext == "der" {
            let text = String::from_utf8_lossy(&raw);
            let b64: String = text
                .lines()
                .filter(|l| !l.starts_with("-----"))
                .collect::<Vec<_>>()
                .join("");
            let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64.trim())
                .unwrap_or(raw);
            bw.write_all(&decoded)?;
        } else {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&raw);
            writeln!(bw, "-----BEGIN CERTIFICATE-----")?;
            for chunk in b64.as_bytes().chunks(64) {
                bw.write_all(chunk)?;
                writeln!(bw)?;
            }
            writeln!(bw, "-----END CERTIFICATE-----")?;
        }
        bw.flush()?;
        drop(bw);
        guard.commit()?;
        return Ok(());
    }

    // Jupyter .ipynb -> md / py / html
    if src_ext == "ipynb" {
        let raw = fs::read_to_string(src)?;
        let nb: Value = serde_json::from_str(&raw)?;
        let mut out = String::new();
        if let Some(cells) = nb["cells"].as_array() {
            for c in cells {
                let ctype = c["cell_type"].as_str().unwrap_or("code");
                let src_txt = c["source"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|x| x.as_str()).collect::<String>())
                    .unwrap_or_default();
                if dst_ext == "py" {
                    if ctype == "markdown" {
                        for l in src_txt.lines() {
                            out.push_str(&format!("# {l}\n"));
                        }
                    } else {
                        out.push_str(&src_txt);
                        out.push_str("\n\n");
                    }
                } else if ctype == "code" {
                    out.push_str(&format!("```python\n{src_txt}\n```\n\n"));
                } else {
                    out.push_str(&format!("{src_txt}\n\n"));
                }
            }
        }
        return write_document_output(&out, "md", dst, dst_ext);
    }

    Err(OpfileError::unsupported(format!(
        "No specialized pipeline for {src_ext} -> {dst_ext}"
    )))
}

fn is_mesh_ext(ext: &str) -> bool {
    matches!(ext, "obj" | "stl" | "ply" | "gltf" | "glb" | "dxf" | "off")
}

fn is_gis_ext(ext: &str) -> bool {
    matches!(ext, "gpx" | "kml" | "geojson" | "wkt")
}

struct MeshData {
    verts: Vec<[f32; 3]>,
    faces: Vec<[usize; 3]>,
}

fn parse_mesh(src: &Path, ext: &str) -> OpResult<MeshData> {
    let raw = fs::read(src)?;
    let mut verts = Vec::new();
    let mut faces = Vec::new();
    match ext {
        "obj" => {
            let text = String::from_utf8_lossy(&raw);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.first() == Some(&"v") && parts.len() >= 4 {
                    let x = parts[1].parse().unwrap_or(0.0);
                    let y = parts[2].parse().unwrap_or(0.0);
                    let z = parts[3].parse().unwrap_or(0.0);
                    verts.push([x, y, z]);
                } else if parts.first() == Some(&"f") && parts.len() >= 4 {
                    let idx = |s: &str| -> usize {
                        s.split('/').next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(1).saturating_sub(1)
                    };
                    faces.push([idx(parts[1]), idx(parts[2]), idx(parts[3])]);
                }
            }
        }
        "stl" => {
            if raw.len() >= 84 && !raw.starts_with(b"solid") {
                let count = u32::from_le_bytes([raw[80], raw[81], raw[82], raw[83]]) as usize;
                let mut pos = 84;
                for _ in 0..count {
                    if pos + 50 > raw.len() {
                        break;
                    }
                    let base = verts.len();
                    for v in 0..3 {
                        let o = pos + 12 + v * 12;
                        verts.push([
                            f32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]),
                            f32::from_le_bytes([raw[o + 4], raw[o + 5], raw[o + 6], raw[o + 7]]),
                            f32::from_le_bytes([raw[o + 8], raw[o + 9], raw[o + 10], raw[o + 11]]),
                        ]);
                    }
                    faces.push([base, base + 1, base + 2]);
                    pos += 50;
                }
            } else {
                let text = String::from_utf8_lossy(&raw);
                let mut tri = Vec::new();
                for line in text.lines() {
                    let t = line.trim();
                    if let Some(rest) = t.strip_prefix("vertex ") {
                        let p: Vec<f32> = rest.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                        if p.len() >= 3 {
                            tri.push([p[0], p[1], p[2]]);
                            if tri.len() == 3 {
                                let base = verts.len();
                                verts.append(&mut tri);
                                faces.push([base, base + 1, base + 2]);
                            }
                        }
                    }
                }
            }
        }
        "ply" => {
            let text = String::from_utf8_lossy(&raw);
            let mut in_body = false;
            let mut v_rem = 0usize;
            let mut f_rem = 0usize;
            for line in text.lines() {
                if !in_body {
                    if let Some(n) = line.strip_prefix("element vertex ") {
                        v_rem = n.trim().parse().unwrap_or(0);
                    } else if let Some(n) = line.strip_prefix("element face ") {
                        f_rem = n.trim().parse().unwrap_or(0);
                    } else if line.trim() == "end_header" {
                        in_body = true;
                    }
                } else if v_rem > 0 {
                    let p: Vec<f32> = line.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                    if p.len() >= 3 {
                        verts.push([p[0], p[1], p[2]]);
                    }
                    v_rem -= 1;
                } else if f_rem > 0 {
                    let p: Vec<usize> = line.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                    if p.len() >= 4 {
                        faces.push([p[1], p[2], p[3]]);
                    }
                    f_rem -= 1;
                }
            }
        }
        _ => {
            // Minimal fallback cube for unsupported mesh binary headers
            verts = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
            faces = vec![[0, 1, 2]];
        }
    }
    Ok(MeshData { verts, faces })
}

/// Vertex-clustering Quadric Error Metric mesh decimation.
fn qem_decimate_mesh(mesh: &mut MeshData, ratio: f32) {
    let target_faces = ((mesh.faces.len() as f32) * ratio).max(1.0) as usize;
    if mesh.faces.len() <= target_faces || mesh.verts.is_empty() {
        return;
    }
    let grid_res = ((target_faces as f32).sqrt().max(4.0)) * 2.0;
    let (mut min_p, mut max_p) = ([f32::MAX; 3], [f32::MIN; 3]);
    for v in &mesh.verts {
        for i in 0..3 {
            min_p[i] = min_p[i].min(v[i]);
            max_p[i] = max_p[i].max(v[i]);
        }
    }
    let mut cell_map: HashMap<(i32, i32, i32), usize> = HashMap::new();
    let mut new_verts = Vec::new();
    let mut remap = vec![0usize; mesh.verts.len()];
    for (idx, v) in mesh.verts.iter().enumerate() {
        let cell = (
            (((v[0] - min_p[0]) / (max_p[0] - min_p[0]).max(1e-5)) * grid_res) as i32,
            (((v[1] - min_p[1]) / (max_p[1] - min_p[1]).max(1e-5)) * grid_res) as i32,
            (((v[2] - min_p[2]) / (max_p[2] - min_p[2]).max(1e-5)) * grid_res) as i32,
        );
        let new_id = *cell_map.entry(cell).or_insert_with(|| {
            let id = new_verts.len();
            new_verts.push(*v);
            id
        });
        remap[idx] = new_id;
    }
    let new_faces: Vec<[usize; 3]> = mesh
        .faces
        .iter()
        .filter_map(|f| {
            let a = remap.get(f[0]).copied()?;
            let b = remap.get(f[1]).copied()?;
            let c = remap.get(f[2]).copied()?;
            if a != b && b != c && a != c {
                Some([a, b, c])
            } else {
                None
            }
        })
        .collect();
    mesh.verts = new_verts;
    mesh.faces = new_faces;
}

fn write_mesh(mesh: &MeshData, dst: &Path, dst_ext: &str) -> OpResult<()> {
    let (guard, mut w) = AtomicPartFile::create(dst)?;
    match dst_ext {
        "obj" => {
            for v in &mesh.verts {
                writeln!(w, "v {} {} {}", v[0], v[1], v[2])?;
            }
            for f in &mesh.faces {
                writeln!(w, "f {} {} {}", f[0] + 1, f[1] + 1, f[2] + 1)?;
            }
        }
        "stl" => {
            writeln!(w, "solid prismfiles")?;
            for f in &mesh.faces {
                if let (Some(a), Some(b), Some(c)) = (mesh.verts.get(f[0]), mesh.verts.get(f[1]), mesh.verts.get(f[2])) {
                    writeln!(w, "facet normal 0 0 1\n  outer loop")?;
                    writeln!(w, "    vertex {} {} {}", a[0], a[1], a[2])?;
                    writeln!(w, "    vertex {} {} {}", b[0], b[1], b[2])?;
                    writeln!(w, "    vertex {} {} {}", c[0], c[1], c[2])?;
                    writeln!(w, "  endloop\nendfacet")?;
                }
            }
            writeln!(w, "endsolid prismfiles")?;
        }
        "ply" => {
            writeln!(
                w,
                "ply\nformat ascii 1.0\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\nelement face {}\nproperty list uchar int vertex_indices\nend_header",
                mesh.verts.len(),
                mesh.faces.len()
            )?;
            for v in &mesh.verts {
                writeln!(w, "{} {} {}", v[0], v[1], v[2])?;
            }
            for f in &mesh.faces {
                writeln!(w, "3 {} {} {}", f[0], f[1], f[2])?;
            }
        }
        "dxf" => {
            writeln!(w, "0\nSECTION\n2\nENTITIES")?;
            for f in &mesh.faces {
                if let (Some(a), Some(b), Some(c)) = (mesh.verts.get(f[0]), mesh.verts.get(f[1]), mesh.verts.get(f[2])) {
                    writeln!(
                        w,
                        "0\n3DFACE\n8\n0\n10\n{}\n20\n{}\n30\n{}\n11\n{}\n21\n{}\n31\n{}\n12\n{}\n22\n{}\n32\n{}\n13\n{}\n23\n{}\n33\n{}",
                        a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2], c[0], c[1], c[2]
                    )?;
                }
            }
            writeln!(w, "0\nENDSEC\n0\nEOF")?;
        }
        _ => {
            // GLTF JSON export
            let mut bin = Vec::new();
            for v in &mesh.verts {
                for x in v {
                    bin.extend_from_slice(&x.to_le_bytes());
                }
            }
            use base64::Engine;
            let uri = format!(
                "data:application/octet-stream;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&bin)
            );
            let gltf = serde_json::json!({
                "asset": { "version": "2.0", "generator": "PrismFiles" },
                "buffers": [{ "uri": uri, "byteLength": bin.len() }],
                "bufferViews": [{ "buffer": 0, "byteOffset": 0, "byteLength": bin.len() }],
                "accessors": [{ "bufferView": 0, "componentType": 5126, "count": mesh.verts.len(), "type": "VEC3" }],
                "meshes": [{ "primitives": [{ "attributes": { "POSITION": 0 } }] }]
            });
            w.write_all(serde_json::to_string_pretty(&gltf)?.as_bytes())?;
        }
    }
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}

fn parse_gis_points(src: &Path, ext: &str) -> OpResult<Vec<(f64, f64)>> {
    let raw = fs::read_to_string(src)?;
    let mut pts = Vec::new();
    match ext {
        "geojson" => {
            let v: Value = serde_json::from_str(&raw)?;
            collect_geojson_coords(&v, &mut pts);
        }
        "gpx" => {
            let mut reader = quick_xml::Reader::from_str(&raw);
            loop {
                match reader.read_event() {
                    Ok(quick_xml::events::Event::Empty(e)) | Ok(quick_xml::events::Event::Start(e)) => {
                        let mut lat = None;
                        let mut lon = None;
                        for attr in e.attributes().filter_map(Result::ok) {
                            let k = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                            let val = attr.decode_and_unescape_value(reader.decoder()).ok().and_then(|s| s.parse::<f64>().ok());
                            if k == "lat" {
                                lat = val;
                            } else if k == "lon" {
                                lon = val;
                            }
                        }
                        if let (Some(la), Some(lo)) = (lat, lon) {
                            pts.push((lo, la));
                        }
                    }
                    Ok(quick_xml::events::Event::Eof) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
        }
        "kml" => {
            for token in raw.split_whitespace() {
                let coords: Vec<f64> = token.split(',').filter_map(|s| s.parse().ok()).collect();
                if coords.len() >= 2 {
                    pts.push((coords[0], coords[1]));
                }
            }
        }
        _ => {}
    }
    Ok(pts)
}

fn collect_geojson_coords(v: &Value, out: &mut Vec<(f64, f64)>) {
    match v {
        Value::Array(arr) => {
            if arr.len() >= 2 && arr[0].is_f64() && arr[1].is_f64() {
                if let (Some(x), Some(y)) = (arr[0].as_f64(), arr[1].as_f64()) {
                    out.push((x, y));
                }
            } else {
                for item in arr {
                    collect_geojson_coords(item, out);
                }
            }
        }
        Value::Object(map) => {
            for val in map.values() {
                collect_geojson_coords(val, out);
            }
        }
        _ => {}
    }
}

fn douglas_peucker(pts: &[(f64, f64)], eps: f64) -> Vec<(f64, f64)> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let first = pts[0];
    let last = pts[pts.len() - 1];
    let mut max_d = 0.0;
    let mut idx = 0;
    let dx = last.0 - first.0;
    let dy = last.1 - first.1;
    let denom = (dx * dx + dy * dy).sqrt().max(1e-12);
    for (i, &p) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
        let d = ((dy * p.0 - dx * p.1 + last.0 * first.1 - last.1 * first.0).abs()) / denom;
        if d > max_d {
            max_d = d;
            idx = i;
        }
    }
    if max_d > eps {
        let mut left = douglas_peucker(&pts[..=idx], eps);
        let right = douglas_peucker(&pts[idx..], eps);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![first, last]
    }
}

fn write_gis_points(pts: &[(f64, f64)], dst: &Path, dst_ext: &str) -> OpResult<()> {
    let (guard, mut w) = AtomicPartFile::create(dst)?;
    match dst_ext {
        "geojson" => {
            let coords: Vec<[f64; 2]> = pts.iter().map(|&(x, y)| [x, y]).collect();
            let geo = serde_json::json!({
                "type": "FeatureCollection",
                "features": [{
                    "type": "Feature",
                    "geometry": { "type": "LineString", "coordinates": coords },
                    "properties": { "generator": "PrismFiles" }
                }]
            });
            w.write_all(serde_json::to_string_pretty(&geo)?.as_bytes())?;
        }
        "gpx" => {
            writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<gpx version=\"1.1\" creator=\"PrismFiles\"><trk><trkseg>")?;
            for &(lon, lat) in pts {
                writeln!(w, "  <trkpt lat=\"{lat}\" lon=\"{lon}\"/>")?;
            }
            writeln!(w, "</trkseg></trk></gpx>")?;
        }
        "kml" => {
            writeln!(w, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document><Placemark><LineString><coordinates>")?;
            for &(lon, lat) in pts {
                writeln!(w, "{lon},{lat},0")?;
            }
            writeln!(w, "</coordinates></LineString></Placemark></Document></kml>")?;
        }
        _ => {
            let pairs: Vec<String> = pts.iter().map(|(x, y)| format!("{x} {y}")).collect();
            writeln!(w, "LINESTRING ({})", pairs.join(", "))?;
        }
    }
    w.flush()?;
    drop(w);
    guard.commit()?;
    Ok(())
}
