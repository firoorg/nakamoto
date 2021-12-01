#!/bin/sh

set -e

MSRV="1.29.0"

CMD="rustup run ${MSRV}"

rm -f Cargo.lock
$CMD cargo generate-lockfile
$CMD cargo update --package "cc" --precise "1.0.41"

$CMD cargo test
