#!/usr/bin/env bash
# Aggregate lint runner. See PLAN.md Section 17.
#
# Composes:
#   - cargo fmt --check
#   - cargo clippy with -D warnings
#   - file-size cap (tools/lint/file_size.sh)
#   - asset I/O whitelist
#   - backend API whitelist
#   - `// ref:` comment requirement on changed gameplay files
#
# Each check is independent. The script reports every failure before exiting
# non-zero so a CI run surfaces all problems in one pass.

set -u
status=0

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

run() {
    local label="$1"
    shift
    echo "==> $label"
    if "$@"; then
        echo "    ok"
    else
        echo "    FAIL: $label" >&2
        status=1
    fi
}

run "cargo fmt --check"          cargo fmt --all -- --check
run "cargo clippy"               cargo clippy --workspace --all-targets -- -D warnings
run "file-size cap"              tools/lint/file_size.sh
run "asset I/O whitelist"        tools/lint/asset_io.sh
run "backend API whitelist"      tools/lint/backend_api.sh
run "ref: gameplay comments"     tools/lint/ref_comments.sh

if [[ $status -ne 0 ]]; then
    echo "lint: failures above" >&2
fi
exit $status
