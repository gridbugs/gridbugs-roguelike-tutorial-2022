#!/bin/sh
git fetch && git checkout origin/main && git branch -f main HEAD && git checkout main
