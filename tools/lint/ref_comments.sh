#!/usr/bin/env bash
# `// ref:` comment requirement for ported gameplay files.
# See PLAN.md Section 17.
#
# Any file under crates/rustic-game/src/ that defines `pub fn` or `pub(crate)
# fn` and does not contain a `// ref:` comment is treated as a missing
# citation. Trivial accessor files can opt out with `// LINT-ALLOW: no-ref
# <reason>`.
#
# This is a starter check; replace with a tighter AST walk if it becomes
# noisy.

set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

violations=0
gameplay_root="crates/rustic-game/src"

if [[ ! -d "$gameplay_root" ]]; then
    exit 0
fi

while IFS= read -r -d '' file; do
    if grep -q 'LINT-ALLOW: no-ref' "$file"; then continue; fi
    if ! grep -qE '^\s*pub(\(crate\))?\s+fn\s' "$file"; then continue; fi
    # Accept `// ref:`, `/// ref:` (doc), `//! ref:` (module doc).
    if ! grep -qE '//[!/]? ref:' "$file"; then
        echo "REF-COMMENT: $file defines public gameplay fn(s) but contains no '// ref:' citation" >&2
        violations=$((violations + 1))
    fi
done < <(find "$gameplay_root" -type f -name '*.rs' -print0)

if (( violations > 0 )); then
    echo "ref_comments: $violations violation(s)" >&2
    exit 1
fi
exit 0
