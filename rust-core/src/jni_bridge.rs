use crate::archive_ops::{
    create_archive, extract_archive, extract_single_entry, list_archive_virtual, search_archive,
    test_archive_integrity, update_archive_entry, CreateArchiveRequest,
};
use crate::axml::inspect_apk;
use crate::convert_ops::{convert_file, get_valid_targets_for_extension, ConvertRequest};
use crate::error::{ApiEnvelope, ErrorCode, OpResult, OpfileError};
use crate::fs_ops::{
    copy_or_move_batch, create_empty_file, create_folder, delete_paths_batch, list_directory,
    read_text_bounded, rename_entry, search_recursive, write_text_atomic, SortField,
};
use crate::task_manager::{
    cancel_task, drop_task, get_task_snapshot, pause_task, resume_task, spawn_rayon_task,
};
use crate::utilities::{
    analyze_cushion_treemap, compute_checksums, execute_batch_rename, generate_qr_matrix,
    preview_batch_rename, read_hex_window, shred_paths_dod, undo_batch_rename, BatchRenameRequest,
    RenameOp,
};
use crate::wifi_server::{get_wifi_server_status, start_wifi_server, stop_wifi_server};
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jfloat, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use serde::Serialize;

fn jstr_to_string(env: &mut JNIEnv, s: JString) -> OpResult<String> {
    if s.is_null() {
        return Ok(String::new());
    }
    env.get_string(&s)
        .map(|js| js.into())
        .map_err(|e| OpfileError::invalid(format!("JNI string decode error: {e}")))
}

fn to_jstring(env: &mut JNIEnv, s: &str) -> jstring {
    match env.new_string(s) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn jni_guard<T, F>(env: &mut JNIEnv, work: F) -> jstring
where
    T: Serialize,
    F: FnOnce(&mut JNIEnv) -> OpResult<T>,
{
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(env)));
    let json = match outcome {
        Ok(Ok(val)) => ApiEnvelope::ok(val),
        Ok(Err(err)) => ApiEnvelope::<()>::err(err),
        Err(_) => ApiEnvelope::<()>::err(OpfileError::new(
            ErrorCode::PanicRecovered,
            "Recovered from unexpected panic across FFI boundary",
        )),
    };
    to_jstring(env, &json)
}

#[derive(Serialize)]
struct TaskStarted {
    task_id: u64,
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_listDirectory(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
    sort_field: JString,
    descending: jboolean,
    show_hidden: jboolean,
    query: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        let sf_str = jstr_to_string(env, sort_field)?;
        let q = jstr_to_string(env, query)?;
        let sf = match sf_str.as_str() {
            "size" => SortField::Size,
            "date" => SortField::Date,
            "type" => SortField::Type,
            _ => SortField::Name,
        };
        list_directory(
            &p,
            sf,
            descending != JNI_FALSE,
            show_hidden != JNI_FALSE,
            if q.is_empty() { None } else { Some(q.as_str()) },
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_searchRecursive(
    mut env: JNIEnv,
    _class: JClass,
    root: JString,
    query: JString,
    max_results: jint,
) -> jstring {
    jni_guard(&mut env, |env| {
        let r = jstr_to_string(env, root)?;
        let q = jstr_to_string(env, query)?;
        search_recursive(&r, &q, (max_results as usize).clamp(1, 5000))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_createFolder(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        create_folder(&p)?;
        Ok(true)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_createEmptyFile(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        create_empty_file(&p)?;
        Ok(true)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_renameEntry(
    mut env: JNIEnv,
    _class: JClass,
    old_path: JString,
    new_name: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let o = jstr_to_string(env, old_path)?;
        let n = jstr_to_string(env, new_name)?;
        rename_entry(&o, &n)
    })
}

#[derive(Serialize)]
pub struct TextContentResponse {
    pub content: String,
    pub bytes_read: usize,
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_readTextBounded(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
    max_bytes: jint,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        let content = read_text_bounded(&p, (max_bytes as usize).clamp(1024, 4 * 1024 * 1024))?;
        let bytes_read = content.len();
        Ok(TextContentResponse { content, bytes_read })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_writeTextAtomic(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
    content: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        let c = jstr_to_string(env, content)?;
        write_text_atomic(&p, &c)?;
        Ok(true)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_listArchiveVirtual(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    inner_dir: JString,
    password: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let ap = jstr_to_string(env, archive_path)?;
        let id = jstr_to_string(env, inner_dir)?;
        let pw = jstr_to_string(env, password)?;
        list_archive_virtual(&ap, &id, if pw.is_empty() { None } else { Some(pw.as_str()) })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_searchArchive(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    query: JString,
    password: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let ap = jstr_to_string(env, archive_path)?;
        let q = jstr_to_string(env, query)?;
        let pw = jstr_to_string(env, password)?;
        search_archive(&ap, &q, if pw.is_empty() { None } else { Some(pw.as_str()) })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_inspectApkManifest(
    mut env: JNIEnv,
    _class: JClass,
    apk_path: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, apk_path)?;
        inspect_apk(&p)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_updateArchiveEntry(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    entry_path: JString,
    content: JString,
    password: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let ap = jstr_to_string(env, archive_path)?;
        let ep = jstr_to_string(env, entry_path)?;
        let c = jstr_to_string(env, content)?;
        let pw = jstr_to_string(env, password)?;
        update_archive_entry(
            &ap,
            &ep,
            c.as_bytes(),
            if pw.is_empty() { None } else { Some(pw.as_str()) },
        )?;
        Ok(true)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_getConversionTargets(
    mut env: JNIEnv,
    _class: JClass,
    ext: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let e = jstr_to_string(env, ext)?;
        Ok(get_valid_targets_for_extension(&e))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_readHexWindow(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
    offset: jlong,
    length: jint,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        read_hex_window(&p, offset.max(0) as u64, length.max(0) as usize)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_previewBatchRename(
    mut env: JNIEnv,
    _class: JClass,
    req_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let js = jstr_to_string(env, req_json)?;
        let req: BatchRenameRequest = serde_json::from_str(&js)?;
        preview_batch_rename(&req)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_executeBatchRename(
    mut env: JNIEnv,
    _class: JClass,
    ops_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let js = jstr_to_string(env, ops_json)?;
        let ops: Vec<RenameOp> = serde_json::from_str(&js)?;
        execute_batch_rename(&ops)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_undoBatchRename(
    mut env: JNIEnv,
    _class: JClass,
    ops_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let js = jstr_to_string(env, ops_json)?;
        let ops: Vec<RenameOp> = serde_json::from_str(&js)?;
        undo_batch_rename(&ops)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_generateQrMatrix(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let t = jstr_to_string(env, text)?;
        generate_qr_matrix(&t)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startWifiServer(
    mut env: JNIEnv,
    _class: JClass,
    root_dir: JString,
    http_port: jint,
    ftp_port: jint,
) -> jstring {
    jni_guard(&mut env, |env| {
        let r = jstr_to_string(env, root_dir)?;
        start_wifi_server(&r, http_port.max(1024) as u16, ftp_port.max(1024) as u16)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_stopWifiServer(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    jni_guard(&mut env, |_| Ok(stop_wifi_server()))
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_getWifiServerStatus(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    jni_guard(&mut env, |_| Ok(get_wifi_server_status()))
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startCopyMoveTask(
    mut env: JNIEnv,
    _class: JClass,
    sources_json: JString,
    dest_dir: JString,
    move_mode: jboolean,
) -> jstring {
    jni_guard(&mut env, |env| {
        let sj = jstr_to_string(env, sources_json)?;
        let dd = jstr_to_string(env, dest_dir)?;
        let sources: Vec<String> = serde_json::from_str(&sj)?;
        let mv = move_mode != JNI_FALSE;
        let title = if mv { "Moving files" } else { "Copying files" };
        let task_id = spawn_rayon_task(title, move |handle| {
            let outs = copy_or_move_batch(sources, &dd, mv, &handle)?;
            Ok(serde_json::to_string(&outs)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startDeleteTask(
    mut env: JNIEnv,
    _class: JClass,
    paths_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let pj = jstr_to_string(env, paths_json)?;
        let paths: Vec<String> = serde_json::from_str(&pj)?;
        let task_id = spawn_rayon_task("Deleting items", move |handle| {
            let count = delete_paths_batch(paths, &handle)?;
            Ok(serde_json::to_string(&count)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startShredTask(
    mut env: JNIEnv,
    _class: JClass,
    paths_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let pj = jstr_to_string(env, paths_json)?;
        let paths: Vec<String> = serde_json::from_str(&pj)?;
        let task_id = spawn_rayon_task("Shredding files (DoD 3-pass)", move |handle| {
            let count = shred_paths_dod(&paths, Some(&handle))?;
            Ok(serde_json::to_string(&count)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startExtractTask(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    dest_dir: JString,
    password: JString,
    entries_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let ap = jstr_to_string(env, archive_path)?;
        let dd = jstr_to_string(env, dest_dir)?;
        let pw = jstr_to_string(env, password)?;
        let ej = jstr_to_string(env, entries_json)?;
        let selected: Option<Vec<String>> = if ej.trim().is_empty() {
            None
        } else {
            Some(serde_json::from_str(&ej)?)
        };
        let task_id = spawn_rayon_task("Extracting archive", move |handle| {
            let outs = extract_archive(
                &ap,
                &dd,
                if pw.is_empty() { None } else { Some(pw.as_str()) },
                selected,
                Some(&handle),
            )?;
            Ok(serde_json::to_string(&outs)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startExtractSingleTask(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    entry_path: JString,
    dest_file: JString,
    password: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let ap = jstr_to_string(env, archive_path)?;
        let ep = jstr_to_string(env, entry_path)?;
        let df = jstr_to_string(env, dest_file)?;
        let pw = jstr_to_string(env, password)?;
        let task_id = spawn_rayon_task("Extracting entry", move |handle| {
            extract_single_entry(
                &ap,
                &ep,
                &df,
                if pw.is_empty() { None } else { Some(pw.as_str()) },
                Some(&handle),
            )?;
            Ok(serde_json::to_string(&df)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startCreateArchiveTask(
    mut env: JNIEnv,
    _class: JClass,
    req_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let rj = jstr_to_string(env, req_json)?;
        let req: CreateArchiveRequest = serde_json::from_str(&rj)?;
        let task_id = spawn_rayon_task("Creating archive", move |handle| {
            let outs = create_archive(&req, Some(&handle))?;
            Ok(serde_json::to_string(&outs)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startTestArchiveTask(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    password: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let ap = jstr_to_string(env, archive_path)?;
        let pw = jstr_to_string(env, password)?;
        let task_id = spawn_rayon_task("Testing archive integrity", move |handle| {
            let rep = test_archive_integrity(
                &ap,
                if pw.is_empty() { None } else { Some(pw.as_str()) },
                Some(&handle),
            )?;
            Ok(serde_json::to_string(&rep)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startConvertTask(
    mut env: JNIEnv,
    _class: JClass,
    req_json: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let rj = jstr_to_string(env, req_json)?;
        let req: ConvertRequest = serde_json::from_str(&rj)?;
        let task_id = spawn_rayon_task(
            format!("Converting to {}", req.target_format.to_uppercase()),
            move |handle| {
                let out = convert_file(&req, Some(&handle))?;
                Ok(serde_json::to_string(&out)?)
            },
        )?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startChecksumTask(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    jni_guard(&mut env, |env| {
        let p = jstr_to_string(env, path)?;
        let task_id = spawn_rayon_task("Computing checksums", move |handle| {
            let rep = compute_checksums(&p, Some(&handle))?;
            Ok(serde_json::to_string(&rep)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_startTreemapTask(
    mut env: JNIEnv,
    _class: JClass,
    root_dir: JString,
    width: jfloat,
    height: jfloat,
) -> jstring {
    jni_guard(&mut env, |env| {
        let r = jstr_to_string(env, root_dir)?;
        let task_id = spawn_rayon_task("Analyzing storage treemap", move |handle| {
            let nodes = analyze_cushion_treemap(&r, width, height, Some(&handle))?;
            Ok(serde_json::to_string(&nodes)?)
        })?;
        Ok(TaskStarted { task_id })
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_getTaskSnapshot(
    mut env: JNIEnv,
    _class: JClass,
    task_id: jlong,
) -> jstring {
    jni_guard(&mut env, |_| {
        get_task_snapshot(task_id.max(0) as u64)
            .ok_or_else(|| OpfileError::new(ErrorCode::NotFound, "Task not found"))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_pauseTask(
    _env: JNIEnv,
    _class: JClass,
    task_id: jlong,
) -> jboolean {
    if pause_task(task_id.max(0) as u64) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_resumeTask(
    _env: JNIEnv,
    _class: JClass,
    task_id: jlong,
) -> jboolean {
    if resume_task(task_id.max(0) as u64) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_cancelTask(
    _env: JNIEnv,
    _class: JClass,
    task_id: jlong,
) -> jboolean {
    if cancel_task(task_id.max(0) as u64) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_com_prismfiles_app_RustCore_dropTask(
    _env: JNIEnv,
    _class: JClass,
    task_id: jlong,
) {
    drop_task(task_id.max(0) as u64);
}
