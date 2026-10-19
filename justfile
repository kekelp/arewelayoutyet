# Fetch the published benchmark history for local website builds.
[positional-arguments]
fetch-data repository="":
    #!/usr/bin/env bash
    set -euo pipefail
    export GITHUB_REPOSITORY="${1:-${GITHUB_REPOSITORY:-}}"
    if [ -z "$GITHUB_REPOSITORY" ]; then
        GITHUB_REPOSITORY="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
    fi
    node scripts/history.mjs download

# Build and serve the website locally.
[positional-arguments]
serve port="8000":
    #!/usr/bin/env bash
    set -euo pipefail
    node scripts/build.mjs
    exec node scripts/serve.mjs "$1"
