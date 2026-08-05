//! Versioned inspection data transfer objects for non-CLI Tosumu consumers.
//!
//! This crate owns a deliberately small wire boundary. Tosumu core continues
//! to own inspection facts; the CLI continues to own its command JSON. A future
//! WASM adapter may expose this boundary without making JSON a core dependency.

use serde::Serialize;
use tosumu_core::error::ErrorReport;
use tosumu_core::inspection_session::{
    inspect_observation_from_bytes, InspectionIssue, InspectionObservation, InspectionSection,
    DEFAULT_INSPECTION_BYTE_INPUT_LIMIT,
};

/// First supported version of this provider-side boundary.
pub const INSPECTION_BOUNDARY_SCHEMA_V1: u16 = 1;
/// Boundary errors retain a short explanation without echoing arbitrary input.
pub const MAX_BOUNDARY_MESSAGE_LENGTH: usize = 256;

/// Bounded raw input supplied by a host such as a browser file picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionBytesRequest<'a> {
    pub schema_version: u16,
    pub bytes: &'a [u8],
    pub byte_limit: usize,
}

impl<'a> InspectionBytesRequest<'a> {
    pub fn with_default_limit(bytes: &'a [u8]) -> Self {
        Self {
            schema_version: INSPECTION_BOUNDARY_SCHEMA_V1,
            bytes,
            byte_limit: DEFAULT_INSPECTION_BYTE_INPUT_LIMIT,
        }
    }
}

/// A boundary result is data in both success and failure cases so hosts do not
/// need to parse errors or infer unsupported storage capabilities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum InspectionBytesResponse {
    Observation {
        schema_version: u16,
        observation: Box<RawBytesObservation>,
    },
    Failure {
        schema_version: u16,
        error: BoundaryError,
    },
}

/// Inspects bytes without manufacturing a path, unlock context, or pager.
pub fn inspect_bytes(request: InspectionBytesRequest<'_>) -> InspectionBytesResponse {
    if request.schema_version != INSPECTION_BOUNDARY_SCHEMA_V1 {
        return failure(
            "INSPECTION_BOUNDARY_SCHEMA_UNSUPPORTED",
            "unsupported inspection boundary schema version",
            "unsupported",
        );
    }

    match inspect_observation_from_bytes(request.bytes, request.byte_limit) {
        Ok(observation) => InspectionBytesResponse::Observation {
            schema_version: INSPECTION_BOUNDARY_SCHEMA_V1,
            observation: Box::new(RawBytesObservation::from_observation(
                &observation,
                request.bytes.len(),
            )),
        },
        Err(error) => error_response(&error.error_report()),
    }
}

/// Serializes a response only at the consumer boundary.
pub fn inspect_bytes_json(request: InspectionBytesRequest<'_>) -> serde_json::Result<String> {
    serde_json::to_string(&inspect_bytes(request))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoundaryError {
    pub code: String,
    pub status: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawBytesObservation {
    pub observation_schema_version: u16,
    pub source: RawBytesSource,
    pub header: RawBytesHeader,
    pub verification: RawBytesVerification,
    pub pages: RawBytesPageList,
    pub tree: RawBytesSection,
    pub wal: RawBytesSection,
    pub keyslots: RawBytesSection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawBytesSource {
    pub kind: &'static str,
    pub byte_count: u64,
    pub unlock_state: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawBytesHeader {
    pub format_version: u16,
    pub page_size: u16,
    pub page_count: u64,
    pub root_page: u64,
    pub flags: u16,
    pub keyslot_count: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawBytesVerification {
    pub pages_checked: u64,
    pub pages_ok: u64,
    pub btree_checked: bool,
    pub btree_ok: bool,
    pub issues: Vec<RawBytesIssue>,
    pub issues_truncated: u64,
    pub btree_issue: Option<RawBytesIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawBytesIssue {
    pub code: String,
    pub message: String,
    pub page_number: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawBytesPageList {
    pub total: u64,
    pub retained: u64,
    pub truncated: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RawBytesSection {
    Unavailable {
        code: String,
        status: String,
        message: String,
    },
}

impl RawBytesObservation {
    fn from_observation(observation: &InspectionObservation, input_byte_count: usize) -> Self {
        Self {
            observation_schema_version: observation.schema_version,
            source: RawBytesSource {
                kind: "uploaded_bytes",
                byte_count: input_byte_count as u64,
                unlock_state: "header_only",
            },
            header: RawBytesHeader {
                format_version: observation.header.format_version,
                page_size: observation.header.page_size,
                page_count: observation.header.page_count,
                root_page: observation.header.root_page,
                flags: observation.header.flags,
                keyslot_count: observation.header.keyslot_count,
            },
            verification: RawBytesVerification {
                pages_checked: observation.verification.pages_checked,
                pages_ok: observation.verification.pages_ok,
                btree_checked: observation.verification.btree_checked,
                btree_ok: observation.verification.btree_ok,
                issues: observation
                    .verification
                    .issues
                    .iter()
                    .map(RawBytesIssue::from)
                    .collect(),
                issues_truncated: observation.verification.issues_truncated,
                btree_issue: observation
                    .verification
                    .btree_issue
                    .as_ref()
                    .map(RawBytesIssue::from),
            },
            pages: RawBytesPageList {
                total: observation.pages.total,
                retained: observation.pages.entries.len() as u64,
                truncated: observation.pages.truncated,
            },
            tree: unavailable_section(&observation.tree),
            wal: unavailable_section(&observation.wal),
            keyslots: unavailable_section(&observation.keyslots),
        }
    }
}

impl From<&InspectionIssue> for RawBytesIssue {
    fn from(issue: &InspectionIssue) -> Self {
        Self {
            code: issue.code.to_owned(),
            message: bounded_message(&issue.message),
            page_number: issue.page_number,
        }
    }
}

fn unavailable_section<T>(section: &InspectionSection<T>) -> RawBytesSection {
    match section {
        InspectionSection::Unavailable(unavailable) => RawBytesSection::Unavailable {
            code: unavailable.code.to_owned(),
            status: unavailable.status.as_str().to_owned(),
            message: bounded_message(&unavailable.message),
        },
        InspectionSection::Available(_) => RawBytesSection::Unavailable {
            code: "RAW_BYTES_SECTION_UNEXPECTED".to_owned(),
            status: "internal".to_owned(),
            message: "raw byte boundary received an unavailable-state violation".to_owned(),
        },
    }
}

fn error_response(report: &ErrorReport) -> InspectionBytesResponse {
    InspectionBytesResponse::Failure {
        schema_version: INSPECTION_BOUNDARY_SCHEMA_V1,
        error: BoundaryError {
            code: report.code.to_owned(),
            status: report.status.as_str().to_owned(),
            message: bounded_message(&report.message),
        },
    }
}

fn failure(
    code: &'static str,
    message: &'static str,
    status: &'static str,
) -> InspectionBytesResponse {
    InspectionBytesResponse::Failure {
        schema_version: INSPECTION_BOUNDARY_SCHEMA_V1,
        error: BoundaryError {
            code: code.to_owned(),
            status: status.to_owned(),
            message: message.to_owned(),
        },
    }
}

fn bounded_message(message: &str) -> String {
    if message.len() <= MAX_BOUNDARY_MESSAGE_LENGTH {
        return message.to_owned();
    }

    let mut end = MAX_BOUNDARY_MESSAGE_LENGTH;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &message[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tosumu_core::format::{
        MAGIC, OFF_FLAGS, OFF_FORMAT_VERSION, OFF_KEYSLOT_COUNT, OFF_PAGE_COUNT, OFF_PAGE_SIZE,
        OFF_ROOT_PAGE, PAGE_SIZE,
    };
    use tosumu_core::inspection_session::{
        inspect_observation_from_bytes, DEFAULT_INSPECTION_BYTE_INPUT_LIMIT,
    };

    fn fixture_bytes() -> Vec<u8> {
        let mut bytes = vec![0_u8; PAGE_SIZE];
        bytes[..MAGIC.len()].copy_from_slice(MAGIC);
        bytes[OFF_FORMAT_VERSION..OFF_FORMAT_VERSION + 2].copy_from_slice(&2_u16.to_le_bytes());
        bytes[OFF_PAGE_SIZE..OFF_PAGE_SIZE + 2].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
        bytes[OFF_FLAGS..OFF_FLAGS + 2].copy_from_slice(&0_u16.to_le_bytes());
        bytes[OFF_PAGE_COUNT..OFF_PAGE_COUNT + 8].copy_from_slice(&3_u64.to_le_bytes());
        bytes[OFF_ROOT_PAGE..OFF_ROOT_PAGE + 8].copy_from_slice(&1_u64.to_le_bytes());
        bytes[OFF_KEYSLOT_COUNT..OFF_KEYSLOT_COUNT + 2].copy_from_slice(&0_u16.to_le_bytes());
        bytes
    }

    fn reviewed_fixture(name: &str) -> &'static [u8] {
        match name {
            "fresh" => include_bytes!(
                "../../../docs/overrides/fixtures/inspection-header-fixture-v1.tosumu"
            ),
            "populated" => include_bytes!(
                "../../../docs/overrides/fixtures/inspection-populated-fixture-v1.tosumu"
            ),
            "invalid_magic" => {
                include_bytes!("../../../docs/overrides/fixtures/inspection-invalid-magic-v1.bin")
            }
            "truncated" => {
                include_bytes!("../../../docs/overrides/fixtures/inspection-truncated-v1.bin")
            }
            "newer_format" => {
                include_bytes!("../../../docs/overrides/fixtures/inspection-newer-format-v1.bin")
            }
            _ => panic!("unknown reviewed inspection fixture: {name}"),
        }
    }

    #[test]
    fn raw_bytes_response_preserves_header_and_unavailable_sections() {
        let bytes = fixture_bytes();
        let response = inspect_bytes(InspectionBytesRequest::with_default_limit(&bytes));
        let InspectionBytesResponse::Observation { observation, .. } = response else {
            panic!("fixture should produce an observation");
        };

        assert_eq!(observation.source.kind, "uploaded_bytes");
        assert_eq!(observation.source.byte_count, bytes.len() as u64);
        assert_eq!(observation.header.page_count, 3);
        assert_eq!(observation.pages.retained, 0);
        assert_eq!(observation.pages.truncated, 3);
        assert!(matches!(
            observation.tree,
            RawBytesSection::Unavailable { ref code, .. } if code == "RAW_BYTES_TREE_UNAVAILABLE"
        ));
    }

    #[test]
    fn boundary_preserves_core_raw_byte_observation_facts() {
        let bytes = fixture_bytes();
        let core =
            inspect_observation_from_bytes(&bytes, DEFAULT_INSPECTION_BYTE_INPUT_LIMIT).unwrap();
        let response = inspect_bytes(InspectionBytesRequest::with_default_limit(&bytes));
        let InspectionBytesResponse::Observation { observation, .. } = response else {
            panic!("fixture should produce an observation");
        };

        assert_eq!(observation.observation_schema_version, core.schema_version);
        assert_eq!(observation.source.byte_count, bytes.len() as u64);
        assert_eq!(
            observation.header.format_version,
            core.header.format_version
        );
        assert_eq!(observation.header.page_size, core.header.page_size);
        assert_eq!(observation.header.page_count, core.header.page_count);
        assert_eq!(observation.header.root_page, core.header.root_page);
        assert_eq!(observation.header.flags, core.header.flags);
        assert_eq!(observation.header.keyslot_count, core.header.keyslot_count);
        assert_eq!(observation.verification.pages_checked, 0);
        assert_eq!(observation.pages.total, core.pages.total);
        assert_eq!(observation.pages.retained, 0);
        assert_eq!(observation.pages.truncated, core.pages.truncated);
        assert!(matches!(
            observation.tree,
            RawBytesSection::Unavailable { ref code, .. } if code == "RAW_BYTES_TREE_UNAVAILABLE"
        ));
        assert!(matches!(
            observation.wal,
            RawBytesSection::Unavailable { ref code, .. } if code == "RAW_BYTES_WAL_UNAVAILABLE"
        ));
        assert!(matches!(
            observation.keyslots,
            RawBytesSection::Unavailable { ref code, .. } if code == "RAW_BYTES_KEYSLOTS_UNAVAILABLE"
        ));
    }

    #[test]
    fn boundary_rejects_unknown_schema_and_bounds_failures() {
        let bytes = fixture_bytes();
        let unsupported = inspect_bytes(InspectionBytesRequest {
            schema_version: 999,
            bytes: &bytes,
            byte_limit: DEFAULT_INSPECTION_BYTE_INPUT_LIMIT,
        });
        assert!(matches!(
            unsupported,
            InspectionBytesResponse::Failure { ref error, .. }
                if error.code == "INSPECTION_BOUNDARY_SCHEMA_UNSUPPORTED"
                    && error.status == "unsupported"
        ));

        let oversized = inspect_bytes(InspectionBytesRequest {
            schema_version: INSPECTION_BOUNDARY_SCHEMA_V1,
            bytes: &bytes,
            byte_limit: 1,
        });
        assert!(matches!(
            oversized,
            InspectionBytesResponse::Failure { ref error, .. }
                if error.status == "invalid_input"
        ));
    }

    #[test]
    fn serialized_boundary_never_contains_host_or_unlock_fields() {
        let bytes = fixture_bytes();
        let json = inspect_bytes_json(InspectionBytesRequest::with_default_limit(&bytes)).unwrap();

        assert!(!json.contains("\"path\""));
        assert!(!json.contains("passphrase"));
        assert!(!json.contains("decrypted"));
        assert!(!json.contains("\"pager\""));
    }

    #[test]
    fn reviewed_browser_fixture_matrix_has_explicit_bounded_outcomes() {
        for fixture in ["fresh", "populated"] {
            let response = inspect_bytes(InspectionBytesRequest::with_default_limit(
                reviewed_fixture(fixture),
            ));
            let InspectionBytesResponse::Observation { observation, .. } = response else {
                panic!("reviewed {fixture} fixture should produce a header observation");
            };

            assert_eq!(
                observation.pages.retained, 0,
                "{fixture} must not expose records"
            );
            assert!(matches!(
                observation.tree,
                RawBytesSection::Unavailable { ref code, .. } if code == "RAW_BYTES_TREE_UNAVAILABLE"
            ));
        }

        for (fixture, expected_code) in [
            (
                "invalid_magic",
                tosumu_core::error::codes::FORMAT_NOT_TOSUMU,
            ),
            ("truncated", tosumu_core::error::codes::FILE_TRUNCATED),
            (
                "newer_format",
                tosumu_core::error::codes::FORMAT_VERSION_UNSUPPORTED,
            ),
        ] {
            let response = inspect_bytes(InspectionBytesRequest::with_default_limit(
                reviewed_fixture(fixture),
            ));
            assert!(
                matches!(
                    response,
                    InspectionBytesResponse::Failure { ref error, .. } if error.code == expected_code
                ),
                "reviewed {fixture} fixture should produce {expected_code}"
            );
        }
    }
}
