#!/usr/bin/env bash
set -euo pipefail

test "$(uname -s)" = "Linux"
rustc --version --verbose
cargo --version
cc --version
ld --version

cargo build -p tosumu-ffi-experimental --features ffi-test-hooks
mkdir -p target/ffi-harness
cc -std=c11 -Wall -Wextra -Werror -DTOSUMU_EXPERIMENTAL_V1_TEST_HOOKS \
  tools/ffi-harness/harness.c \
  -Itools/ffi-harness \
  -Ltarget/debug -ltosumu_ffi_experimental \
  '-Wl,-rpath,$ORIGIN/../debug' \
  -o target/ffi-harness/tosumu-ffi-harness
cc -std=c11 -Wall -Wextra -Werror -DTOSUMU_EXPERIMENTAL_V1_TEST_HOOKS \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  tools/ffi-harness/harness.c \
  -Itools/ffi-harness \
  -Ltarget/debug -ltosumu_ffi_experimental \
  '-Wl,-rpath,$ORIGIN/../debug' \
  -o target/ffi-harness/tosumu-ffi-harness-sanitized

nm -D --defined-only target/debug/libtosumu_ffi_experimental.so \
  | awk '{print $3}' \
  | grep '^tosumu_experimental_v1_' \
  | sort > target/ffi-harness/observed-symbols.txt
diff -u tools/ffi-harness/symbols.txt target/ffi-harness/observed-symbols.txt

clean_artifacts() {
  rm -f \
    target/ffi-harness/database.tsm \
    target/ffi-harness/database.tsm.wal \
    target/ffi-harness/database.tsm.writer.lock \
    target/ffi-harness/missing.tsm \
    target/ffi-harness/missing.tsm.wal \
    target/ffi-harness/missing.tsm.writer.lock
}

clean_artifacts
target/ffi-harness/tosumu-ffi-harness \
  target/ffi-harness/database.tsm \
  target/ffi-harness/missing.tsm

clean_artifacts
ASAN_OPTIONS=detect_leaks=1:halt_on_error=1 \
UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1 \
target/ffi-harness/tosumu-ffi-harness-sanitized \
  target/ffi-harness/database.tsm \
  target/ffi-harness/missing.tsm
