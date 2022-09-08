#!/usr/bin/env bash
set -euo pipefail

N=5

git log --pretty="%h %s" | while read line; do
    HASH=$(echo $line | cut -d' ' -f1)
    BRANCH=part-$(echo $line | cut -d' ' -f2 | tr -d :)
    git branch --force $BRANCH $HASH
    git push origin $BRANCH --force
done

make_start_branch() {
    n=$1
    git branch --force part-$n.0 part-$n.1~1
    git push origin part-$n.0 --force
}

for i in $(seq 1 $N); do
    make_start_branch $i
done
