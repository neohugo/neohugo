#!/bin/sh
# Minimal goreleaser post-release hook for linux builds.
# Intentionally simple: log and exit success so goreleaser can continue.
echo "Running release-hook-post-linux.sh"
echo "Args: $*"
# Add real post-build steps here if desired (e.g., strip, sign, upload).
exit 0
