#!/bin/bash

set -eu -o pipefail

echo
echo "-----------------"
date
git log --oneline | head -n 1 || true

cargo run --bin enumerate --release
hyperfine ./target/release/enumerate
