#!/usr/bin/env bash

status=0
runner_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

for workers in 12 10 8 6 4 2 1; do
    "$runner_dir/run.py" all -w "$workers" || status=$?
done
exit "$status"
