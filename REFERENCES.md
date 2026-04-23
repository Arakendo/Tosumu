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
