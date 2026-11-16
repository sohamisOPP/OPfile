use prismfiles_core::archive_ops::{
    create_archive, extract_archive, extract_single_entry, list_archive_virtual, search_archive,
    test_archive_integrity, update_archive_entry, CompressionProfile, CreateArchiveRequest,
};
use prismfiles_core::convert_ops::{convert_file, get_valid_targets_for_extension, total_format_count, ConvertRequest};
use prismfiles_core::fs_ops::{list_directory, SortField};
use prismfiles_core::utilities::{compute_checksums, read_hex_window, shred_paths_dod};
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("prismfiles_test_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn test_directory_natural_dir_first_sort() {
    let dir = temp_dir("sort");
    fs::create_dir_all(dir.join("z_folder")).unwrap();
    fs::create_dir_all(dir.join("a_folder")).unwrap();
    fs::write(dir.join("file10.txt"), b"10").unwrap();
    fs::write(dir.join("file2.txt"), b"2").unwrap();
    fs::write(dir.join(".hidden"), b"h").unwrap();

    let listing = list_directory(dir.to_str().unwrap(), SortField::Name, false, false, None).unwrap();
    let names: Vec<&str> = listing.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["a_folder", "z_folder", "file2.txt", "file10.txt"]);
    assert!(listing.entries[0].permissions.starts_with('d'));
    assert!(listing.entries[2].permissions.starts_with('-'));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_archive_zip_7z_edit_split_and_verify() {
    let dir = temp_dir("arch");
    let src_dir = dir.join("payload");
    fs::create_dir_all(src_dir.join("sub")).unwrap();
    fs::write(src_dir.join("config.ini"), b"mode=fast\n").unwrap();
    fs::write(src_dir.join("sub/notes.txt"), b"hello opfile archive").unwrap();

    let zip_path = dir.join("bundle.zip");
    create_archive(
        &CreateArchiveRequest {
            sources: vec![src_dir.to_string_lossy().into_owned()],
            dest_path: zip_path.to_string_lossy().into_owned(),
            format: "zip".to_string(),
            profile: CompressionProfile::Normal,
            dict_size_mb: None,
            password: None,
            split_size_mb: None,
        },
        None,
    )
    .unwrap();

    let vpage = list_archive_virtual(zip_path.to_str().unwrap(), "payload", None).unwrap();
    assert!(vpage.entries.iter().any(|e| e.name == "config.ini"));
    assert!(vpage.entries.iter().any(|e| e.name == "sub" && e.is_dir));

    let found = search_archive(zip_path.to_str().unwrap(), "notes", None).unwrap();
    assert_eq!(found.len(), 1);

    update_archive_entry(
        zip_path.to_str().unwrap(),
        "payload/config.ini",
        b"mode=ultra\n",
        None,
    )
    .unwrap();

    let single_out = dir.join("single_cfg.ini");
    extract_single_entry(
        zip_path.to_str().unwrap(),
        "payload/config.ini",
        single_out.to_str().unwrap(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&single_out).unwrap(), "mode=ultra\n");

    let rep = test_archive_integrity(zip_path.to_str().unwrap(), None, None).unwrap();
    assert!(rep.passed);

    // 7z roundtrip
    let sz_path = dir.join("bundle.7z");
    create_archive(
        &CreateArchiveRequest {
            sources: vec![src_dir.to_string_lossy().into_owned()],
            dest_path: sz_path.to_string_lossy().into_owned(),
            format: "7z".to_string(),
            profile: CompressionProfile::Normal,
            dict_size_mb: Some(16),
            password: None,
            split_size_mb: None,
        },
        None,
    )
    .unwrap();
    let sz_out = dir.join("sz_extract");
    extract_archive(sz_path.to_str().unwrap(), sz_out.to_str().unwrap(), None, None, None).unwrap();
    assert_eq!(
        fs::read_to_string(sz_out.join("payload/sub/notes.txt")).unwrap(),
        "hello opfile archive"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_format_registry_and_conversions() {
    assert!(total_format_count() >= 255, "Format count was {}", total_format_count());
    let targets = get_valid_targets_for_extension("png");
    assert!(targets.primary_targets.contains(&"webp".to_string()));
    assert!(targets.primary_targets.contains(&"svg".to_string()));

    let dir = temp_dir("conv");
    // Image PNG -> WebP -> SVG -> PDF
    let img_path = dir.join("sample.png");
    let img = image::RgbaImage::from_pixel(16, 16, image::Rgba([108, 92, 231, 255]));
    img.save(&img_path).unwrap();

    let webp_out = convert_file(
        &ConvertRequest {
            source_path: img_path.to_string_lossy().into_owned(),
            target_format: "webp".to_string(),
            dest_path: None,
            quality: Some(90),
            width: Some(8),
            height: Some(8),
            sample_rate: None,
            normalize_lufs: None,
            decimate_ratio: None,
            unicode_form: None,
        },
        None,
    )
    .unwrap();
    assert!(PathBuf::from(&webp_out).exists());

    let svg_out = convert_file(
        &ConvertRequest {
            source_path: img_path.to_string_lossy().into_owned(),
            target_format: "svg".to_string(),
            dest_path: None,
            quality: None,
            width: None,
            height: None,
            sample_rate: None,
            normalize_lufs: None,
            decimate_ratio: None,
            unicode_form: None,
        },
        None,
    )
    .unwrap();
    assert!(fs::read_to_string(&svg_out).unwrap().contains("<svg"));

    // JSON -> YAML -> CSV -> XLSX -> SQLite
    let json_path = dir.join("rows.json");
    fs::write(&json_path, r#"[{"name":"PrismFiles","score":"100"}]"#).unwrap();
    for fmt in ["yaml", "csv", "xlsx", "sqlite", "parquet"] {
        let out = convert_file(
            &ConvertRequest {
                source_path: json_path.to_string_lossy().into_owned(),
                target_format: fmt.to_string(),
                dest_path: None,
                quality: None,
                width: None,
                height: None,
                sample_rate: None,
                normalize_lufs: None,
                decimate_ratio: None,
                unicode_form: None,
            },
            None,
        )
        .unwrap();
        assert!(PathBuf::from(&out).metadata().unwrap().len() > 0);
    }

    // Checksums + mmap hex + shredder
    let ck = compute_checksums(json_path.to_str().unwrap(), None).unwrap();
    assert_eq!(ck.sha256.len(), 64);
    assert_eq!(ck.blake3.len(), 64);
    let hw = read_hex_window(json_path.to_str().unwrap(), 0, 16).unwrap();
    assert_eq!(hw.length, 16);

    let wiped = shred_paths_dod(&[json_path.to_string_lossy().into_owned()], None).unwrap();
    assert_eq!(wiped, 1);
    assert!(!json_path.exists());

    let _ = fs::remove_dir_all(&dir);
}
