#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"
cp ./target/debug/lasttree .
./lasttree
# ./target/debug/lasttree
