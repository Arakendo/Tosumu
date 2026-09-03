//! MVP+10 shared-reader benchmarks against SQLite WAL transactions.
//!
//! These workloads include thread and reader-lifecycle costs. Tosumu clones its
//! intended `SharedKvStore` owner; SQLite opens one connection per reader
//! thread. Results are observations for this workload, not performance claims.

use std::hint::black_box;
use std::path::Path;
use std::sync::{Arc, Barrier};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tempfile::TempDir;
use tosumu_core::SharedKvStore;

const ROWS: u64 = 10_000;
const READS_PER_READER: u64 = 128;
const READER_COUNTS: [usize; 3] = [1, 4, 8];
const OVERLAP_READERS: usize = 4;
const PAYLOAD: [u8; 128] = [0x42; 128];

#[inline]
fn u64_key(value: u64) -> [u8; 8] {
    value.to_be_bytes()
}

#[inline]
fn xorshift64(mut value: u64) -> u64 {
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value
}

fn populate_shared(store: &SharedKvStore) {
    store
        .write(|transaction| {
            for index in 0..ROWS {
                transaction.put(&u64_key(index), &PAYLOAD)?;
            }
            Ok(())
        })
        .expect("populate shared store");
}

fn open_sqlite(path: &Path) -> rusqlite::Connection {
    let connection = rusqlite::Connection::open(path).expect("open SQLite database");
    connection
        .busy_timeout(std::time::Duration::from_secs(5))
        .expect("set SQLite busy timeout");
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS kv(
                 key BLOB PRIMARY KEY,
                 value BLOB NOT NULL
             ) WITHOUT ROWID;",
        )
        .expect("configure SQLite database");
    connection
}

fn populate_sqlite(connection: &rusqlite::Connection) {
    connection.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let mut statement = connection
        .prepare("INSERT INTO kv(key, value) VALUES(?1, ?2)")
        .unwrap();
    for index in 0..ROWS {
        statement
            .execute(rusqlite::params![
                u64_key(index).as_slice(),
                PAYLOAD.as_slice()
            ])
            .unwrap();
    }
    drop(statement);
    connection.execute_batch("COMMIT;").unwrap();
}

fn sqlite_get(connection: &rusqlite::Connection, key: &[u8]) -> Vec<u8> {
    connection
        .query_row(
            "SELECT value FROM kv WHERE key=?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .expect("read SQLite value")
}

fn run_shared_readers(store: &SharedKvStore, reader_count: usize) -> usize {
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..reader_count)
            .map(|reader| {
                let store = store.clone();
                scope.spawn(move || {
                    let snapshot = store.snapshot().expect("open shared snapshot");
                    let mut random = 0x9e37_79b9_u64 ^ reader as u64;
                    let mut observed = 0usize;
                    for _ in 0..READS_PER_READER {
                        random = xorshift64(random);
                        observed ^= snapshot
                            .get(&u64_key(random % ROWS))
                            .expect("read shared snapshot")
                            .expect("preloaded shared key")
                            .len();
                    }
                    observed
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("shared reader thread"))
            .fold(0, |checksum, observed| checksum ^ observed)
    })
}

fn run_sqlite_readers(path: &Path, reader_count: usize) -> usize {
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..reader_count)
            .map(|reader| {
                scope.spawn(move || {
                    let connection = open_sqlite(path);
                    connection.execute_batch("BEGIN;").unwrap();
                    let mut random = 0x9e37_79b9_u64 ^ reader as u64;
                    let mut observed = 0usize;
                    for _ in 0..READS_PER_READER {
                        random = xorshift64(random);
                        observed ^= sqlite_get(&connection, &u64_key(random % ROWS)).len();
                    }
                    connection.execute_batch("COMMIT;").unwrap();
                    observed
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("SQLite reader thread"))
            .fold(0, |checksum, observed| checksum ^ observed)
    })
}

fn benchmark_concurrent_readers(criterion: &mut Criterion) {
    let shared_directory = TempDir::new().unwrap();
    let shared_path = shared_directory.path().join("readers.tsm");
    let shared = SharedKvStore::create(&shared_path).unwrap();
    populate_shared(&shared);

    let sqlite_directory = TempDir::new().unwrap();
    let sqlite_path = sqlite_directory.path().join("readers.db");
    let sqlite = open_sqlite(&sqlite_path);
    populate_sqlite(&sqlite);
    drop(sqlite);

    let mut group = criterion.benchmark_group("concurrent_readers/plain");
    group.sample_size(20);
    for reader_count in READER_COUNTS {
        group.throughput(Throughput::Elements(reader_count as u64 * READS_PER_READER));
        group.bench_with_input(
            BenchmarkId::new("tosumu", reader_count),
            &reader_count,
            |bencher, &readers| {
                bencher.iter(|| black_box(run_shared_readers(&shared, readers)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("sqlite", reader_count),
            &reader_count,
            |bencher, &readers| {
                bencher.iter(|| black_box(run_sqlite_readers(&sqlite_path, readers)));
            },
        );
    }
    group.finish();
}

fn revision_payload(revision: u64) -> [u8; 128] {
    let mut payload = PAYLOAD;
    payload[..8].copy_from_slice(&revision.to_le_bytes());
    payload
}

fn run_shared_reader_writer(store: &SharedKvStore, revision: u64) {
    let key = u64_key(0);
    let expected = store.get(&key).unwrap().unwrap();
    let replacement = revision_payload(revision);
    let ready = Arc::new(Barrier::new(OVERLAP_READERS + 1));
    let release = Arc::new(Barrier::new(OVERLAP_READERS + 1));

    std::thread::scope(|scope| {
        for _ in 0..OVERLAP_READERS {
            let store = store.clone();
            let ready = Arc::clone(&ready);
            let release = Arc::clone(&release);
            let expected = &expected;
            scope.spawn(move || {
                let snapshot = store.snapshot().unwrap();
                assert_eq!(
                    snapshot.get(&key).unwrap().as_deref(),
                    Some(expected.as_slice())
                );
                ready.wait();
                release.wait();
                assert_eq!(
                    snapshot.get(&key).unwrap().as_deref(),
                    Some(expected.as_slice())
                );
            });
        }
        ready.wait();
        store.put(&key, &replacement).unwrap();
        release.wait();
    });

    assert_eq!(
        store.get(&key).unwrap().as_deref(),
        Some(replacement.as_slice())
    );
}

fn run_sqlite_reader_writer(writer: &rusqlite::Connection, path: &Path, revision: u64) {
    let key = u64_key(0);
    let expected = sqlite_get(writer, &key);
    let replacement = revision_payload(revision);
    let ready = Arc::new(Barrier::new(OVERLAP_READERS + 1));
    let release = Arc::new(Barrier::new(OVERLAP_READERS + 1));

    std::thread::scope(|scope| {
        for _ in 0..OVERLAP_READERS {
            let ready = Arc::clone(&ready);
            let release = Arc::clone(&release);
            let expected = &expected;
            scope.spawn(move || {
                let connection = open_sqlite(path);
                connection.execute_batch("BEGIN;").unwrap();
                assert_eq!(sqlite_get(&connection, &key), *expected);
                ready.wait();
                release.wait();
                assert_eq!(sqlite_get(&connection, &key), *expected);
                connection.execute_batch("COMMIT;").unwrap();
            });
        }
        ready.wait();
        writer
            .execute(
                "UPDATE kv SET value=?1 WHERE key=?2",
                rusqlite::params![replacement.as_slice(), key.as_slice()],
            )
            .unwrap();
        release.wait();
    });

    assert_eq!(sqlite_get(writer, &key), replacement);
}

fn benchmark_reader_writer(criterion: &mut Criterion) {
    let shared_directory = TempDir::new().unwrap();
    let shared_path = shared_directory.path().join("reader-writer.tsm");
    let shared = SharedKvStore::create(&shared_path).unwrap();
    populate_shared(&shared);

    let sqlite_directory = TempDir::new().unwrap();
    let sqlite_path = sqlite_directory.path().join("reader-writer.db");
    let sqlite = open_sqlite(&sqlite_path);
    populate_sqlite(&sqlite);

    let mut group = criterion.benchmark_group("reader_writer/plain");
    group.sample_size(20);
    group.throughput(Throughput::Elements((OVERLAP_READERS * 2 + 1) as u64));

    let mut shared_revision = 1u64;
    group.bench_function("tosumu", |bencher| {
        bencher.iter(|| {
            shared_revision += 1;
            run_shared_reader_writer(&shared, shared_revision);
        });
    });

    let mut sqlite_revision = 1u64;
    group.bench_function("sqlite", |bencher| {
        bencher.iter(|| {
            sqlite_revision += 1;
            run_sqlite_reader_writer(&sqlite, &sqlite_path, sqlite_revision);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    benchmark_concurrent_readers,
    benchmark_reader_writer
);
criterion_main!(benches);
