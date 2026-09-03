#!/usr/bin/env bash
set -euo pipefail

test "$(uname -s)" = "Linux"
rustc --version --verbose
cargo --version
cc --version
ld --version

cargo build -p tosumu-ffi-experimental
mkdir -p target/ffi-harness
cc -std=c11 -Wall -Wextra -Werror \
  tools/ffi-harness/harness.c \
  -Itools/ffi-harness \
  -Ltarget/debug -ltosumu_ffi_experimental \
  '-Wl,-rpath,$ORIGIN/../debug' \
  -o target/ffi-harness/tosumu-ffi-harness

nm -D --defined-only target/debug/libtosumu_ffi_experimental.so \
  | awk '{print $3}' \
  | grep '^tosumu_experimental_v1_' \
  | sort > target/ffi-harness/observed-symbols.txt
diff -u tools/ffi-harness/symbols.txt target/ffi-harness/observed-symbols.txt

rm -f \
  target/ffi-harness/database.tsm \
  target/ffi-harness/database.tsm.wal \
  target/ffi-harness/database.tsm.writer.lock \
  target/ffi-harness/missing.tsm \
  target/ffi-harness/missing.tsm.wal \
  target/ffi-harness/missing.tsm.writer.lock
target/ffi-harness/tosumu-ffi-harness \
  target/ffi-harness/database.tsm \
  target/ffi-harness/missing.tsm
