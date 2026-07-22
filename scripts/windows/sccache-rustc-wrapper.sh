#!/bin/sh
# Cargo may set CARGO_INCREMENTAL=0 in the rustc-wrapper environment; sccache
# rejects the variable when present at all (even when set to 0).
unset CARGO_INCREMENTAL CARGO_BUILD_INCREMENTAL
exec /usr/local/bin/sccache "$@"
