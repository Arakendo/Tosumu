// Write-Ahead Log (WAL) — durability for MVP +4.
//
// Source of truth: docs/Specifications/Tosumu Software Design Document.md §7.2–§7.3.
//
// Wire format per record:
//   lsn:         u64 LE  (8 bytes)
//   record_type: u8      (1 byte)
//   payload_len: u32 LE  (4 bytes)
//   payload:     [u8]    (payload_len bytes)
//   crc32:       u32 LE  (4 bytes) — crc32fast over [lsn..payload]
//
// Record types:
//   0x01 = Begin     { txn_id: u64 }
//   0x02 = PageWrite { pgno: u64, page_version: u64, frame: [u8; PAGE_SIZE] }
//   0x03 = Commit    { txn_id: u64 }
//   0x04 = Checkpoint{ up_to_lsn: u64 }
//
// Physical logging: PageWrite stores the full encrypted frame (PAGE_SIZE bytes).
// Recovery applies PageWrite records from committed transactions only.

use std::fs::{File, OpenOptions};
use std::io::{BufReader, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::error::{Result, TosumuError};
use crate::format::PAGE_SIZE;
use crate::writer_gate::WriterGuard;

mod index;
pub(crate) use index::CommittedWalIndex;

// ── Transient-lock retry ─────────────────────────────────────────────────────

/// Default maximum number of retry attempts when a transient file-lock error is
/// encountered before giving up with `TosumuError::FileBusy`.
///
/// Each retry waits 10 ms in production (skipped in tests).  Total worst-case
/// wait with the default is `DEFAULT_MAX_RETRIES × 10 ms = 50 ms`.
///
/// AV scanners, backup tools, or indexers can hold locks for longer.  If you
/// observe `FileBusy` errors under normal operation, increase this value via a
/// future `DatabaseConfig` (TODO Stage N: expose via `DatabaseConfig::lock_retry_budget`).
pub const DEFAULT_MAX_RETRIES: u32 = 5;

// Internal alias so existing code stays readable.
const MAX_RETRIES: u32 = DEFAULT_MAX_RETRIES;

/// Returns `true` if `e` is a transient file-lock error that may resolve on retry.
///
/// - Windows: ERROR_SHARING_VIOLATION (32), ERROR_LOCK_VIOLATION (33).
/// - Test mode: OS error 32 is accepted as a fault-injection signal on all platforms.
fn is_transient_lock(e: &std::io::Error) -> bool {
    #[cfg(windows)]
    if matches!(e.raw_os_error(), Some(32) | Some(33)) {
        return true;
    }
    // Fault injection in tests synthesises OS error 32 on any platform.
    #[cfg(test)]
    if e.raw_os_error() == Some(32) {
        return true;
    }
    let _ = e;
    false
}

/// In non-test builds: delegate directly to `open_fn`.
#[cfg(not(test))]
fn inject_or_open(open_fn: &impl Fn() -> std::io::Result<File>) -> std::io::Result<File> {
    open_fn()
}

/// In test builds: consume a fault-injection ticket before calling `open_fn`.
#[cfg(test)]
fn inject_or_open(open_fn: &impl Fn() -> std::io::Result<File>) -> std::io::Result<File> {
    if fault_injection::should_inject() {
        Err(std::io::Error::from_raw_os_error(32))
    } else {
        open_fn()
    }
}

/// Open a file with bounded retry on transient lock errors.
///
/// Makes up to `max_retries + 1` attempts.  Each transient failure (lock held
/// by another process) waits 10 ms before retrying.  After exhausting all
/// attempts returns `TosumuError::FileBusy { path, operation }`.
///
/// Non-transient errors (permission denied, file not found, …) propagate
/// immediately without retrying.
fn open_file_retrying<F>(path: &Path, open_fn: F, operation: &'static str) -> Result<File>
where
    F: Fn() -> std::io::Result<File>,
{
    open_file_retrying_n(path, open_fn, operation, MAX_RETRIES)
}

/// Like `open_file_retrying` but with an explicit retry limit.  Kept internal
/// for now; will be surfaced via `DatabaseConfig` in a future stage.
fn open_file_retrying_n<F>(
    path: &Path,
    open_fn: F,
    operation: &'static str,
    max_retries: u32,
) -> Result<File>
where
    F: Fn() -> std::io::Result<File>,
{
    for attempt in 0..=max_retries {
        match inject_or_open(&open_fn) {
            Ok(f) => return Ok(f),
            Err(e) if is_transient_lock(&e) && attempt < max_retries => {
                // Brief pause to let the lock holder release.
                // Skipped in tests to keep the suite fast.
                #[cfg(not(test))]
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) if is_transient_lock(&e) => {
                // Retries exhausted — report as FileBusy.
                return Err(TosumuError::FileBusy {
                    path: path.to_path_buf(),
                    operation,
                });
            }
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!("loop exits via Ok or FileBusy")
}

// ── Fault injection (test-only) ───────────────────────────────────────────────

/// Fault injection state for lock-error simulation in tests.
///
/// Tests that use fault injection MUST hold `LOCK` for their duration to
/// prevent the fault counter from bleeding into parallel tests.
#[cfg(test)]
pub(crate) mod fault_injection {
    use std::cell::Cell;
    use std::sync::Mutex;

    /// Serialises all fault-injection tests.
    pub static LOCK: Mutex<()> = Mutex::new(());
    thread_local! {
        static FAULTS: Cell<u32> = const { Cell::new(0) };
    }

    /// Set the number of lock faults to inject.
    pub fn arm(n: u32) {
        FAULTS.with(|faults| faults.set(n));
    }

    /// Clear all pending faults (called by `FaultGuard` on drop).
    pub fn disarm() {
        FAULTS.with(|faults| faults.set(0));
    }

    /// Atomically consume one fault ticket.  Returns `true` iff a fault should
    /// be injected (counter was > 0 and was decremented).
    pub fn should_inject() -> bool {
        FAULTS.with(|faults| {
            let remaining = faults.get();
            if remaining == 0 {
                false
            } else {
                faults.set(remaining - 1);
                true
            }
        })
    }
}

/// RAII guard that clears fault injection state on drop (normal exit *and* panic).
#[cfg(test)]
struct FaultGuard;
#[cfg(test)]
impl Drop for FaultGuard {
    fn drop(&mut self) {
        fault_injection::disarm();
    }
}

// ── Record type discriminants ─────────────────────────────────────────────────

const RT_BEGIN: u8 = 0x01;
const RT_PAGE_WRITE: u8 = 0x02;
const RT_COMMIT: u8 = 0x03;
const RT_CHECKPOINT: u8 = 0x04;

/// Largest payload any valid WAL record can carry.
///
/// PageWrite stores: pgno(u64) + page_version(u64) + full page frame.
const MAX_WAL_PAYLOAD_LEN: usize = 16 + PAGE_SIZE;

// ── Header sizes ─────────────────────────────────────────────────────────────

/// Fixed overhead per record: lsn(8) + type(1) + payload_len(4) + crc32(4) = 17 bytes.
pub const RECORD_HEADER_SIZE: usize = 17;

// ── WalRecord ────────────────────────────────────────────────────────────────

/// A decoded WAL record.
#[derive(Debug, Clone)]
pub enum WalRecord {
    Begin {
        txn_id: u64,
    },
    PageWrite {
        pgno: u64,
        page_version: u64,
        frame: Box<[u8; PAGE_SIZE]>,
    },
    Commit {
        txn_id: u64,
    },
    Checkpoint {
        up_to_lsn: u64,
    },
}

impl WalRecord {
    /// Encode this record into `out` with the given `lsn`.
    ///
    /// Layout: [lsn u64][type u8][payload_len u32][payload][crc32 u32]
    pub fn encode(&self, lsn: u64, out: &mut Vec<u8>) {
        let payload = self.encode_payload();
        let payload_len = payload.len() as u32;

        let header_start = out.len();
        out.extend_from_slice(&lsn.to_le_bytes());
        out.push(self.type_byte());
        out.extend_from_slice(&payload_len.to_le_bytes());
        out.extend_from_slice(&payload);

        // CRC32 covers [lsn..end-of-payload].
        let crc = crc32fast::hash(&out[header_start..]);
        out.extend_from_slice(&crc.to_le_bytes());
    }

    fn type_byte(&self) -> u8 {
        match self {
            WalRecord::Begin { .. } => RT_BEGIN,
            WalRecord::PageWrite { .. } => RT_PAGE_WRITE,
            WalRecord::Commit { .. } => RT_COMMIT,
            WalRecord::Checkpoint { .. } => RT_CHECKPOINT,
        }
    }

    fn encode_payload(&self) -> Vec<u8> {
        match self {
            WalRecord::Begin { txn_id } => txn_id.to_le_bytes().to_vec(),
            WalRecord::PageWrite {
                pgno,
                page_version,
                frame,
            } => {
                let mut p = Vec::with_capacity(8 + 8 + PAGE_SIZE);
                p.extend_from_slice(&pgno.to_le_bytes());
                p.extend_from_slice(&page_version.to_le_bytes());
                p.extend_from_slice(frame.as_ref());
                p
            }
            WalRecord::Commit { txn_id } => txn_id.to_le_bytes().to_vec(),
            WalRecord::Checkpoint { up_to_lsn } => up_to_lsn.to_le_bytes().to_vec(),
        }
    }
}

// ── WalWriter ────────────────────────────────────────────────────────────────

/// Appends WAL records to the `.wal` sidecar file.
/// Low-level physical WAL writer.
///
/// # Concurrency
///
/// Direct `WalWriter` mutation does not participate in database writer
/// admission because this type receives a WAL path, not a database identity.
/// It is unsupported to use these mutation methods while a database handle or
/// coordinated maintenance operation may access the same database/WAL pair.
pub struct WalWriter {
    file: File,
    /// LSN to assign to the next record written.
    next_lsn: u64,
}

impl WalWriter {
    /// Create a new WAL file. Fails if the file already exists.
    pub fn create(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(WalWriter { file, next_lsn: 1 })
    }

    /// Open an existing WAL file for appending.
    ///
    /// Scans all validated existing records to determine `next_lsn` and trims
    /// any torn partial tail before appending.
    pub fn open(path: &Path) -> Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let (max_lsn, append_offset) = scan_append_state(&file)?;
        let next_lsn = max_lsn.checked_add(1).ok_or(TosumuError::CorruptRecord {
            offset: append_offset,
            reason: "WAL LSN overflow",
        })?;
        if file.metadata()?.len() > append_offset {
            file.set_len(append_offset)?;
        }
        let mut w = WalWriter { file, next_lsn };
        w.file.seek(SeekFrom::Start(append_offset))?;
        Ok(w)
    }

    /// Open or create: if the WAL does not exist, create it; otherwise open for append.
    pub fn open_or_create(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::open(path)
        } else {
            Self::create(path)
        }
    }

    /// Write one record to the WAL. Does NOT fsync — call `sync()` after a full transaction.
    pub fn append(&mut self, record: &WalRecord) -> Result<u64> {
        let lsn = self.next_lsn;
        let mut buf = Vec::with_capacity(RECORD_HEADER_SIZE + PAGE_SIZE);
        record.encode(lsn, &mut buf);
        self.file.write_all(&buf)?;
        self.next_lsn += 1;
        Ok(lsn)
    }

    /// Flush OS buffers to durable storage.
    pub fn sync(&mut self) -> Result<()> {
        self.file.sync_data()?;
        Ok(())
    }

    /// The LSN that will be assigned to the next appended record.
    pub fn next_lsn(&self) -> u64 {
        self.next_lsn
    }

    /// Truncate the WAL to zero bytes (used after a full checkpoint).
    pub fn truncate(&mut self) -> Result<()> {
        self.file.seek(SeekFrom::Start(0))?;
        self.file.set_len(0)?;
        self.file.sync_data()?;
        self.next_lsn = 1;
        Ok(())
    }
}

trait WalCheckpointTruncateFile: Seek {
    fn set_len(&mut self, size: u64) -> std::io::Result<()>;
    fn sync_data(&mut self) -> std::io::Result<()>;
}

impl WalCheckpointTruncateFile for File {
    fn set_len(&mut self, size: u64) -> std::io::Result<()> {
        File::set_len(self, size)
    }

    fn sync_data(&mut self) -> std::io::Result<()> {
        File::sync_data(self)
    }
}

#[cfg(test)]
impl WalCheckpointTruncateFile for crate::test_helpers::CrashFile {
    fn set_len(&mut self, size: u64) -> std::io::Result<()> {
        crate::test_helpers::CrashFile::set_len(self, size)
    }

    fn sync_data(&mut self) -> std::io::Result<()> {
        crate::test_helpers::CrashFile::sync_data(self)
    }
}

fn truncate_wal_after_checkpoint<T: WalCheckpointTruncateFile>(file: &mut T) -> Result<()> {
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.sync_data()?;
    Ok(())
}

// ── WalReader ────────────────────────────────────────────────────────────────

/// Reads WAL records sequentially from a `.wal` file.
pub struct WalReader {
    reader: BufReader<File>,
    /// Byte offset of the start of the record currently being read.
    pos: u64,
}

impl WalReader {
    /// Open a WAL file for reading from the beginning.
    pub fn open(path: &Path) -> Result<Self> {
        let file = open_file_retrying(
            path,
            || OpenOptions::new().read(true).open(path),
            "opening WAL for record replay",
        )?;
        Ok(WalReader {
            reader: BufReader::new(file),
            pos: 0,
        })
    }

    /// Read the next record. Returns `None` at clean EOF; error on truncation/CRC failure.
    pub fn next_record(&mut self) -> Result<Option<(u64, WalRecord)>> {
        let record_start = self.pos;
        // Read fixed header: lsn(8) + type(1) + payload_len(4).
        let mut hdr = [0u8; 13];
        match self.reader.read_exact(&mut hdr) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }

        let lsn = u64::from_le_bytes(hdr[0..8].try_into().unwrap());
        let record_type = hdr[8];
        let payload_len = u32::from_le_bytes(hdr[9..13].try_into().unwrap()) as usize;
        validate_payload_len(payload_len, record_start)?;

        let mut payload = vec![0u8; payload_len];
        self.reader.read_exact(&mut payload).map_err(|e| {
            if e.kind() == ErrorKind::UnexpectedEof {
                TosumuError::CorruptRecord {
                    offset: record_start,
                    reason: "WAL record truncated in payload",
                }
            } else {
                e.into()
            }
        })?;

        let mut crc_bytes = [0u8; 4];
        self.reader.read_exact(&mut crc_bytes).map_err(|e| {
            if e.kind() == ErrorKind::UnexpectedEof {
                TosumuError::CorruptRecord {
                    offset: record_start,
                    reason: "WAL record truncated in CRC",
                }
            } else {
                e.into()
            }
        })?;
        let stored_crc = u32::from_le_bytes(crc_bytes);

        // Verify CRC: covers [lsn..end-of-payload].
        let mut covered = Vec::with_capacity(13 + payload_len);
        covered.extend_from_slice(&hdr);
        covered.extend_from_slice(&payload);
        let computed_crc = crc32fast::hash(&covered);
        if computed_crc != stored_crc {
            return Err(TosumuError::CorruptRecord {
                offset: record_start,
                reason: "WAL record CRC mismatch",
            });
        }

        // Advance position past this record: 13-byte header + payload + 4-byte CRC.
        self.pos = record_start + 13 + payload_len as u64 + 4;

        let record = decode_payload(record_type, &payload, record_start)?;
        Ok(Some((lsn, record)))
    }

    /// Collect all valid records from the WAL, stopping at the first CRC error or EOF.
    pub fn read_all(path: &Path) -> Result<Vec<(u64, WalRecord)>> {
        let mut rdr = Self::open(path)?;
        let mut out = Vec::new();
        loop {
            match rdr.next_record() {
                Ok(Some(r)) => out.push(r),
                Ok(None) => break,
                // Ignore only torn tail records; complete-record corruption must surface.
                Err(TosumuError::CorruptRecord {
                    reason: "WAL record truncated in payload",
                    ..
                }) => break,
                Err(TosumuError::CorruptRecord {
                    reason: "WAL record truncated in CRC",
                    ..
                }) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }
}

/// Visit each sequentially framed transaction that reaches its matching commit.
///
/// Transaction IDs are not treated as globally unique: a later incomplete
/// transaction may reuse an earlier ID without making its frames committed.
pub(crate) fn for_each_committed_transaction<F>(
    records: &[(u64, WalRecord)],
    mut visit: F,
) -> Result<()>
where
    F: FnMut(u64, u64, &[(u64, WalRecord)]) -> Result<()>,
{
    let mut active: Option<(u64, usize)> = None;

    for (index, (lsn, record)) in records.iter().enumerate() {
        match record {
            WalRecord::Begin { txn_id } => active = Some((*txn_id, index + 1)),
            WalRecord::Commit { txn_id } => {
                if let Some((active_txn_id, first_record)) = active {
                    if active_txn_id == *txn_id {
                        visit(*txn_id, *lsn, &records[first_record..index])?;
                        active = None;
                    }
                }
            }
            WalRecord::PageWrite { .. } | WalRecord::Checkpoint { .. } => {}
        }
    }

    Ok(())
}

// ── Recovery ─────────────────────────────────────────────────────────────────

/// Apply all committed WAL transactions from `wal_path` into `db_path`.
///
/// For each committed transaction (Begin … Commit pair), every `PageWrite`
/// record is written into the main `.tsm` file at the correct page offset.
/// Uncommitted records (no matching `Commit`) are discarded.
///
/// Returns the LSN of the last checkpoint record seen (0 if none).
pub fn recover(db_path: &Path, wal_path: &Path) -> Result<u64> {
    let writer_guard = WriterGuard::acquire(db_path)?;
    recover_guarded(db_path, wal_path, &writer_guard)
}

pub(crate) fn recover_guarded(
    db_path: &Path,
    wal_path: &Path,
    _writer_guard: &WriterGuard,
) -> Result<u64> {
    if !wal_path.exists() {
        return Ok(0);
    }

    let records = WalReader::read_all(wal_path)?;

    let mut last_checkpoint_lsn = 0u64;

    // Open the main file for writing page frames.
    let mut db_file = open_file_retrying(
        db_path,
        || OpenOptions::new().read(true).write(true).open(db_path),
        "applying WAL recovery to database",
    )?;

    for (_, record) in &records {
        if let WalRecord::Checkpoint { up_to_lsn } = record {
            last_checkpoint_lsn = *up_to_lsn;
        }
    }

    apply_committed_writes(&records, &mut db_file)?;

    db_file.sync_data()?;
    Ok(last_checkpoint_lsn)
}

/// Walk records in order, tracking the current txn_id.
/// Apply PageWrite records that belong to committed transactions.
fn apply_committed_writes(records: &[(u64, WalRecord)], db_file: &mut File) -> Result<()> {
    for_each_committed_transaction(records, |_, _, transaction_records| {
        for (_, record) in transaction_records {
            match record {
                WalRecord::PageWrite { pgno, frame, .. } => {
                    let offset =
                        pgno.checked_mul(PAGE_SIZE as u64)
                            .ok_or(TosumuError::Corrupt {
                                pgno: *pgno,
                                reason: "WAL page offset overflow",
                            })?;
                    db_file.seek(SeekFrom::Start(offset))?;
                    db_file.write_all(frame.as_ref())?;
                }
                WalRecord::Begin { .. }
                | WalRecord::Commit { .. }
                | WalRecord::Checkpoint { .. } => {}
            }
        }
        Ok(())
    })
}

// ── Checkpoint ───────────────────────────────────────────────────────────────

/// Checkpoint: copy committed WAL frames into the main `.tsm` file, then truncate the WAL.
///
/// Equivalent to a full checkpoint (`CheckpointMode::Truncate` in §7.8).
/// For MVP+4 there is no reader LSN pinning — the WAL is always fully truncated.
pub fn checkpoint(db_path: &Path, wal_path: &Path) -> Result<()> {
    let writer_guard = WriterGuard::acquire(db_path)?;
    checkpoint_guarded(db_path, wal_path, &writer_guard)
}

pub(crate) fn checkpoint_guarded(
    db_path: &Path,
    wal_path: &Path,
    writer_guard: &WriterGuard,
) -> Result<()> {
    recover_guarded(db_path, wal_path, writer_guard)?;
    // Truncate WAL — only reached if recovery succeeded, so safe to overwrite.
    let mut file = open_file_retrying(
        wal_path,
        || OpenOptions::new().write(true).open(wal_path),
        "truncating WAL during checkpoint",
    )?;
    truncate_wal_after_checkpoint(&mut file)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the path of the WAL sidecar for a given `.tsm` database path.
pub fn wal_path(db_path: &Path) -> PathBuf {
    let mut p = db_path.as_os_str().to_owned();
    p.push(".wal");
    PathBuf::from(p)
}

fn decode_payload(record_type: u8, payload: &[u8], offset: u64) -> Result<WalRecord> {
    match record_type {
        RT_BEGIN => {
            if payload.len() < 8 {
                return Err(TosumuError::CorruptRecord {
                    offset,
                    reason: "Begin payload too short",
                });
            }
            Ok(WalRecord::Begin {
                txn_id: u64::from_le_bytes(payload[0..8].try_into().unwrap()),
            })
        }
        RT_PAGE_WRITE => {
            let expected = 8 + 8 + PAGE_SIZE;
            if payload.len() < expected {
                return Err(TosumuError::CorruptRecord {
                    offset,
                    reason: "PageWrite payload too short",
                });
            }
            let pgno = u64::from_le_bytes(payload[0..8].try_into().unwrap());
            let page_version = u64::from_le_bytes(payload[8..16].try_into().unwrap());
            let mut frame = Box::new([0u8; PAGE_SIZE]);
            frame.copy_from_slice(&payload[16..16 + PAGE_SIZE]);
            Ok(WalRecord::PageWrite {
                pgno,
                page_version,
                frame,
            })
        }
        RT_COMMIT => {
            if payload.len() < 8 {
                return Err(TosumuError::CorruptRecord {
                    offset,
                    reason: "Commit payload too short",
                });
            }
            Ok(WalRecord::Commit {
                txn_id: u64::from_le_bytes(payload[0..8].try_into().unwrap()),
            })
        }
        RT_CHECKPOINT => {
            if payload.len() < 8 {
                return Err(TosumuError::CorruptRecord {
                    offset,
                    reason: "Checkpoint payload too short",
                });
            }
            Ok(WalRecord::Checkpoint {
                up_to_lsn: u64::from_le_bytes(payload[0..8].try_into().unwrap()),
            })
        }
        _ => Err(TosumuError::CorruptRecord {
            offset,
            reason: "unknown WAL record type",
        }),
    }
}

fn validate_payload_len(payload_len: usize, offset: u64) -> Result<()> {
    if payload_len > MAX_WAL_PAYLOAD_LEN {
        return Err(TosumuError::CorruptRecord {
            offset,
            reason: "WAL payload_len out of range",
        });
    }
    Ok(())
}

/// Scan the WAL to find the highest LSN and the byte offset after the last
/// fully validated record. A truncated tail is ignored; structured corruption
/// in a complete record is surfaced as an error.
fn scan_append_state(file: &File) -> Result<(u64, u64)> {
    let mut reader = BufReader::new(file.try_clone()?);
    let mut max_lsn = 0u64;
    let mut offset = 0u64;
    loop {
        let record_start = offset;
        let mut hdr = [0u8; 13];
        match reader.read_exact(&mut hdr) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }
        let lsn = u64::from_le_bytes(hdr[0..8].try_into().unwrap());
        let record_type = hdr[8];
        let payload_len = u32::from_le_bytes(hdr[9..13].try_into().unwrap()) as usize;
        validate_payload_len(payload_len, record_start)?;

        let mut payload = vec![0u8; payload_len];
        match reader.read_exact(&mut payload) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let mut crc_bytes = [0u8; 4];
        match reader.read_exact(&mut crc_bytes) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(e.into()),
        }

        let stored_crc = u32::from_le_bytes(crc_bytes);
        let mut covered = Vec::with_capacity(13 + payload_len);
        covered.extend_from_slice(&hdr);
        covered.extend_from_slice(&payload);
        let computed_crc = crc32fast::hash(&covered);
        if computed_crc != stored_crc {
            return Err(TosumuError::CorruptRecord {
                offset: record_start,
                reason: "WAL record CRC mismatch",
            });
        }

        decode_payload(record_type, &payload, record_start)?;
        offset = record_start + 13 + payload_len as u64 + 4;
        max_lsn = max_lsn.max(lsn);
    }
    Ok((max_lsn, offset))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "wal/tests/mod.rs"]
mod tests;
