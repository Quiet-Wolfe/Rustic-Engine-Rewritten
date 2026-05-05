#!/usr/bin/env bash
# Backend API whitelist. See PLAN.md Sections 7 and 17.
#
# RusticV3 v1 goes through wgpu only. Direct Ash/Vulkan/Metal/D3D12/OpenGL
# calls in release crates are forbidden until v2 plans them in.
#
# Allowed locations:
#   - tools/lint/                         (this file references the names)
#   - crates/rustic-dev/experiments/      (sandboxed dev experiments)
#   - tests/                              (integration tests can probe APIs)

set -u

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

ALLOWED_RE='^(tools/lint/|crates/rustic-dev/experiments/|tests/)'
PATTERN='\bash::|\bvk::|\bmetal::|\bd3d12::|\bwindows::Win32::Graphics::Direct3D12|\bgl::|\bglow::|\bopengl_'

violations=0
while IFS= read -r line; do
    file="${line%%:*}"
    rel="${file#./}"
    if [[ "$rel" =~ $ALLOWED_RE ]]; then continue; fi
    echo "BACKEND-API: $line" >&2
    violations=$((violations + 1))
done < <(grep -rn -E "$PATTERN" --include='*.rs' \
    --exclude-dir=target --exclude-dir=references --exclude-dir=.git \
    . 2>/dev/null || true)

if (( violations > 0 )); then
    echo "backend_api: $violations violation(s)" >&2
    exit 1
fi
exit 0
