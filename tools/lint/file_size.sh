#!/usr/bin/env bash
# File-size enforcement. See PLAN.md Section 17.
#
# Rules:
#   - 400-line soft cap requires no annotation.
#   - 401..=800 lines require a `// LINT-ALLOW: long-file <reason>` line.
#   - >800 lines is a hard cap with no exception.
#
# Generated files, golden data, lockfiles, and vendored third-party files
# are exempt and listed in EXCLUDE_RE below.

set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

EXCLUDE_RE='^(target/|assets/baked/|tests/golden/|tests/golden_dev/|tests/synthetic_charts/|.*/Cargo\.lock|.*\.lock|vendor/|crates/.*/vendor/)'

soft_cap=400
hard_cap=800
status=0

# Find all *.rs files tracked-or-not, excluding target/etc.
while IFS= read -r -d '' file; do
    rel="${file#./}"
    if [[ "$rel" =~ $EXCLUDE_RE ]]; then continue; fi
    lines=$(wc -l <"$file" | tr -d ' ')
    if (( lines > hard_cap )); then
        echo "HARD CAP: $rel has $lines lines (>$hard_cap)" >&2
        status=1
    elif (( lines > soft_cap )); then
        if ! grep -q 'LINT-ALLOW: long-file' "$file"; then
            echo "SOFT CAP: $rel has $lines lines (>$soft_cap) without LINT-ALLOW: long-file <reason>" >&2
            status=1
        fi
    fi
done < <(find . -type f -name '*.rs' -not -path './target/*' -print0)

exit $status
