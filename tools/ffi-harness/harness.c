#include "tosumu_experimental_v1.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void require_success(tosumu_experimental_v1_outcome outcome) {
    assert(outcome.tag == TOSUMU_EXPERIMENTAL_V1_SUCCESS);
}

static void require_bytes(uint64_t handle, const uint8_t *expected, size_t expected_length) {
    tosumu_experimental_v1_outcome length = tosumu_experimental_v1_bytes_length(handle);
    require_success(length);
    assert(length.payload == expected_length);

    uint8_t *buffer = expected_length == 0 ? NULL : malloc(expected_length);
    assert(expected_length == 0 || buffer != NULL);
    tosumu_experimental_v1_outcome copied =
        tosumu_experimental_v1_bytes_copy(handle, buffer, expected_length);
    require_success(copied);
    assert(copied.payload == expected_length);
    assert(expected_length == 0 || memcmp(buffer, expected, expected_length) == 0);
    free(buffer);
    require_success(tosumu_experimental_v1_bytes_close(handle));
}

static void require_nonempty_bytes(uint64_t handle) {
    tosumu_experimental_v1_outcome length = tosumu_experimental_v1_bytes_length(handle);
    require_success(length);
    assert(length.payload > 0);
    uint8_t *buffer = malloc((size_t)length.payload);
    assert(buffer != NULL);
    require_success(tosumu_experimental_v1_bytes_copy(
        handle, buffer, (size_t)length.payload));
    free(buffer);
    require_success(tosumu_experimental_v1_bytes_close(handle));
}

int main(int argc, char **argv) {
    assert(argc == 3);
    assert(sizeof(tosumu_experimental_v1_outcome) == 16);
    assert(tosumu_experimental_v1_abi_version() == 1);

    const uint8_t key[] = {0x00, 'k', 0xff};
    const uint8_t first[] = {'f', 'i', 'r', 's', 't'};
    const uint8_t second[] = {'s', 'e', 'c', 'o', 'n', 'd'};
    const uint8_t range_upper[] = "page-12";
    const uint8_t large_key[] = "page-large";
    const size_t large_value_length = 20000;
    uint8_t *large_value = malloc(large_value_length);
    assert(large_value != NULL);
    memset(large_value, 0x5a, large_value_length);

    tosumu_experimental_v1_outcome database = tosumu_experimental_v1_database_create(
        (const uint8_t *)argv[1], strlen(argv[1]));
    require_success(database);
    require_success(tosumu_experimental_v1_database_put(
        database.payload, key, sizeof(key), first, sizeof(first)));
    for (int row = 0; row < 13; row++) {
        char row_key[16];
        char row_value[16];
        int key_length = snprintf(row_key, sizeof(row_key), "page-%02d", row);
        int value_length = snprintf(row_value, sizeof(row_value), "value-%02d", row);
        assert(key_length > 0 && (size_t)key_length < sizeof(row_key));
        assert(value_length > 0 && (size_t)value_length < sizeof(row_value));
        require_success(tosumu_experimental_v1_database_put(
            database.payload,
            (const uint8_t *)row_key,
            (size_t)key_length,
            (const uint8_t *)row_value,
            (size_t)value_length));
    }
    require_success(tosumu_experimental_v1_database_put(
        database.payload,
        large_key,
        sizeof(large_key) - 1,
        large_value,
        large_value_length));

    tosumu_experimental_v1_outcome snapshot =
        tosumu_experimental_v1_snapshot_begin(database.payload);
    require_success(snapshot);
    assert(tosumu_experimental_v1_snapshot_generation(snapshot.payload).payload > 0);

    require_success(tosumu_experimental_v1_database_put(
        database.payload, key, sizeof(key), second, sizeof(second)));
    tosumu_experimental_v1_outcome latest =
        tosumu_experimental_v1_database_get(database.payload, key, sizeof(key));
    tosumu_experimental_v1_outcome pinned =
        tosumu_experimental_v1_snapshot_get(snapshot.payload, key, sizeof(key));
    require_success(latest);
    require_success(pinned);
    tosumu_experimental_v1_outcome connection =
        tosumu_experimental_v1_database_connection_info(database.payload);
    require_success(connection);
    tosumu_experimental_v1_outcome active_readers =
        tosumu_experimental_v1_connection_field(
            connection.payload, TOSUMU_EXPERIMENTAL_V1_CONNECTION_ACTIVE_READERS);
    require_success(active_readers);
    assert(active_readers.payload == 1);
    require_success(tosumu_experimental_v1_database_close(database.payload));
    require_success(tosumu_experimental_v1_connection_close(connection.payload));
    require_bytes(latest.payload, second, sizeof(second));
    require_bytes(pinned.payload, first, sizeof(first));

    for (int page_index = 0; page_index < 4; page_index++) {
        int first_row = page_index * 4;
        int expected_count = first_row + 4 <= 13 ? 4 : 13 - first_row;
        char start[16];
        int start_length = snprintf(start, sizeof(start), "page-%02d", first_row);
        assert(start_length > 0 && (size_t)start_length < sizeof(start));
        tosumu_experimental_v1_outcome page =
            tosumu_experimental_v1_snapshot_scan_page(
                snapshot.payload,
                (const uint8_t *)start,
                (size_t)start_length,
                range_upper,
                sizeof(range_upper) - 1,
                4,
                1024);
        require_success(page);
        tosumu_experimental_v1_outcome pair_count =
            tosumu_experimental_v1_scan_page_pair_count(page.payload);
        require_success(pair_count);
        assert(pair_count.payload == (uint64_t)expected_count);
        assert(tosumu_experimental_v1_scan_page_blocked_entry_payload_bytes(
                   page.payload)
                   .tag == TOSUMU_EXPERIMENTAL_V1_ABSENT);
        for (int offset = 0; offset < expected_count; offset++) {
            int row = first_row + offset;
            char expected_key[16];
            char expected_value[16];
            int key_length =
                snprintf(expected_key, sizeof(expected_key), "page-%02d", row);
            int value_length =
                snprintf(expected_value, sizeof(expected_value), "value-%02d", row);
            tosumu_experimental_v1_outcome pair_key =
                tosumu_experimental_v1_scan_page_pair_key(
                    page.payload, (uint64_t)offset);
            tosumu_experimental_v1_outcome pair_value =
                tosumu_experimental_v1_scan_page_pair_value(
                    page.payload, (uint64_t)offset);
            require_success(pair_key);
            require_success(pair_value);
            require_bytes(pair_key.payload,
                          (const uint8_t *)expected_key,
                          (size_t)key_length);
            require_bytes(pair_value.payload,
                          (const uint8_t *)expected_value,
                          (size_t)value_length);
        }

        tosumu_experimental_v1_outcome next =
            tosumu_experimental_v1_scan_page_next_start(page.payload);
        require_success(tosumu_experimental_v1_scan_page_close(page.payload));
        if (first_row + expected_count < 13) {
            char expected_next[16];
            int next_length = snprintf(expected_next,
                                       sizeof(expected_next),
                                       "page-%02d",
                                       first_row + expected_count);
            require_success(next);
            require_bytes(next.payload,
                          (const uint8_t *)expected_next,
                          (size_t)next_length);
        } else {
            assert(next.tag == TOSUMU_EXPERIMENTAL_V1_ABSENT);
        }
    }

    tosumu_experimental_v1_outcome blocked =
        tosumu_experimental_v1_snapshot_scan_page(
            snapshot.payload,
            large_key,
            sizeof(large_key) - 1,
            large_key,
            sizeof(large_key) - 1,
            1,
            100);
    require_success(blocked);
    tosumu_experimental_v1_outcome blocked_count =
        tosumu_experimental_v1_scan_page_pair_count(blocked.payload);
    require_success(blocked_count);
    assert(blocked_count.payload == 0);
    tosumu_experimental_v1_outcome blocked_next =
        tosumu_experimental_v1_scan_page_next_start(blocked.payload);
    require_success(blocked_next);
    tosumu_experimental_v1_outcome required =
        tosumu_experimental_v1_scan_page_blocked_entry_payload_bytes(
            blocked.payload);
    require_success(required);
    assert(required.payload == (sizeof(large_key) - 1) + large_value_length);
    require_success(tosumu_experimental_v1_scan_page_close(blocked.payload));
    require_bytes(blocked_next.payload, large_key, sizeof(large_key) - 1);

    tosumu_experimental_v1_outcome admitted =
        tosumu_experimental_v1_snapshot_scan_page(
            snapshot.payload,
            large_key,
            sizeof(large_key) - 1,
            large_key,
            sizeof(large_key) - 1,
            1,
            required.payload);
    require_success(admitted);
    tosumu_experimental_v1_outcome admitted_count =
        tosumu_experimental_v1_scan_page_pair_count(admitted.payload);
    require_success(admitted_count);
    assert(admitted_count.payload == 1);
    tosumu_experimental_v1_outcome admitted_key =
        tosumu_experimental_v1_scan_page_pair_key(admitted.payload, 0);
    tosumu_experimental_v1_outcome admitted_value =
        tosumu_experimental_v1_scan_page_pair_value(admitted.payload, 0);
    require_success(admitted_key);
    require_success(admitted_value);
    assert(tosumu_experimental_v1_scan_page_next_start(admitted.payload).tag ==
           TOSUMU_EXPERIMENTAL_V1_ABSENT);
    assert(tosumu_experimental_v1_scan_page_blocked_entry_payload_bytes(
               admitted.payload)
               .tag == TOSUMU_EXPERIMENTAL_V1_ABSENT);
    require_success(tosumu_experimental_v1_scan_page_close(admitted.payload));
    require_bytes(admitted_key.payload, large_key, sizeof(large_key) - 1);
    require_bytes(admitted_value.payload, large_value, large_value_length);
    assert(tosumu_experimental_v1_scan_page_pair_count(snapshot.payload).status ==
           TOSUMU_EXPERIMENTAL_V1_BOUNDARY_WRONG_KIND);
    free(large_value);

    tosumu_experimental_v1_outcome after_close =
        tosumu_experimental_v1_snapshot_get(snapshot.payload, key, sizeof(key));
    require_success(after_close);
    require_bytes(after_close.payload, first, sizeof(first));
    require_success(tosumu_experimental_v1_snapshot_close(snapshot.payload));
    assert(tosumu_experimental_v1_snapshot_close(snapshot.payload).tag ==
           TOSUMU_EXPERIMENTAL_V1_BOUNDARY_FAILURE);

    tosumu_experimental_v1_outcome reopened = tosumu_experimental_v1_database_open(
        (const uint8_t *)argv[1], strlen(argv[1]));
    require_success(reopened);
    tosumu_experimental_v1_outcome panic_result =
        tosumu_experimental_v1_test_inject_database_panic(reopened.payload);
    assert(panic_result.tag == TOSUMU_EXPERIMENTAL_V1_BOUNDARY_FAILURE);
    assert(panic_result.status == TOSUMU_EXPERIMENTAL_V1_BOUNDARY_PANIC);
    tosumu_experimental_v1_outcome poisoned =
        tosumu_experimental_v1_database_get(reopened.payload, key, sizeof(key));
    assert(poisoned.tag == TOSUMU_EXPERIMENTAL_V1_BOUNDARY_FAILURE);
    assert(poisoned.status == TOSUMU_EXPERIMENTAL_V1_BOUNDARY_POISONED);
    require_success(tosumu_experimental_v1_database_close(reopened.payload));

    tosumu_experimental_v1_outcome missing = tosumu_experimental_v1_database_open(
        (const uint8_t *)argv[2], strlen(argv[2]));
    assert(missing.tag == TOSUMU_EXPERIMENTAL_V1_ERROR);
    tosumu_experimental_v1_outcome code =
        tosumu_experimental_v1_error_code(missing.payload);
    require_success(code);
    assert(tosumu_experimental_v1_error_status(missing.payload).payload ==
           TOSUMU_EXPERIMENTAL_V1_ERROR_EXTERNAL_FAILURE);
    require_bytes(code.payload, (const uint8_t *)"FILE_IO_FAILED", 14);
    tosumu_experimental_v1_outcome message =
        tosumu_experimental_v1_error_message(missing.payload);
    require_success(message);
    require_nonempty_bytes(message.payload);
    assert(tosumu_experimental_v1_error_detail_count(missing.payload).payload == 1);
    tosumu_experimental_v1_outcome detail_key =
        tosumu_experimental_v1_error_detail_key(missing.payload, 0);
    require_success(detail_key);
    require_bytes(detail_key.payload, (const uint8_t *)"source", 6);
    assert(tosumu_experimental_v1_error_detail_type(missing.payload, 0).payload ==
           TOSUMU_EXPERIMENTAL_V1_DETAIL_STRING);
    tosumu_experimental_v1_outcome detail_value =
        tosumu_experimental_v1_error_detail_string(missing.payload, 0);
    require_success(detail_value);
    require_nonempty_bytes(detail_value.payload);
    assert(tosumu_experimental_v1_bytes_length(missing.payload).status ==
           TOSUMU_EXPERIMENTAL_V1_BOUNDARY_WRONG_KIND);
    require_success(tosumu_experimental_v1_error_close(missing.payload));

    assert(tosumu_experimental_v1_database_open(NULL, 1).status ==
           TOSUMU_EXPERIMENTAL_V1_BOUNDARY_INVALID_POINTER);

    puts("independent C ABI harness: ok");
    return 0;
}
