# Reference Implementations

This document lists external codebases and resources that inform tosumu's implementation. These are battle-tested algorithms and patterns we can learn from.

---

## DataStructures Library (C#)

**Location:** `F:\LocalSource\ClassLibrary\DataStructures`

A comprehensive .NET 8 library with **65+ data structures**, 836+ unit tests, and production-quality implementations. Several structures are directly relevant to tosumu's core engine.

### High-priority references

These implementations should be consulted during the corresponding stage:

#### **BPlusTree.cs** — Stage 2 (B+ tree index)
**Path:** `Trees/BPlusTree.cs`

**What to reference:**
- Node splitting logic and thresholds (when to split an internal vs. leaf node).
- Maintaining leaf-node links during splits and merges.
- Delete rebalancing strategies (merge sibling vs. redistribute keys).
- Order statistics implementation (`Rank`, `Select`).

**Adaptation notes:**
- C# version is in-memory (node pointers). Tosumu's version is page-based (page numbers).
- Key translation: `Node*` → `PageNum`, pointer chasing → `pager.get_page(page_num)`.
- Core split/merge/redistribute algorithms are language-agnostic.

**Usage:**
```bash
# Review before implementing tosumu's btree.rs
code F:\LocalSource\ClassLibrary\DataStructures\Trees\BPlusTree.cs
```

---

#### **LruCache.cs** — Stage 1 (page cache eviction)
**Path:** `Caches/LruCache.cs`

**What to reference:**
- `Dictionary<TKey, LinkedListNode<T>>` + `LinkedList<T>` pattern for O(1) get/put/evict.
- Move-to-front logic on access (mark as recently used).
- Eviction callback (`OnEvicted`) for flushing dirty pages before removal.

**Adaptation notes:**
- Translate to Rust: `HashMap<PageNum, *mut Node>` + intrusive doubly-linked list.
- Use `unsafe` for raw pointer manipulation (or `Rc<RefCell<>>` for safe alternative).
- Must flush dirty pages in the eviction callback before reclaiming the slot.

**Usage:**
```bash
# Review before implementing tosumu's pager.rs cache
code F:\LocalSource\ClassLibrary\DataStructures\Caches\LruCache.cs
```

---

#### **BloomFilter.cs** — Stage 6+ (negative lookup optimization)
**Path:** `Probabilistic/BloomFilter.cs`

**What to reference:**
- Optimal sizing formula: `m = -n * ln(p) / (ln(2)^2)`, `k = (m/n) * ln(2)`.
- Double-hashing trick for deriving k hash functions from two base hashes.
- False-positive rate estimation as the filter fills.

**Adaptation notes:**
- Tosumu use case: per-page or per-table Bloom filters to skip pages during scans.
- Store filter bits in page header or dedicated metadata page.
- Rebuild filter on page compaction or table rebuild.

**When to implement:** After Stage 2 works and performance profiling shows scan bottlenecks.

**Usage:**
```bash
# Reference when adding Bloom filters (Stage 6+)
code F:\LocalSource\ClassLibrary\DataStructures\Probabilistic\BloomFilter.cs
```

---

#### **RingBuffer.cs** — Stage 3 (WAL buffering)
**Path:** `Buffers/RingBuffer.cs`

**What to reference:**
- Fixed-size circular buffer with head/tail pointers.
- Overwrite semantics when full.
- O(1) push/pop at both ends.

**Adaptation notes:**
- Tosumu use case: batch WAL frames in memory before fsyncing to disk.
- Size = number of dirty pages buffered before flush (e.g., 64 frames).
- On overflow, trigger early flush instead of overwriting.

**Usage:**
```bash
# Review before implementing WAL frame buffering
code F:\LocalSource\ClassLibrary\DataStructures\Buffers\RingBuffer.cs
```

---

### Interesting for future extensions

These are out of scope for Stages 1–6 but could inform hypothetical Stage 7+ work.

#### **MerkleTree.cs** — Hardened integrity mode
**Path:** `Trees/MerkleTree.cs`

**What it offers:**
- O(log n) proof generation and verification.
- Incremental verification (prove one page is authentic without reading entire DB).
- Tamper detection for individual pages.

**Tosumu integration idea:**
- Replace single header MAC with Merkle root over all page hashes.
- Store internal node hashes in a separate metadata page or in-memory.
- Recompute root hash on every page write (expensive but strong).

**Trade-offs:**
- Adds O(log n) storage overhead.
- Write amplification (update page → recompute path to root).
- Benefit: Can prove "page 42 is authentic" without loading entire DB.

**When to consider:** If tosumu needs cryptographic audit logs or wants a "paranoid integrity mode."

---

#### **RTree.cs** — Spatial indexing
**Path:** `Spatial/RTree.cs`

**Relevance:** DESIGN.md §17.3 mentions spatial indexes as a "reasonable Stage 7+ extension." This is a reference implementation for bounding-box splits and nearest-neighbor queries.

---

#### **Trie.cs / RadixTree.cs** — Prefix search
**Path:** `Trees/Trie.cs`, `Trees/RadixTree.cs`

**Relevance:** Simpler than FSTs (DESIGN.md §17.2.1), incrementally mutable. Could be adapted to page-based storage for autocomplete-style queries on string keys. Less compact than FSTs but easier to implement.

---

### Out of scope

The following structures in the DataStructures library are well-implemented but not relevant to tosumu's goals:

- **Text editing structures** (Rope, PieceTable, GapBuffer) — tosumu stores key/value pairs, not documents.
- **Persistent data structures** (PersistentList, PersistentMap) — tosumu uses mutable pages with WAL, not immutable trees.
- **Statistics / Simulation** (EWMA, Markov chains, etc.) — not storage-engine concerns.
- **Graph structures** — tosumu is key/value, not graph traversal (DESIGN.md §17.4 explicitly rules this out).
- **DancingLinks** (exact cover solver) — unrelated to database internals.

---

## DatabaseTools Library (C#)

**Location:** `F:\LocalSource\ClassLibrary\DatabaseTools`

A comprehensive database tooling library with migration runner, schema builder, sync engine, and backup/export/import utilities. Built for SQLite but has design patterns applicable to any embedded database.

### Migration system

#### **ICodeMigration** + **SqliteMigrationRunner** — Directly relevant to DESIGN.md §12
**Path:** `ICodeMigration.cs`, `SqliteMigrationRunner.cs`, `MIGRATIONS.md`

**What to reference:**
- **Version-based migration ordering** — each migration has a `long Version` (uses timestamps like `202602190900`).
- **Up/Down pattern** — forward migration + explicit rollback. Tosumu's §12.6 `FormatMigration` trait mirrors this.
- **Migration history table** — `__MigrationHistory` tracks applied migrations. Tosumu should store this in page 0 header or a system page.
- **Transaction wrapping** — migrations run inside transactions by default, with opt-out for long-running ops.
- **Discovery + sorting** — runner auto-discovers migrations, sorts by version, applies pending ones.

**Adaptation notes:**
- C# version uses `SqliteConnection` + ADO.NET. Tosumu's version works on raw pages.
- Core pattern is identical: versioned changes, recorded history, safe rollback.
- Tosumu's migration categories (§12.2) are more granular (metadata-only vs. page-local vs. destructive).

**Key lesson:** The **migration history table** pattern is crucial. Tosumu should store `(version, name, applied_at, rolled_back_at)` tuples, not just `format_version` in the header.

**Usage:**
```bash
# Review before implementing tosumu's migration system
code F:\LocalSource\ClassLibrary\DatabaseTools\MIGRATIONS.md
code F:\LocalSource\ClassLibrary\DatabaseTools\ICodeMigration.cs
code F:\LocalSource\ClassLibrary\DatabaseTools\SqliteMigrationRunner.cs
```

---

#### **SqliteSchemaBuilder** — Fluent DDL pattern
**Path:** `SqliteSchemaBuilder.cs`

**What to reference:**
- Fluent API for DDL (`CreateTable`, `AddColumn`, `CreateIndex`, etc.).
- Table-level constraints (primary key, foreign key, unique).
- Accumulates statements, then executes as a batch.

**Tosumu use case:**
- If tosumu's Stage 5 SQL layer grows DDL support (`CREATE TABLE`, `CREATE INDEX`), this is a reference for the builder pattern.
- Not urgent for Stage 1–4 (hand-written page layouts), but useful for Stage 5+.

---

### Sync engine

#### **SyncEngine** + **ISyncChangeLog** — Ideas for future multi-device sync
**Path:** `SyncEngine.cs`, `SqliteSyncChangeLog.cs`, `SYNC.md`

**What it does:**
- **Change tracking** — captures INSERT/UPDATE/DELETE via application-level triggers.
- **ULID-based change IDs** — globally unique, time-sortable, no clock sync required.
- **Bidirectional sync** — push local changes to server, pull remote changes, apply both.
- **Conflict resolution** — LastWriteWins (ULID timestamp), ServerWins, ClientWins, Manual.
- **Watermarks** — track sync progress per remote node, only exchange new changes.
- **Offline-first** — all changes captured locally, sync when connectivity available.

**Tosumu integration idea (Stage 7+, hypothetical):**
- Store change log in a system table: `_sync_changelog(id ULID, table_name, operation, row_key, data, timestamp)`.
- Use tosumu's LSN (log sequence number) instead of ULID for ordering.
- Conflict resolution for distributed tosumu instances (not single-process anymore).
- Would require rearchitecting tosumu from single-writer to multi-writer with coordination.

**Why this is deferred:**
- Tosumu is explicitly single-process (DESIGN.md §1.2).
- Sync is a Stage 7+ extension, not core design.
- But: the **ULID-based change ID** pattern is excellent for globally-ordered events in distributed systems.

**When to reference:** If tosumu ever needs distributed sync, cloud backup, or multi-device replication.

---

### Other useful patterns

- **DatabaseExporter/Importer** — full-DB export to JSON/SQLite/CSV. Useful for tosumu's backup/restore (Stage 4b "backup before migration").
- **SchemaComparer** — detects differences between two databases. Tosumu could use this for migration validation tests.
- **Content packages** — self-contained SQLite files with metadata. Similar to tosumu's fixture files (§10.9).

---

## MemoryStore Library (C#)

**Location:** `F:\LocalSource\ClassLibrary\MemoryStore`

An in-memory virtual file system with URI-based resource management. Thread-safe, no disk I/O, comprehensive text/binary/JSON/XML operations.

### Interesting patterns for tosumu

#### **ConcurrentDictionary with case-insensitive URI keys** — Portable resource lookup
**Path:** `InMemoryResourceStore.cs`

**What to reference:**
- Custom `IEqualityComparer<Uri>` for case-insensitive lookups (cross-platform Windows/Linux compatibility).
- `ConcurrentDictionary<Uri, byte[]>` for thread-safe storage.
- Separate timestamp tracking (`ConcurrentDictionary<Uri, DateTime>`) for metadata.

**Tosumu use case:**
- If tosumu ever needs an in-memory mode (no disk writes, useful for testing), this is the pattern.
- Could be a `Database::open_memory()` constructor that uses a memory-backed pager instead of file I/O.
- Testing: fast, deterministic, no temp file cleanup.

**When to reference:** If Stage 1 testing becomes slow due to disk I/O, or if someone requests an in-memory mode.

---

#### **Content-type inference** — Metadata from file extensions
**Path:** `FEATURE_SUMMARY.md`, `GetContentType` method

**What it does:**
- Maps file extensions to MIME types (`.json` → `application/json`, `.xml` → `text/xml`, etc.).
- Used for serving resources via HTTP or generating metadata.

**Tosumu use case:**
- Not directly relevant (tosumu stores opaque key/value pairs, not files).
- But: if tosumu's Stage 5 SQL layer grows blob storage with content-type metadata, this is a reference.

**Deferred to Stage 5+ or never.**

---

#### **Hashing and comparison** — Integrity checks
**Path:** `GetMD5Hash`, `GetSHA256Hash`, `AreEqual` methods

**What it does:**
- Compute cryptographic hashes (MD5, SHA256) over stored resources.
- Byte-by-byte equality comparison.

**Tosumu use case:**
- Tosumu already has HMAC-SHA256 for header MAC (§8.4) and AEAD tags for pages (§8.2).
- This pattern is useful for **migration validation tests**: compute hash of pre-migration file, run migration, verify hash of specific pages or metadata hasn't changed when it shouldn't.
- Could also be used for **backup integrity** — hash backup files, verify on restore.

**When to reference:** Stage 3+ (WAL checksums), Stage 4 (migration validation), Stage 6 (backup integrity).

---

### Why MemoryStore is less critical for tosumu

MemoryStore is designed for **document processing pipelines** (XML transformations, XSLT, FOP rendering). Tosumu is a **page-based storage engine**. The overlap is smaller than with DataStructures or DatabaseTools.

**What's useful:**
- In-memory mode pattern (testing, fast iteration).
- Thread-safe resource management (if tosumu grows concurrent readers in Stage 6).
- Metadata tracking pattern (timestamps, content types).

**What's not useful:**
- XML/XSLT integration (tosumu stores opaque blobs, not documents).
- URI-based resource resolution (tosumu uses page numbers, not URIs).

---

## Other references

### SQLite source code
**URL:** https://www.sqlite.org/src/doc/trunk/README.md

**What to study:**
- B-tree implementation in `btree.c` — page-based node representation, overflow pages, cell format.
- Pager in `pager.c` — page cache, dirty page tracking, write-ahead log integration.
- WAL implementation in `wal.c` — frame format, checkpointing, readers/writer coordination.

**Tosumu differences:**
- SQLite is unencrypted by default (encryption via extensions like SQLCipher).
- SQLite's B-tree is more complex (supports interior data, overflow chains, etc.).
- Tosumu's design is simpler by choice (no overflow pages in Stage 1, no interior data).

---

### LevelDB / RocksDB source code
**URL:** https://github.com/google/leveldb

**What to study (if considering LSM-tree mode in §17.3):**
- SSTable format (sorted string table) — immutable files with Bloom filters.
- Compaction strategies (leveled, tiered).
- Memtable + write-ahead log pattern.

**Tosumu differences:**
- LevelDB is LSM-based (append-only, periodic compaction).
- Tosumu uses in-place B+ tree updates with WAL.
- If LSM mode is ever added, this is the reference.

---

### RustCrypto crates documentation
**URL:** https://docs.rs/chacha20poly1305/, https://docs.rs/argon2/

**What to reference:**
- AEAD API patterns (encrypt_in_place, decrypt_in_place).
- AAD construction best practices.
- Argon2id parameter selection (memory cost, iterations, parallelism).

**Critical for Stage 4.**

---

## How to use this document

1. **Before starting a stage**, read the DESIGN.md section for that stage.
2. **Before implementing a module** (e.g., `btree.rs`), review the corresponding reference from this document.
3. **Copy algorithms, not code.** These references are in C# or C; tosumu is Rust. Understand the algorithm, then write idiomatic Rust.
4. **Test everything.** Even proven algorithms can have bugs when adapted to a new context (page-based storage, encryption, etc.).

This document is a living reference. Add new entries as useful implementations are discovered.
