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

int main(int argc, char **argv) {
    assert(argc == 3);
    assert(sizeof(tosumu_experimental_v1_outcome) == 16);
    assert(tosumu_experimental_v1_abi_version() == 1);

    const uint8_t key[] = {0x00, 'k', 0xff};
    const uint8_t first[] = {'f', 'i', 'r', 's', 't'};
    const uint8_t second[] = {'s', 'e', 'c', 'o', 'n', 'd'};

    tosumu_experimental_v1_outcome database = tosumu_experimental_v1_database_create(
        (const uint8_t *)argv[1], strlen(argv[1]));
    require_success(database);
    require_success(tosumu_experimental_v1_database_put(
        database.payload, key, sizeof(key), first, sizeof(first)));

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
    require_success(tosumu_experimental_v1_database_close(database.payload));
    require_bytes(latest.payload, second, sizeof(second));
    require_bytes(pinned.payload, first, sizeof(first));

    tosumu_experimental_v1_outcome after_close =
        tosumu_experimental_v1_snapshot_get(snapshot.payload, key, sizeof(key));
    require_success(after_close);
    require_bytes(after_close.payload, first, sizeof(first));
    require_success(tosumu_experimental_v1_snapshot_close(snapshot.payload));
    assert(tosumu_experimental_v1_snapshot_close(snapshot.payload).tag ==
           TOSUMU_EXPERIMENTAL_V1_BOUNDARY_FAILURE);

    tosumu_experimental_v1_outcome missing = tosumu_experimental_v1_database_open(
        (const uint8_t *)argv[2], strlen(argv[2]));
    assert(missing.tag == TOSUMU_EXPERIMENTAL_V1_ERROR);
    tosumu_experimental_v1_outcome code =
        tosumu_experimental_v1_error_code(missing.payload);
    require_success(code);
    assert(tosumu_experimental_v1_error_status(missing.payload).payload != 0);
    require_bytes(code.payload, (const uint8_t *)"FILE_IO_FAILED", 14);
    require_success(tosumu_experimental_v1_error_close(missing.payload));

    puts("independent C ABI harness: ok");
    return 0;
}
