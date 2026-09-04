#include "tosumu_experimental_v1.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int main(int argc, char **argv) {
    if (tosumu_experimental_v1_abi_version() != 1U) {
        return 2;
    }

    char *allocated_path = NULL;
    const char *path = argc == 2 ? argv[1] : NULL;
    if (path == NULL) {
        const char *home = getenv("HOME");
        if (home == NULL) {
            return 3;
        }

        static const char suffix[] = "/Documents/tosumu-loader-ok";
        size_t path_length = strlen(home) + sizeof(suffix);
        allocated_path = malloc(path_length);
        if (allocated_path == NULL) {
            return 4;
        }
        if (snprintf(allocated_path, path_length, "%s%s", home, suffix) < 0) {
            free(allocated_path);
            return 5;
        }
        path = allocated_path;
    }

    FILE *marker = fopen(path, "wb");
    free(allocated_path);
    if (marker == NULL) {
        return 6;
    }
    static const char evidence[] = "tosumu-experimental-v1=1\n";
    size_t written = fwrite(evidence, 1, sizeof(evidence) - 1U, marker);
    int close_result = fclose(marker);
    return written == sizeof(evidence) - 1U && close_result == 0 ? 0 : 7;
}
