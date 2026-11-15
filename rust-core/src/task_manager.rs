use crate::error::{ErrorCode, OpResult, OpfileError};
use dashmap::DashMap;
use rayon::ThreadPool;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use tokio::runtime::Runtime;

pub const STREAM_BUF_SIZE: usize = 128 * 1024;

static TOKIO_RT: OnceLock<Runtime> = OnceLock::new();
static RAYON_POOL: OnceLock<ThreadPool> = OnceLock::new();
static TASKS: OnceLock<DashMap<u64, Arc<TaskHandle>>> = OnceLock::new();
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

pub fn tokio_rt() -> OpResult<&'static Runtime> {
    if let Some(rt) = TOKIO_RT.get() {
        return Ok(rt);
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("opfile-tokio")
        .build()
        .map_err(|e| OpfileError::io(format!("Failed to initialize Tokio runtime: {e}")))?;
    let _ = TOKIO_RT.set(rt);
    TOKIO_RT
        .get()
        .ok_or_else(|| OpfileError::io("Tokio runtime initialization race"))
}

pub fn rayon_pool() -> OpResult<&'static ThreadPool> {
    if let Some(pool) = RAYON_POOL.get() {
        return Ok(pool);
    }
    let threads = std::thread::available_parallelism()
        .map(|n| n.get().clamp(2, 8))
        .unwrap_or(4);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .thread_name(|i| format!("opfile-rayon-{i}"))
        .build()
        .map_err(|e| OpfileError::io(format!("Failed to initialize Rayon pool: {e}")))?;
    let _ = RAYON_POOL.set(pool);
    RAYON_POOL
        .get()
        .ok_or_else(|| OpfileError::io("Rayon pool initialization race"))
}

fn task_map() -> &'static DashMap<u64, Arc<TaskHandle>> {
    TASKS.get_or_init(DashMap::new)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TaskState {
    Running = 0,
    Paused = 1,
    Completed = 2,
    Failed = 3,
    Cancelled = 4,
}

impl TaskState {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Running,
            1 => Self::Paused,
            2 => Self::Completed,
            3 => Self::Failed,
            _ => Self::Cancelled,
        }
    }
}

pub struct TaskHandle {
    pub id: u64,
    pub title: String,
    pub bytes_done: AtomicU64,
    pub bytes_total: AtomicU64,
    pub items_done: AtomicU64,
    pub items_total: AtomicU64,
    pub state: AtomicU8,
    pub cancelled: AtomicBool,
    pause_lock: Mutex<bool>,
    pause_cv: Condvar,
    pub current_item: Mutex<String>,
    pub result_json: Mutex<Option<String>>,
    pub error_message: Mutex<Option<String>>,
}

impl TaskHandle {
    pub fn new(id: u64, title: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            id,
            title: title.into(),
            bytes_done: AtomicU64::new(0),
            bytes_total: AtomicU64::new(0),
            items_done: AtomicU64::new(0),
            items_total: AtomicU64::new(0),
            state: AtomicU8::new(TaskState::Running as u8),
            cancelled: AtomicBool::new(false),
            pause_lock: Mutex::new(false),
            pause_cv: Condvar::new(),
            current_item: Mutex::new(String::new()),
            result_json: Mutex::new(None),
            error_message: Mutex::new(None),
        })
    }

    pub fn check_checkpoint(&self) -> OpResult<()> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(OpfileError::cancelled());
        }
        if let Ok(mut paused) = self.pause_lock.lock() {
            while *paused {
                if self.cancelled.load(Ordering::SeqCst) {
                    return Err(OpfileError::cancelled());
                }
                paused = match self.pause_cv.wait(paused) {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
            }
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(OpfileError::cancelled());
        }
        Ok(())
    }

    pub fn set_current_item(&self, item: impl Into<String>) {
        if let Ok(mut guard) = self.current_item.lock() {
            *guard = item.into();
        }
    }

    pub fn set_totals(&self, bytes_total: u64, items_total: u64) {
        self.bytes_total.store(bytes_total, Ordering::Relaxed);
        self.items_total.store(items_total, Ordering::Relaxed);
    }

    pub fn add_bytes(&self, delta: u64) {
        self.bytes_done.fetch_add(delta, Ordering::Relaxed);
    }

    pub fn inc_items(&self) {
        self.items_done.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        if let Ok(mut paused) = self.pause_lock.lock() {
            *paused = true;
            self.state.store(TaskState::Paused as u8, Ordering::SeqCst);
        }
    }

    pub fn resume(&self) {
        if let Ok(mut paused) = self.pause_lock.lock() {
            *paused = false;
            if !self.cancelled.load(Ordering::SeqCst) {
                self.state.store(TaskState::Running as u8, Ordering::SeqCst);
            }
            self.pause_cv.notify_all();
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.state.store(TaskState::Cancelled as u8, Ordering::SeqCst);
        if let Ok(mut paused) = self.pause_lock.lock() {
            *paused = false;
            self.pause_cv.notify_all();
        }
    }

    pub fn finish_ok(&self, payload: String) {
        if let Ok(mut res) = self.result_json.lock() {
            *res = Some(payload);
        }
        self.state.store(TaskState::Completed as u8, Ordering::SeqCst);
    }

    pub fn finish_err(&self, err: &OpfileError) {
        if matches!(err.code, ErrorCode::Cancelled) {
            self.state.store(TaskState::Cancelled as u8, Ordering::SeqCst);
        } else {
            self.state.store(TaskState::Failed as u8, Ordering::SeqCst);
        }
        if let Ok(mut guard) = self.error_message.lock() {
            *guard = Some(err.message.clone());
        }
    }
}

#[derive(Serialize)]
pub struct TaskSnapshot {
    pub id: u64,
    pub title: String,
    pub state: TaskState,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub items_done: u64,
    pub items_total: u64,
    pub current_item: String,
    pub result_json: Option<String>,
    pub error_message: Option<String>,
}

pub fn spawn_rayon_task<F>(title: impl Into<String>, work: F) -> OpResult<u64>
where
    F: FnOnce(Arc<TaskHandle>) -> OpResult<String> + Send + 'static,
{
    let id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
    let handle = TaskHandle::new(id, title);
    task_map().insert(id, Arc::clone(&handle));
    let pool = rayon_pool()?;
    pool.spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(Arc::clone(&handle))));
        match outcome {
            Ok(Ok(payload)) => handle.finish_ok(payload),
            Ok(Err(e)) => handle.finish_err(&e),
            Err(_) => handle.finish_err(&OpfileError::new(
                ErrorCode::PanicRecovered,
                "Background worker recovered from unexpected panic",
            )),
        }
    });
    Ok(id)
}

pub fn get_task_snapshot(id: u64) -> Option<TaskSnapshot> {
    task_map().get(&id).map(|entry| {
        let h = entry.value();
        TaskSnapshot {
            id: h.id,
            title: h.title.clone(),
            state: TaskState::from_u8(h.state.load(Ordering::SeqCst)),
            bytes_done: h.bytes_done.load(Ordering::Relaxed),
            bytes_total: h.bytes_total.load(Ordering::Relaxed),
            items_done: h.items_done.load(Ordering::Relaxed),
            items_total: h.items_total.load(Ordering::Relaxed),
            current_item: h.current_item.lock().map(|g| g.clone()).unwrap_or_default(),
            result_json: h.result_json.lock().ok().and_then(|g| g.clone()),
            error_message: h.error_message.lock().ok().and_then(|g| g.clone()),
        }
    })
}

pub fn pause_task(id: u64) -> bool {
    if let Some(h) = task_map().get(&id) {
        h.pause();
        true
    } else {
        false
    }
}

pub fn resume_task(id: u64) -> bool {
    if let Some(h) = task_map().get(&id) {
        h.resume();
        true
    } else {
        false
    }
}

pub fn cancel_task(id: u64) -> bool {
    if let Some(h) = task_map().get(&id) {
        h.cancel();
        true
    } else {
        false
    }
}

pub fn drop_task(id: u64) {
    task_map().remove(&id);
}

/// Atomic part-file wrapper that writes to `<final>.opfile.<pid>.part` and
/// automatically deletes the partial file on drop unless `commit()` succeeds.
pub struct AtomicPartFile {
    pub part_path: PathBuf,
    pub final_path: PathBuf,
    committed: bool,
}

impl AtomicPartFile {
    pub fn create(final_path: &Path) -> OpResult<(Self, BufWriter<File>)> {
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let file_name = final_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("output");
        let part_path = final_path.with_file_name(format!(".{file_name}.{stamp}.opfile.part"));
        let file = File::create(&part_path)?;
        let writer = BufWriter::with_capacity(STREAM_BUF_SIZE, file);
        Ok((
            Self {
                part_path,
                final_path: final_path.to_path_buf(),
                committed: false,
            },
            writer,
        ))
    }

    pub fn part_path_only(final_path: &Path) -> OpResult<Self> {
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let file_name = final_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("output");
        let part_path = final_path.with_file_name(format!(".{file_name}.{stamp}.opfile.part"));
        Ok(Self {
            part_path,
            final_path: final_path.to_path_buf(),
            committed: false,
        })
    }

    pub fn commit(mut self) -> OpResult<PathBuf> {
        if self.final_path.exists() {
            let _ = fs::remove_file(&self.final_path);
        }
        fs::rename(&self.part_path, &self.final_path)?;
        self.committed = true;
        Ok(self.final_path.clone())
    }
}

impl Drop for AtomicPartFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.part_path);
        }
    }
}

pub fn stream_copy_with_task(
    src: &Path,
    dst: &Path,
    task: Option<&TaskHandle>,
) -> OpResult<u64> {
    let file_in = File::open(src)?;
    let mut reader = BufReader::with_capacity(STREAM_BUF_SIZE, file_in);
    let (guard, mut writer) = AtomicPartFile::create(dst)?;
    let mut buf = vec![0u8; STREAM_BUF_SIZE];
    let mut total = 0u64;
    loop {
        if let Some(t) = task {
            t.check_checkpoint()?;
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buf[..n])?;
        total += n as u64;
        if let Some(t) = task {
            t.add_bytes(n as u64);
        }
    }
    writer.flush()?;
    drop(writer);
    guard.commit()?;
    Ok(total)
}
