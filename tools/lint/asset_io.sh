#!/usr/bin/env bash
# Asset I/O whitelist. See PLAN.md Sections 6 and 17.
#
# The rule is "no asset I/O outside rustic-asset/xtask/dev tooling". Settings
# files, save data, and panic dumps are not assets — `rustic-app` owns those
# per Section 4/12, so it is also allowed.
#
# Direct file I/O is restricted to:
#   - crates/rustic-asset/   (the resolver, the modding contract)
#   - crates/rustic-app/     (settings, save data, panic dumps)
#   - crates/rustic-dev/     (dev-only tooling)
#   - xtask/                 (build tooling)
#   - tests/                 (integration tests)
#
# Anywhere else, going around the AssetResolver is a bug.

set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

ALLOWED_RE='^(crates/rustic-asset/|crates/rustic-app/|crates/rustic-dev/|xtask/|tests/)'
# Runtime file I/O only. `include_bytes!`/`include_str!` are compile-time
# embeds (e.g. WGSL shaders shipped with the renderer), not asset I/O —
# they don't bypass the resolver at runtime.
PATTERN='std::fs::|tokio::fs::|std::io::BufReader::new\(File|File::open|File::create|fs::read|fs::write'

violations=0
while IFS= read -r line; do
    file="${line%%:*}"
    rel="${file#./}"
    if [[ "$rel" =~ $ALLOWED_RE ]]; then continue; fi
    echo "ASSET-IO: $line" >&2
    violations=$((violations + 1))
done < <(grep -rn -E "$PATTERN" --include='*.rs' \
    --exclude-dir=target --exclude-dir=references --exclude-dir=.git \
    . 2>/dev/null || true)

if (( violations > 0 )); then
    echo "asset_io: $violations violation(s)" >&2
    exit 1
fi
exit 0
