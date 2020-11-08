#!/usr/bin/env bash

# allow user to override go executable by running as GOEXE=xxx make ...
GOEXE="${GOEXE-go}"

# Convenience script to
# - For a given branch
# - Run  benchmark tests for a given package
# - Do the same for main
# - then compare the two runs with benchcmp

if (( $# < 1 ));
  then
    echo "USAGE: ./bench.sh <git-branch> <package-to-bench> (and <benchmark filter> (regexp, optional))"
    exit 1
fi

if [ $# -eq 1 ]; then
  PACKAGE=./...
  PACKAGEFILE="all"
else
  PACKAGE=./"$2"
  PACKAGEFILE="$(echo "$2" | sed 's/[\/\.]/-/g')"
fi


if [ $# -eq 3 ]; then
  benchFilter=$3
else
  benchFilter="."
fi

BRANCH="$1"
BRANCHFILE="$(echo "$1" | sed 's/[\/\.]/-/g')"

git checkout "$BRANCH"
"${GOEXE}" test -test.run=NONE -bench="$benchFilter" -test.benchmem=true "$PACKAGE" > "/tmp/bench-$PACKAGEFILE-$BRANCHFILE.txt"

git checkout main
"${GOEXE}" test -test.run=NONE -bench="$benchFilter" -test.benchmem=true "$PACKAGE" > "/tmp/bench-$PACKAGEFILE-main.txt"

benchstat -split xxx "/tmp/bench-$PACKAGEFILE-main.txt" "/tmp/bench-$PACKAGEFILE-$BRANCHFILE.txt"
