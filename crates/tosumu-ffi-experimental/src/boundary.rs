use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::thread::{self, ThreadId};

use tosumu_core::error::{ErrorReport, ErrorStatus, ErrorValue, TosumuError};
use tosumu_core::{KvConnectionInfo, KvReadTransaction, KvScanPage, SharedKvStore};

pub const TAG_SUCCESS: u32 = 0;
pub const TAG_ABSENT: u32 = 1;
pub const TAG_ERROR: u32 = 2;
pub const TAG_BOUNDARY_FAILURE: u32 = 3;

pub const BOUNDARY_INVALID_POINTER: u32 = 1;
pub const BOUNDARY_INVALID_UTF8: u32 = 2;
pub const BOUNDARY_INVALID_HANDLE: u32 = 3;
pub const BOUNDARY_WRONG_KIND: u32 = 4;
pub const BOUNDARY_WRONG_THREAD: u32 = 5;
pub const BOUNDARY_PANIC: u32 = 6;
pub const BOUNDARY_POISONED: u32 = 7;
pub const BOUNDARY_REGISTRY_FULL: u32 = 8;
pub const BOUNDARY_INVALID_PATH: u32 = 9;
pub const BOUNDARY_INVALID_INDEX: u32 = 10;
pub const BOUNDARY_WRONG_DETAIL_TYPE: u32 = 11;
pub const BOUNDARY_LIMIT_OUT_OF_RANGE: u32 = 12;

const MAX_LIVE_HANDLES: usize = 4096;

pub struct DatabaseObject {
    store: SharedKvStore,
    origin: ThreadId,
    poisoned: AtomicBool,
}

pub struct SnapshotObject {
    snapshot: Mutex<KvReadTransaction>,
    origin: ThreadId,
}

#[derive(Clone)]
enum Entry {
    Database(Arc<DatabaseObject>),
    Snapshot(Arc<SnapshotObject>),
    Bytes(Arc<Vec<u8>>),
    Error(Arc<ErrorReport>),
    Connection(Arc<KvConnectionInfo>),
    ScanPage(Arc<KvScanPage>),
}

#[derive(Clone, Copy)]
pub enum Kind {
    Database,
    Snapshot,
    Bytes,
    Error,
    Connection,
    ScanPage,
}

#[derive(Default)]
struct Registry {
    next: u64,
    entries: HashMap<u64, Entry>,
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    REGISTRY.get_or_init(|| {
        Mutex::new(Registry {
            next: 1,
            entries: HashMap::new(),
        })
    })
}

fn lock_registry() -> MutexGuard<'static, Registry> {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn insert(entry: Entry) -> Result<u64, u32> {
    let mut registry = lock_registry();
    if registry.entries.len() >= MAX_LIVE_HANDLES {
        return Err(BOUNDARY_REGISTRY_FULL);
    }
    let handle = registry.next;
    registry.next = registry.next.checked_add(1).ok_or(BOUNDARY_REGISTRY_FULL)?;
    registry.entries.insert(handle, entry);
    Ok(handle)
}

fn lookup(handle: u64) -> Result<Entry, u32> {
    if handle == 0 {
        return Err(BOUNDARY_INVALID_HANDLE);
    }
    lock_registry()
        .entries
        .get(&handle)
        .cloned()
        .ok_or(BOUNDARY_INVALID_HANDLE)
}

pub fn close(handle: u64, expected: Kind) -> Result<(), u32> {
    if handle == 0 {
        return Err(BOUNDARY_INVALID_HANDLE);
    }
    let mut registry = lock_registry();
    let entry = registry
        .entries
        .get(&handle)
        .ok_or(BOUNDARY_INVALID_HANDLE)?;
    if !entry.is_kind(expected) {
        return Err(BOUNDARY_WRONG_KIND);
    }
    registry.entries.remove(&handle);
    Ok(())
}

impl Entry {
    fn is_kind(&self, kind: Kind) -> bool {
        matches!(
            (self, kind),
            (Self::Database(_), Kind::Database)
                | (Self::Snapshot(_), Kind::Snapshot)
                | (Self::Bytes(_), Kind::Bytes)
                | (Self::Error(_), Kind::Error)
                | (Self::Connection(_), Kind::Connection)
                | (Self::ScanPage(_), Kind::ScanPage)
        )
    }
}

pub fn create(path: &[u8]) -> Result<u64, CallFailure> {
    let path = parse_path(path)?;
    let store = SharedKvStore::create(path).map_err(CallFailure::Core)?;
    insert(Entry::Database(Arc::new(DatabaseObject {
        store,
        origin: thread::current().id(),
        poisoned: AtomicBool::new(false),
    })))
    .map_err(CallFailure::Boundary)
}

pub fn open(path: &[u8]) -> Result<u64, CallFailure> {
    let path = parse_path(path)?;
    let store = SharedKvStore::open(path).map_err(CallFailure::Core)?;
    insert(Entry::Database(Arc::new(DatabaseObject {
        store,
        origin: thread::current().id(),
        poisoned: AtomicBool::new(false),
    })))
    .map_err(CallFailure::Boundary)
}

fn parse_path(bytes: &[u8]) -> Result<&Path, CallFailure> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| CallFailure::Boundary(BOUNDARY_INVALID_UTF8))?;
    if text.is_empty() || text.as_bytes().contains(&0) {
        return Err(CallFailure::Boundary(BOUNDARY_INVALID_PATH));
    }
    Ok(Path::new(text))
}

fn database(handle: u64) -> Result<Arc<DatabaseObject>, CallFailure> {
    let Entry::Database(database) = lookup(handle).map_err(CallFailure::Boundary)? else {
        return Err(CallFailure::Boundary(BOUNDARY_WRONG_KIND));
    };
    if database.origin != thread::current().id() {
        return Err(CallFailure::Boundary(BOUNDARY_WRONG_THREAD));
    }
    if database.poisoned.load(Ordering::Acquire) {
        return Err(CallFailure::Boundary(BOUNDARY_POISONED));
    }
    Ok(database)
}

fn snapshot(handle: u64) -> Result<Arc<SnapshotObject>, CallFailure> {
    let Entry::Snapshot(snapshot) = lookup(handle).map_err(CallFailure::Boundary)? else {
        return Err(CallFailure::Boundary(BOUNDARY_WRONG_KIND));
    };
    if snapshot.origin != thread::current().id() {
        return Err(CallFailure::Boundary(BOUNDARY_WRONG_THREAD));
    }
    Ok(snapshot)
}

pub fn poison_database(handle: u64) {
    if let Ok(Entry::Database(database)) = lookup(handle) {
        database.poisoned.store(true, Ordering::Release);
    }
}

pub fn put(handle: u64, key: &[u8], value: &[u8]) -> Result<(), CallFailure> {
    database(handle)?
        .store
        .put(key, value)
        .map_err(CallFailure::Core)
}

pub fn delete(handle: u64, key: &[u8]) -> Result<(), CallFailure> {
    database(handle)?
        .store
        .delete(key)
        .map_err(CallFailure::Core)
}

pub fn get(handle: u64, key: &[u8]) -> Result<Option<u64>, CallFailure> {
    let value = database(handle)?
        .store
        .get(key)
        .map_err(CallFailure::Core)?;
    value.map(insert_bytes).transpose()
}

pub fn snapshot_begin(handle: u64) -> Result<u64, CallFailure> {
    let snapshot = database(handle)?
        .store
        .snapshot()
        .map_err(CallFailure::Core)?;
    insert(Entry::Snapshot(Arc::new(SnapshotObject {
        snapshot: Mutex::new(snapshot),
        origin: thread::current().id(),
    })))
    .map_err(CallFailure::Boundary)
}

pub fn connection_info(handle: u64) -> Result<u64, CallFailure> {
    let info = database(handle)?
        .store
        .connection_info()
        .map_err(CallFailure::Core)?;
    insert(Entry::Connection(Arc::new(info))).map_err(CallFailure::Boundary)
}

pub fn connection_field(handle: u64, field: u32) -> Result<Option<u64>, u32> {
    let Entry::Connection(info) = lookup(handle)? else {
        return Err(BOUNDARY_WRONG_KIND);
    };
    match field {
        1 => Ok(Some(info.active_readers)),
        2 => Ok(Some(info.maximum_readers)),
        3 => Ok(info.oldest_reader_generation),
        4 => Ok(Some(info.checkpoint_generation)),
        5 => Ok(Some(info.latest_generation)),
        6 => Ok(Some(info.retained_wal_bytes)),
        7 => Ok(Some(info.retained_frame_versions)),
        8 => Ok(Some(u64::from(info.checkpoint_blocked))),
        _ => Err(BOUNDARY_INVALID_INDEX),
    }
}

pub fn snapshot_generation(handle: u64) -> Result<u64, CallFailure> {
    let snapshot = snapshot(handle)?;
    let guard = snapshot
        .snapshot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Ok(guard.generation())
}

pub fn snapshot_get(handle: u64, key: &[u8]) -> Result<Option<u64>, CallFailure> {
    let snapshot = snapshot(handle)?;
    let value = snapshot
        .snapshot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(key)
        .map_err(CallFailure::Core)?;
    value.map(insert_bytes).transpose()
}

pub fn snapshot_scan_page(
    handle: u64,
    start: &[u8],
    end: &[u8],
    maximum_pairs: u64,
    maximum_payload_bytes: u64,
) -> Result<u64, CallFailure> {
    let maximum_pairs = usize::try_from(maximum_pairs)
        .map_err(|_| CallFailure::Boundary(BOUNDARY_LIMIT_OUT_OF_RANGE))?;
    let snapshot = snapshot(handle)?;
    let page = snapshot
        .snapshot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .scan_page(start, end, maximum_pairs, maximum_payload_bytes)
        .map_err(CallFailure::Core)?;
    insert(Entry::ScanPage(Arc::new(page))).map_err(CallFailure::Boundary)
}

fn scan_page(handle: u64) -> Result<Arc<KvScanPage>, u32> {
    match lookup(handle)? {
        Entry::ScanPage(page) => Ok(page),
        _ => Err(BOUNDARY_WRONG_KIND),
    }
}

pub fn scan_page_pair_count(handle: u64) -> Result<u64, u32> {
    u64::try_from(scan_page(handle)?.pairs.len()).map_err(|_| BOUNDARY_INVALID_INDEX)
}

pub fn scan_page_pair_key(handle: u64, index: u64) -> Result<u64, CallFailure> {
    let page = scan_page(handle).map_err(CallFailure::Boundary)?;
    let index =
        usize::try_from(index).map_err(|_| CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    let pair = page
        .pairs
        .get(index)
        .ok_or(CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    insert_bytes(pair.0.clone())
}

pub fn scan_page_pair_value(handle: u64, index: u64) -> Result<u64, CallFailure> {
    let page = scan_page(handle).map_err(CallFailure::Boundary)?;
    let index =
        usize::try_from(index).map_err(|_| CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    let pair = page
        .pairs
        .get(index)
        .ok_or(CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    insert_bytes(pair.1.clone())
}

pub fn scan_page_next_start(handle: u64) -> Result<Option<u64>, CallFailure> {
    scan_page(handle)
        .map_err(CallFailure::Boundary)?
        .next_start_inclusive
        .clone()
        .map(insert_bytes)
        .transpose()
}

pub fn scan_page_blocked_entry_payload_bytes(handle: u64) -> Result<Option<u64>, u32> {
    Ok(scan_page(handle)?.blocked_entry_payload_bytes)
}

pub fn insert_bytes(bytes: Vec<u8>) -> Result<u64, CallFailure> {
    insert(Entry::Bytes(Arc::new(bytes))).map_err(CallFailure::Boundary)
}

pub fn bytes(handle: u64) -> Result<Arc<Vec<u8>>, u32> {
    match lookup(handle)? {
        Entry::Bytes(bytes) => Ok(bytes),
        _ => Err(BOUNDARY_WRONG_KIND),
    }
}

pub fn insert_error(error: TosumuError) -> Result<u64, u32> {
    insert(Entry::Error(Arc::new(error.error_report())))
}

fn error(handle: u64) -> Result<Arc<ErrorReport>, u32> {
    match lookup(handle)? {
        Entry::Error(error) => Ok(error),
        _ => Err(BOUNDARY_WRONG_KIND),
    }
}

pub fn error_code(handle: u64) -> Result<u64, CallFailure> {
    insert_bytes(
        error(handle)
            .map_err(CallFailure::Boundary)?
            .code
            .as_bytes()
            .to_vec(),
    )
}

pub fn error_message(handle: u64) -> Result<u64, CallFailure> {
    insert_bytes(
        error(handle)
            .map_err(CallFailure::Boundary)?
            .message
            .as_bytes()
            .to_vec(),
    )
}

pub fn error_status(handle: u64) -> Result<u64, u32> {
    Ok(match error(handle)?.status {
        ErrorStatus::InvalidInput => 1,
        ErrorStatus::NotFound => 2,
        ErrorStatus::Conflict => 3,
        ErrorStatus::PermissionDenied => 4,
        ErrorStatus::Busy => 5,
        ErrorStatus::IntegrityFailure => 6,
        ErrorStatus::ExternalFailure => 7,
        ErrorStatus::Unsupported => 8,
        ErrorStatus::Internal => 9,
    })
}

pub fn error_detail_count(handle: u64) -> Result<u64, u32> {
    u64::try_from(error(handle)?.details.len()).map_err(|_| BOUNDARY_INVALID_INDEX)
}

pub fn error_detail_key(handle: u64, index: u64) -> Result<u64, CallFailure> {
    let report = error(handle).map_err(CallFailure::Boundary)?;
    let index =
        usize::try_from(index).map_err(|_| CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    let detail = report
        .details
        .get(index)
        .ok_or(CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    insert_bytes(detail.key.as_bytes().to_vec())
}

pub fn error_detail_type(handle: u64, index: u64) -> Result<u64, u32> {
    let report = error(handle)?;
    let index = usize::try_from(index).map_err(|_| BOUNDARY_INVALID_INDEX)?;
    Ok(
        match &report
            .details
            .get(index)
            .ok_or(BOUNDARY_INVALID_INDEX)?
            .value
        {
            ErrorValue::Bool(_) => 1,
            ErrorValue::Str(_) => 2,
            ErrorValue::U16(_) => 3,
            ErrorValue::U64(_) => 4,
        },
    )
}

pub fn error_detail_scalar(handle: u64, index: u64) -> Result<u64, u32> {
    let report = error(handle)?;
    let index = usize::try_from(index).map_err(|_| BOUNDARY_INVALID_INDEX)?;
    match &report
        .details
        .get(index)
        .ok_or(BOUNDARY_INVALID_INDEX)?
        .value
    {
        ErrorValue::Bool(value) => Ok(u64::from(*value)),
        ErrorValue::U16(value) => Ok(u64::from(*value)),
        ErrorValue::U64(value) => Ok(*value),
        ErrorValue::Str(_) => Err(BOUNDARY_WRONG_DETAIL_TYPE),
    }
}

pub fn error_detail_string(handle: u64, index: u64) -> Result<u64, CallFailure> {
    let report = error(handle).map_err(CallFailure::Boundary)?;
    let index =
        usize::try_from(index).map_err(|_| CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?;
    match &report
        .details
        .get(index)
        .ok_or(CallFailure::Boundary(BOUNDARY_INVALID_INDEX))?
        .value
    {
        ErrorValue::Str(value) => insert_bytes(value.as_bytes().to_vec()),
        _ => Err(CallFailure::Boundary(BOUNDARY_WRONG_DETAIL_TYPE)),
    }
}

pub enum CallFailure {
    Boundary(u32),
    Core(TosumuError),
}
