//! Provider-neutral bounded range results.

/// One bounded page from an inclusive, generation-pinned key range.
///
/// `pairs` contains the admitted logical payload in raw-key order. When
/// `next_start_inclusive` is present, pass it unchanged as the next call's
/// inclusive lower bound on the same read transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "a scan page may contain a continuation that must be handled"]
pub struct KvScanPage {
    /// Ordered key/value pairs admitted by both limits.
    pub pairs: Vec<(Vec<u8>, Vec<u8>)>,
    /// First unconsumed key, or `None` when the requested range is exhausted.
    pub next_start_inclusive: Option<Vec<u8>>,
    /// Full logical size of the first unconsumed entry when the byte limit,
    /// rather than the pair limit, prevented its admission.
    ///
    /// Logical size is `key.len() + value.len()`. The excluded value is not
    /// materialized merely to report this size.
    pub blocked_entry_payload_bytes: Option<u64>,
}
