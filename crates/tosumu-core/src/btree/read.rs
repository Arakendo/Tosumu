use std::collections::{BTreeMap, HashSet};

use crate::error::{Result, TosumuError};
use crate::format::{
    read_u64, MAX_VALUE_SIZE, OVERFLOW_PAYLOAD_SIZE, PAGE_HEADER_SIZE, PAGE_PLAINTEXT_SIZE,
    PAGE_TYPE_INTERNAL, PAGE_TYPE_LEAF, PAGE_TYPE_OVERFLOW,
};
use crate::pager::Pager;
use crate::snapshot_registry::SnapshotPin;

use super::{
    internal_find_child, leaf_get, leaf_read_all_refs, BTree, LeafValue, HDR_LEFTMOST,
    HDR_PAGE_TYPE,
};

trait ReadSource {
    fn root_page(&self) -> u64;
    fn page_count(&self) -> u64;
    fn with_page<F, T>(&self, pgno: u64, f: F) -> Result<T>
    where
        F: FnOnce(&[u8; PAGE_PLAINTEXT_SIZE]) -> Result<T>;
}

struct CurrentReadSource<'a> {
    pager: &'a Pager,
}

impl ReadSource for CurrentReadSource<'_> {
    fn root_page(&self) -> u64 {
        self.pager.root_page()
    }

    fn page_count(&self) -> u64 {
        self.pager.page_count()
    }

    fn with_page<F, T>(&self, pgno: u64, f: F) -> Result<T>
    where
        F: FnOnce(&[u8; PAGE_PLAINTEXT_SIZE]) -> Result<T>,
    {
        self.pager.with_page(pgno, f)
    }
}

#[cfg_attr(not(test), allow(dead_code))]
struct SnapshotReadSource<'a> {
    pager: &'a Pager,
    pin: &'a SnapshotPin,
    root_page: u64,
    page_count: u64,
}

#[cfg_attr(not(test), allow(dead_code))]
impl SnapshotReadSource<'_> {
    fn new<'a>(pager: &'a Pager, pin: &'a SnapshotPin) -> Result<SnapshotReadSource<'a>> {
        let (root_page, page_count) = pager.snapshot_metadata(pin)?;
        Ok(SnapshotReadSource {
            pager,
            pin,
            root_page,
            page_count,
        })
    }
}

impl ReadSource for SnapshotReadSource<'_> {
    fn root_page(&self) -> u64 {
        self.root_page
    }

    fn page_count(&self) -> u64 {
        self.page_count
    }

    fn with_page<F, T>(&self, pgno: u64, f: F) -> Result<T>
    where
        F: FnOnce(&[u8; PAGE_PLAINTEXT_SIZE]) -> Result<T>,
    {
        self.pager.with_snapshot_page(self.pin, pgno, f)
    }
}

impl BTree {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn pin_snapshot(&self) -> Result<SnapshotPin> {
        self.pager.pin_latest_snapshot()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn get_at_snapshot(&self, pin: &SnapshotPin, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let source = SnapshotReadSource::new(&self.pager, pin)?;
        get_from(&source, key)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn scan_at_snapshot(
        &self,
        pin: &SnapshotPin,
        start: &[u8],
        end: &[u8],
    ) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let source = SnapshotReadSource::new(&self.pager, pin)?;
        scan_from(&source, start, end)
    }
}

pub(super) fn get(pager: &Pager, key: &[u8]) -> Result<Option<Vec<u8>>> {
    get_from(&CurrentReadSource { pager }, key)
}

pub(super) fn find_leaf(pager: &Pager, key: &[u8]) -> Result<u64> {
    find_leaf_from(&CurrentReadSource { pager }, key)
}

pub(super) fn scan(pager: &Pager, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    scan_from(&CurrentReadSource { pager }, start, end)
}

pub(super) fn read_overflow_chain(pager: &Pager, head: u64, length: u64) -> Result<Vec<u8>> {
    read_overflow_chain_from(&CurrentReadSource { pager }, head, length)
}

fn get_from<R: ReadSource>(source: &R, key: &[u8]) -> Result<Option<Vec<u8>>> {
    let leaf_pgno = find_leaf_from(source, key)?;
    let value = source.with_page(leaf_pgno, |page| leaf_get(page, leaf_pgno, key))?;
    match value {
        Some(LeafValue::Inline(value)) => Ok(Some(value)),
        Some(LeafValue::Overflow { head, length }) => {
            Ok(Some(read_overflow_chain_from(source, head, length)?))
        }
        None => Ok(None),
    }
}

fn scan_from<R: ReadSource>(
    source: &R,
    start: &[u8],
    end: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let first_leaf = find_leaf_from(source, start)?;
    let mut results: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
    let mut cursor = first_leaf;

    loop {
        let (pairs, next) = source.with_page(cursor, |page| {
            Ok((leaf_read_all_refs(page), read_u64(page, HDR_LEFTMOST)))
        })?;
        let mut past_end = false;
        for (key, value) in pairs {
            if key.as_slice() > end {
                past_end = true;
                break;
            }
            if key.as_slice() >= start {
                let value = match value {
                    LeafValue::Inline(value) => value,
                    LeafValue::Overflow { head, length } => {
                        read_overflow_chain_from(source, head, length)?
                    }
                };
                results.insert(key, value);
            }
        }
        if next == 0 || past_end {
            break;
        }
        cursor = next;
    }

    Ok(results.into_iter().collect())
}

fn find_leaf_from<R: ReadSource>(source: &R, key: &[u8]) -> Result<u64> {
    const MAX_DEPTH: usize = 64;
    let mut pgno = source.root_page();
    let mut depth = 0usize;
    loop {
        depth += 1;
        if depth > MAX_DEPTH {
            return Err(TosumuError::Corrupt {
                pgno,
                reason: "traversal depth exceeds maximum (cycle suspected)",
            });
        }
        if pgno == 0 || pgno >= source.page_count() {
            return Err(TosumuError::Corrupt {
                pgno,
                reason: "traversal reached out-of-range page number",
            });
        }
        let page_type = source.with_page(pgno, |page| Ok(page[HDR_PAGE_TYPE]))?;
        match page_type {
            PAGE_TYPE_LEAF => return Ok(pgno),
            PAGE_TYPE_INTERNAL => {
                pgno = source.with_page(pgno, |page| internal_find_child(page, pgno, key))?;
            }
            _ => {
                return Err(TosumuError::Corrupt {
                    pgno,
                    reason: "unexpected page type during traversal",
                });
            }
        }
    }
}

fn read_overflow_chain_from<R: ReadSource>(source: &R, head: u64, length: u64) -> Result<Vec<u8>> {
    let length = usize::try_from(length).map_err(|_| TosumuError::OverflowChainCorrupt {
        pgno: head,
        length,
        reason: "overflow logical length does not fit usize",
    })?;
    if length > MAX_VALUE_SIZE {
        return Err(TosumuError::OverflowChainCorrupt {
            pgno: head,
            length: length as u64,
            reason: "overflow logical length exceeds maximum",
        });
    }

    let expected_pages = length.div_ceil(OVERFLOW_PAYLOAD_SIZE);
    let mut value = Vec::with_capacity(length);
    let mut current = head;
    let mut seen = HashSet::new();
    for _ in 0..expected_pages {
        if current == 0 || !seen.insert(current) {
            return Err(TosumuError::OverflowChainCorrupt {
                pgno: current,
                length: length as u64,
                reason: "overflow chain is missing or cyclic",
            });
        }
        let (next, payload) = source.with_page(current, |page| {
            if page[HDR_PAGE_TYPE] != PAGE_TYPE_OVERFLOW {
                return Err(TosumuError::OverflowChainCorrupt {
                    pgno: current,
                    length: length as u64,
                    reason: "overflow chain references a non-overflow page",
                });
            }
            let remaining = length - value.len();
            let count = remaining.min(OVERFLOW_PAYLOAD_SIZE);
            Ok((
                read_u64(page, HDR_LEFTMOST),
                page[PAGE_HEADER_SIZE..PAGE_HEADER_SIZE + count].to_vec(),
            ))
        })?;
        value.extend_from_slice(&payload);
        current = next;
    }

    if value.len() != length || current != 0 {
        return Err(TosumuError::OverflowChainCorrupt {
            pgno: current,
            length: length as u64,
            reason: "overflow chain length or termination mismatch",
        });
    }
    Ok(value)
}
