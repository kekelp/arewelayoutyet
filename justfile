# Fetch the published benchmark history for local website builds.
[positional-arguments]
fetch-data repository="":
    #!/usr/bin/env bash
    set -euo pipefail
    export GITHUB_REPOSITORY="${1:-${GITHUB_REPOSITORY:-}}"
    if [ -z "$GITHUB_REPOSITORY" ]; then
        GITHUB_REPOSITORY="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
    fi
    python3 scripts/history.py download
