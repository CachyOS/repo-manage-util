#!/bin/bash
# Copyright (C) 2026 Vladislav Nepogodin
#
# This file is part of CachyOS repo-manage-util.
#
# This program is free software; you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation; either version 2 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along
# with this program; if not, write to the Free Software Foundation, Inc.,
# 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.

set -e

cd "`dirname "$0"`"

export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort -Cinstrument-coverage"
export RUSTDOCFLAGS="-Cpanic=abort"

cargo build
cargo test

grcov . -s . --binary-path ./target/debug/ -t html \
    --branch --ignore-not-existing --ignore build.rs \
    --excl-br-line "^\s*((debug_)?assert(_eq|_ne)?\#\[derive\()" \
    -o ./target/debug/coverage/
rm default_*.profraw
