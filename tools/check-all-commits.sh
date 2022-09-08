#!/usr/bin/env bash
set -euo pipefail

git log --pretty="%h %s" | tac | tail -n +2 | while read line; do
    HASH=$(echo $line | cut -d' ' -f1)
    git checkout $HASH
    cargo check
done

