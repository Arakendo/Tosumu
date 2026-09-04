#ifndef TOSUMU_EXPERIMENTAL_V1_H
#define TOSUMU_EXPERIMENTAL_V1_H

#include <stddef.h>
#include <stdint.h>

/* Private test contract. No source or binary compatibility is promised. */
typedef struct tosumu_experimental_v1_outcome {
    uint32_t tag;
    uint32_t status;
    uint64_t payload;
} tosumu_experimental_v1_outcome;

enum {
    TOSUMU_EXPERIMENTAL_V1_SUCCESS = 0,
    TOSUMU_EXPERIMENTAL_V1_ABSENT = 1,
    TOSUMU_EXPERIMENTAL_V1_ERROR = 2,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_FAILURE = 3
};

enum {
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_ACTIVE_READERS = 1,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_MAXIMUM_READERS = 2,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_OLDEST_READER_GENERATION = 3,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_CHECKPOINT_GENERATION = 4,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_LATEST_GENERATION = 5,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_RETAINED_WAL_BYTES = 6,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_RETAINED_FRAME_VERSIONS = 7,
    TOSUMU_EXPERIMENTAL_V1_CONNECTION_CHECKPOINT_BLOCKED = 8
};

enum {
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_INVALID_POINTER = 1,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_INVALID_UTF8 = 2,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_INVALID_HANDLE = 3,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_WRONG_KIND = 4,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_WRONG_THREAD = 5,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_PANIC = 6,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_POISONED = 7,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_REGISTRY_FULL = 8,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_INVALID_PATH = 9,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_INVALID_INDEX = 10,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_WRONG_DETAIL_TYPE = 11,
    TOSUMU_EXPERIMENTAL_V1_BOUNDARY_LIMIT_OUT_OF_RANGE = 12
};

enum {
    TOSUMU_EXPERIMENTAL_V1_ERROR_INVALID_INPUT = 1,
    TOSUMU_EXPERIMENTAL_V1_ERROR_NOT_FOUND = 2,
    TOSUMU_EXPERIMENTAL_V1_ERROR_CONFLICT = 3,
    TOSUMU_EXPERIMENTAL_V1_ERROR_PERMISSION_DENIED = 4,
    TOSUMU_EXPERIMENTAL_V1_ERROR_BUSY = 5,
    TOSUMU_EXPERIMENTAL_V1_ERROR_INTEGRITY_FAILURE = 6,
    TOSUMU_EXPERIMENTAL_V1_ERROR_EXTERNAL_FAILURE = 7,
    TOSUMU_EXPERIMENTAL_V1_ERROR_UNSUPPORTED = 8,
    TOSUMU_EXPERIMENTAL_V1_ERROR_INTERNAL = 9
};

enum {
    TOSUMU_EXPERIMENTAL_V1_DETAIL_BOOL = 1,
    TOSUMU_EXPERIMENTAL_V1_DETAIL_STRING = 2,
    TOSUMU_EXPERIMENTAL_V1_DETAIL_U16 = 3,
    TOSUMU_EXPERIMENTAL_V1_DETAIL_U64 = 4
};

uint32_t tosumu_experimental_v1_abi_version(void);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_create(const uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_open(const uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_close(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_connection_info(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_put(uint64_t, const uint8_t *, size_t, const uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_delete(uint64_t, const uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_database_get(uint64_t, const uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_snapshot_begin(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_snapshot_generation(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_snapshot_get(uint64_t, const uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_snapshot_scan_page(uint64_t, const uint8_t *, size_t, const uint8_t *, size_t, uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_snapshot_close(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_scan_page_pair_count(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_scan_page_pair_key(uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_scan_page_pair_value(uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_scan_page_next_start(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_scan_page_blocked_entry_payload_bytes(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_scan_page_close(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_bytes_length(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_bytes_copy(uint64_t, uint8_t *, size_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_bytes_close(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_connection_field(uint64_t, uint32_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_connection_close(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_code(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_status(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_message(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_detail_count(uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_detail_key(uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_detail_type(uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_detail_scalar(uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_detail_string(uint64_t, uint64_t);
tosumu_experimental_v1_outcome tosumu_experimental_v1_error_close(uint64_t);

#ifdef TOSUMU_EXPERIMENTAL_V1_TEST_HOOKS
tosumu_experimental_v1_outcome tosumu_experimental_v1_test_inject_database_panic(uint64_t);
#endif

#endif
